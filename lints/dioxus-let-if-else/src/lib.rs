#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use dioxus_lint_scope::file_allowed;
use proc_macro2::Span;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{BytePos, SourceFile};
use syn::visit::Visit;

dylint_linting::declare_late_lint! {
    /// Bans hoisting an `if` / `if-else` whose branches build `rsx!` into a
    /// `let` binding.
    ///
    /// Moving the branch out of rsx satisfies `dioxus-rsx-no-if-else` on a
    /// technicality but keeps the component's view split into a branch block.
    /// The project already has a single-expression form: use
    /// `ui_util::pick::pick_with(cond, || rsx! { .. }, || rsx! { .. })` for
    /// `Element` branches (lazy), or `ui_util::pick::pick(cond, a, b)` for
    /// cheap `Copy` values.
    ///
    /// ### Why is this bad?
    ///
    /// A `let node = if ..` block adds a second view-shaped value and pushes
    /// the reader through a branch before reaching the returned rsx. `pick` /
    /// `pick_with` keep the choice at the call site, in one expression.
    ///
    /// ### Known problems
    ///
    /// Source-text based; files that fail to parse are skipped. Only `let`
    /// initializers are inspected, so an `if` written directly inside rsx is
    /// left to `dioxus-rsx-no-if-else` (no double reporting). Ordinary
    /// `let x = if ..` values without an `rsx!` branch are not reported.
    ///
    /// ### Example
    ///
    /// ```rust
    /// let node = if show { rsx! { span { "visible" } } } else { rsx! {} };
    /// rsx! { div { {node} } }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// rsx! {
    ///     div {
    ///         { pick_with(show, || rsx! { span { "visible" } }, || rsx! {}) }
    ///     }
    /// }
    /// ```
    pub DIOXUS_LET_IF_ELSE,
    Warn,
    "rsx branches hoisted into a let-if-else; use pick_with / pick"
}

const MSG: &str = "不要把 rsx 分支 hoist 成 let-if-else：用 pick_with(cond, || rsx!{...}, || rsx!{...})（Element/惰性），cheap Copy 值用 pick(cond, a, b)";

/// One finding produced from source text (line/column are 1-based line and
/// 0-based column, matching `proc_macro2`'s span-locations).
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
    let mut visitor = LetIfVisitor { out: Vec::new() };
    visitor.visit_file(&file);
    visitor.out
}

struct LetIfVisitor {
    out: Vec<Finding>,
}

impl<'ast> Visit<'ast> for LetIfVisitor {
    fn visit_local(&mut self, local: &'ast syn::Local) {
        if let Some(init) = &local.init
            && let syn::Expr::If(if_expr) = &*init.expr
            && if_expr_has_rsx(if_expr)
        {
            self.out.push(finding(if_expr.if_token.span, 2, MSG));
        }
        syn::visit::visit_local(self, local);
    }
}

/// True when the `if` (including the `else if` chain) contains an `rsx!` macro
/// in any branch. The condition is visited too; an `rsx!` there is equally
/// suspicious, and the branch walk covers the normal hoisting pattern.
fn if_expr_has_rsx(expr: &syn::ExprIf) -> bool {
    let mut finder = RsxFinder { found: false };
    finder.visit_expr_if(expr);
    finder.found
}

struct RsxFinder {
    found: bool,
}

impl<'ast> Visit<'ast> for RsxFinder {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if mac
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "rsx")
        {
            self.found = true;
        }
        syn::visit::visit_macro(self, mac);
    }
}

fn finding(span: Span, len: usize, msg: &str) -> Finding {
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

fn span_for(file: &SourceFile, text: &str, finding: &Finding) -> rustc_span::Span {
    let offset = byte_offset(text, finding.line, finding.column);
    let lo = file.start_pos + BytePos(offset as u32);
    let hi = lo + BytePos(finding.len.max(1) as u32);
    rustc_span::Span::with_root_ctxt(lo, hi)
}

impl<'tcx> LateLintPass<'tcx> for DioxusLetIfElse {
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
                span_lint(cx, DIOXUS_LET_IF_ELSE, span, finding.msg);
            }
        }
    }
}

impl Default for DioxusLetIfElse {
    fn default() -> Self {
        DioxusLetIfElse
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings(source: &str) -> Vec<Finding> {
        scan_source(source)
    }

    #[test]
    fn flags_hoisted_let_if() {
        let source = r#"
fn view(c: bool) {
    let node = if c { rsx! { span { "a" } } } else { rsx! { span { "b" } } };
    rsx! { div { {node} } }
}
"#;
        assert_eq!(findings(source).len(), 1);
    }

    #[test]
    fn flags_hoisted_let_if_let() {
        let source = r#"
fn view(opt: Option<u8>) {
    let node = if let Some(x) = opt { rsx! { span { "{x}" } } } else { rsx! {} };
    rsx! { div { {node} } }
}
"#;
        assert_eq!(findings(source).len(), 1);
    }

    #[test]
    fn ignores_plain_let_if() {
        let source = r#"
fn view(c: bool) {
    let n = if c { 1 } else { 2 };
    let _ = n;
}
"#;
        assert!(findings(source).is_empty());
    }

    #[test]
    fn ignores_pick_with() {
        let source = r#"
fn view(c: bool) {
    let _ = rsx! {
        div {
            { pick_with(c, || rsx! { span { "a" } }, || rsx! { span { "b" } }) }
        }
    };
}
"#;
        assert!(findings(source).is_empty());
    }

    #[test]
    fn ignores_if_inside_rsx() {
        let source = r#"
fn view(c: bool) {
    let _ = rsx! {
        div {
            if c { span { "a" } }
        }
    };
}
"#;
        assert!(findings(source).is_empty());
    }
}
