#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;

use clippy_utils::diagnostics::span_lint;
use dioxus_lint_scope::file_allowed;
use dioxus_rsx::{BodyNode, CallBody, TemplateBody};
use quote::ToTokens;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use dioxus_rsx_scan::{Finding, finding, span_for};
use syn::visit::Visit;

dylint_linting::declare_late_lint! {
    /// Bans Chinese text inside rsx element bodies.
    ///
    /// User-visible copy must come from the i18n table / `LBL_` constants; a
    /// literal inside rsx cannot be translated and drifts from the key set.
    /// `data-testid` values, numbers, symbols and units stay literal; comments
    /// are not text nodes, so they are not flagged.
    ///
    /// ### Why is this bad?
    ///
    /// The project keeps copy in `locales/*.yaml` and text constants. A Chinese
    /// literal in rsx bypasses translation and shows up untranslated in the
    /// English locale.
    ///
    /// ### Known problems
    ///
    /// Source-text based; files that fail to parse are skipped. Only CJK in
    /// `BodyNode::Text` is flagged; attribute values are not text nodes.
    ///
    /// ### Example
    ///
    /// ```rust
    /// rsx! { span { "保存" } }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// rsx! { span { "{t(\"save\")}" } }
    /// ```
    pub DIOXUS_RSX_CJK_TEXT,
    Warn,
    "Chinese text inside rsx element body; use i18n / text constants"
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
        BodyNode::Text(text) => {
            let rendered = text.to_token_stream().to_string();
            if has_cjk(&rendered) {
                out.push(finding(
                    text.input.span(),
                    format!("rsx 元素体内出现中文文本：{rendered}（H3：文案走 i18n/常量）"),
                ));
            }
        }
        BodyNode::Element(element) => {
            for child in &element.children {
                walk_node(child, out);
            }
        }
        BodyNode::Component(component) => walk_body(&component.children, out),
        BodyNode::ForLoop(looped) => walk_body(&looped.body, out),
        BodyNode::IfChain(chain) => {
            chain.for_each_branch(&mut |branch| walk_body(branch, out));
        }
        BodyNode::RawExpr(_) => {}
    }
}

fn has_cjk(text: &str) -> bool {
    text.chars().any(is_cjk)
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF)
}

fn is_ui_crate(name: &str) -> bool {
    name.starts_with("ui_") || name == "web"
}

fn is_fixture(text: &str) -> bool {
    text.lines()
        .take(5)
        .any(|line| line.contains("dioxus-lint-fixture"))
}

impl<'tcx> LateLintPass<'tcx> for DioxusRsxCjkText {
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
                span_lint(cx, DIOXUS_RSX_CJK_TEXT, span, finding.msg);
            }
        }
    }
}

impl Default for DioxusRsxCjkText {
    fn default() -> Self {
        DioxusRsxCjkText
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
    fn flags_chinese_text() {
        let source = r#"
fn view() {
    rsx! {
        span { "保存" }
    }
}
"#;
        assert!(!scan_source(source).is_empty());
    }

    #[test]
    fn allows_english_text() {
        let source = r#"
fn view() {
    rsx! {
        span { "save" }
    }
}
"#;
        assert!(scan_source(source).is_empty());
    }

    #[test]
    fn ignores_attribute_values() {
        let source = r#"
fn view() {
    rsx! {
        span { "data-testid": "保存按钮" }
    }
}
"#;
        assert!(scan_source(source).is_empty());
    }
}
