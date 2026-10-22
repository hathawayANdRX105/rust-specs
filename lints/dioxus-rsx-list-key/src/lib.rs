#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use dioxus_lint_scope::file_allowed;
use dioxus_rsx::{Attribute, BodyNode, CallBody, TemplateBody};
use quote::ToTokens;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{BytePos, SourceFile, Span};
use syn::visit::Visit;

dylint_linting::declare_late_lint! {
    /// Bans array-index list keys in rsx.
    ///
    /// A `for` loop that renders `key: "{index}"` (or `{i}` / `{idx}`) uses the
    /// array position as the diff key. On reorder/insert/delete, Dioxus reuses
    /// the wrong node and component state jumps to another row. Use a stable
    /// business id (`key: "{item.id}"`).
    ///
    /// ### Why is this bad?
    ///
    /// Index keys are only correct for static, never-reordered lists (tabs,
    /// navigation). For data lists they silently corrupt state.
    ///
    /// ### Known problems
    ///
    /// Source-text based; files that fail to parse are skipped. Only the
    /// literal `"{index}"` / `"{i}"` / `"{idx}"` shapes are caught; a computed
    /// key expression is not.
    ///
    /// ### Example
    ///
    /// ```rust
    /// rsx! {
    ///     for (index, item) in items.iter().enumerate() {
    ///         div { key: "{index}", "{item.name}" }
    ///     }
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// rsx! {
    ///     for item in items {
    ///         div { key: "{item.id}", "{item.name}" }
    ///     }
    /// }
    /// ```
    pub DIOXUS_RSX_LIST_KEY,
    Warn,
    "array-index list key in rsx; use a stable business id"
}

const MSG: &str = "列表 key 使用数组下标（{index}/{i}/{idx}）：动态列表必须用稳定业务 id";

#[derive(Debug, PartialEq, Eq)]
struct Finding {
    line: usize,
    column: usize,
    len: usize,
    msg: String,
}

fn scan_source(text: &str) -> Vec<Finding> {
    let Ok(file) = syn::parse_file(text) else {
        return Vec::new();
    };
    let mut visitor = RsxVisitor { calls: Vec::new() };
    visitor.visit_file(&file);
    let mut out = Vec::new();
    for call in &visitor.calls {
        walk_body(&call.body, &mut out);
    }
    out
}

struct RsxVisitor {
    calls: Vec<CallBody>,
}

impl<'ast> Visit<'ast> for RsxVisitor {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let is_rsx = mac
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "rsx");
        if is_rsx && let Ok(body) = syn::parse2::<CallBody>(mac.tokens.clone()) {
            self.calls.push(body);
        }
        syn::visit::visit_macro(self, mac);
    }
}

fn walk_body(body: &TemplateBody, out: &mut Vec<Finding>) {
    for node in &body.roots {
        walk_node(node, out);
    }
}

fn walk_node(node: &BodyNode, out: &mut Vec<Finding>) {
    match node {
        BodyNode::ForLoop(looped) => {
            for child in &looped.body.roots {
                check_key(child, out);
            }
            walk_body(&looped.body, out);
        }
        BodyNode::Element(element) => {
            for child in &element.children {
                walk_node(child, out);
            }
        }
        BodyNode::Component(component) => walk_body(&component.children, out),
        BodyNode::IfChain(chain) => {
            chain.for_each_branch(&mut |branch| walk_body(branch, out));
        }
        BodyNode::Text(_) | BodyNode::RawExpr(_) => {}
    }
}

fn check_key(node: &BodyNode, out: &mut Vec<Finding>) {
    match node {
        BodyNode::Element(element) => check_attrs(&element.raw_attributes, out),
        BodyNode::Component(component) => check_attrs(&component.fields, out),
        _ => {}
    }
}

fn check_attrs(attributes: &[Attribute], out: &mut Vec<Finding>) {
    for attribute in attributes {
        if !attribute.name.is_likely_key() {
            continue;
        }
        let value = attribute.value.to_token_stream().to_string();
        if value.contains("{index}") || value.contains("{i}") || value.contains("{idx}") {
            out.push(finding(attribute.name.span(), 3, MSG));
        }
    }
}

fn finding(span: proc_macro2::Span, len: usize, msg: &str) -> Finding {
    let start = span.start();
    Finding {
        line: start.line,
        column: start.column,
        len,
        msg: msg.to_string(),
    }
}

fn is_ui_crate(name: &str) -> bool {
    name.starts_with("ui_") || name == "web"
}

fn is_fixture(text: &str) -> bool {
    text.lines()
        .take(5)
        .any(|line| line.contains("dioxus-lint-fixture"))
}

fn byte_offset(text: &str, line: usize, column: usize) -> usize {
    let mut offset = 0;
    for (index, current) in text.split_inclusive('\n').enumerate() {
        if index + 1 == line {
            let line_len = current.trim_end_matches('\n').len();
            return offset + column.min(line_len);
        }
        offset += current.len();
    }
    offset
}

fn span_for(file: &SourceFile, text: &str, finding: &Finding) -> Span {
    let offset = byte_offset(text, finding.line, finding.column);
    let lo = file.start_pos + BytePos(offset as u32);
    let hi = lo + BytePos(finding.len.max(1) as u32);
    Span::with_root_ctxt(lo, hi)
}

impl<'tcx> LateLintPass<'tcx> for DioxusRsxListKey {
    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        let crate_name = cx.tcx.crate_name(LOCAL_CRATE);
        let scoped = is_ui_crate(crate_name.as_str());
        let source_map = cx.sess().source_map();
        for file in source_map.files().iter() {
            let Some(text) = file.src.as_deref().map(|text| text.as_str()) else {
                continue;
            };
            if !scoped && !is_fixture(text) {
                continue;
            }
            if !text.contains("rsx") {
                continue;
            }
            if !file_allowed(file) {
                continue;
            }
            for finding in scan_source(text) {
                let span = span_for(file, text, &finding);
                span_lint(cx, DIOXUS_RSX_LIST_KEY, span, finding.msg);
            }
        }
    }
}

impl Default for DioxusRsxListKey {
    fn default() -> Self {
        DioxusRsxListKey
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_index_key() {
        let source = r#"
fn view() {
    rsx! {
        div {
            for (index, item) in items.iter().enumerate() {
                div { key: "{index}", "{item.name}" }
            }
        }
    }
}
"#;
        assert!(!scan_source(source).is_empty());
    }

    #[test]
    fn allows_business_id_key() {
        let source = r#"
fn view() {
    rsx! {
        div {
            for item in items {
                div { key: "{item.id}", "{item.name}" }
            }
        }
    }
}
"#;
        assert!(scan_source(source).is_empty());
    }

    #[test]
    fn flags_component_key() {
        let source = r#"
fn view() {
    rsx! {
        div {
            for (i, item) in items.iter().enumerate() {
                Row { key: "{i}", item: item.clone() }
            }
        }
    }
}
"#;
        assert!(!scan_source(source).is_empty());
    }
}
