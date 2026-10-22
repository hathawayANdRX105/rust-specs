#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use dioxus_rsx::{BodyNode, CallBody, TemplateBody};
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{BytePos, SourceFile, Span};
use syn::visit::Visit;

dylint_linting::declare_late_lint! {
    /// Checks rsx element nesting depth.
    ///
    /// The root element is depth 1; one level of nesting (depth 2) is allowed,
    /// and depth 3 or deeper is reported. `for` and `if` bodies continue at the
    /// same depth; a component's children are a separate rsx and do not add to
    /// the depth.
    ///
    /// ### Why is this bad?
    ///
    /// Deeply nested rsx means one component is rendering a whole view.
    /// Extract the subtree into a child component (one numbered section per
    /// component is the minimum boundary).
    ///
    /// ### Known problems
    ///
    /// Source-text based; files that fail to parse are skipped. The threshold
    /// is fixed at "more than one level of nesting" (canon's ferrite setting).
    ///
    /// ### Example
    ///
    /// ```rust
    /// rsx! {
    ///     div {
    ///         div {
    ///             span { "deep" }
    ///         }
    ///     }
    /// }
    /// ```
    pub DIOXUS_RSX_NESTING,
    Warn,
    "rsx element nesting deeper than one level; extract a child component"
}

const MAX_DEPTH: usize = 2;
const MSG: &str = "rsx 元素嵌套超过 1 层（根元素算第 1 层）：抽成子组件";

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
        walk_body(&call.body, 0, &mut out);
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

fn walk_body(body: &TemplateBody, depth: usize, out: &mut Vec<Finding>) {
    for node in &body.roots {
        walk_node(node, depth, out);
    }
}

fn walk_node(node: &BodyNode, depth: usize, out: &mut Vec<Finding>) {
    match node {
        BodyNode::Element(element) => {
            let next = depth + 1;
            if next > MAX_DEPTH {
                out.push(finding(element.name.span(), 1, MSG));
            }
            for child in &element.children {
                walk_node(child, next, out);
            }
        }
        // A component renders its own rsx; its children do not add depth here.
        BodyNode::Component(_) => {}
        BodyNode::ForLoop(looped) => walk_body(&looped.body, depth, out),
        BodyNode::IfChain(chain) => {
            chain.for_each_branch(&mut |branch| walk_body(branch, depth, out));
        }
        BodyNode::Text(_) | BodyNode::RawExpr(_) => {}
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

impl<'tcx> LateLintPass<'tcx> for DioxusRsxNesting {
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
            for finding in scan_source(text) {
                let span = span_for(file, text, &finding);
                span_lint(cx, DIOXUS_RSX_NESTING, span, finding.msg);
            }
        }
    }
}

impl Default for DioxusRsxNesting {
    fn default() -> Self {
        DioxusRsxNesting
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
    fn allows_one_level_of_nesting() {
        let source = r#"
fn view() {
    rsx! {
        div {
            span { "one level" }
        }
    }
}
"#;
        assert!(scan_source(source).is_empty());
    }

    #[test]
    fn flags_two_levels_of_nesting() {
        let source = r#"
fn view() {
    rsx! {
        div {
            div {
                span { "two levels" }
            }
        }
    }
}
"#;
        assert!(!scan_source(source).is_empty());
    }

    #[test]
    fn component_children_are_separate() {
        let source = r#"
fn view() {
    rsx! {
        div {
            Card {
                div {
                    span { "inside card" }
                }
            }
        }
    }
}
"#;
        assert!(scan_source(source).is_empty());
    }

    #[test]
    fn for_body_continues_at_same_depth() {
        let source = r#"
fn view() {
    rsx! {
        div {
            for item in items {
                span { "{item}" }
            }
        }
    }
}
"#;
        assert!(scan_source(source).is_empty());
    }
}
