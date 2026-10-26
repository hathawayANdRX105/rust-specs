#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;

use clippy_utils::diagnostics::span_lint;
use dioxus_lint_scope::file_allowed;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use dioxus_rsx_scan::{Finding, finding, span_for};
use syn::visit::Visit;

dylint_linting::declare_late_lint! {
    /// Bans hoisting an `if` / `if-else` whose branches build `rsx!` into a
    /// `let` binding.
    ///
    /// The rule applies to every crate. Moving the branch out of rsx satisfies
    /// `dioxus-rsx-no-if-else` on a technicality but keeps the component's view
    /// split into a branch block. The project already has a single-expression
    /// form for `Element` branches:
    /// `ui_util::pick::pick(cond, rsx! { .. }, rsx! { .. })`. Use
    /// `pick_with(cond, || .., || ..)` only when a branch must stay lazy.
    ///
    /// A `let` whose branches are plain values is not reported: there the
    /// `if` / `else` expression is the preferred form, and `pick` is for rsx
    /// branches only (see `pick-outside-rsx`).
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
    ///         { pick(show, rsx! { span { "visible" } }, rsx! {}) }
    ///     }
    /// }
    /// ```
    pub LET_IF_ELSE,
    Warn,
    "rsx branches hoisted into a let-if-else; use pick / pick_with"
}

const MSG: &str = "不要把 rsx 分支 hoist 成 let-if-else：用 pick(cond, rsx!{...}, rsx!{...})；只有未选分支有副作用/集合遍历时才用 pick_with";

/// One finding produced from source text (line/column are 1-based line and
/// 0-based column, matching `proc_macro2`'s span-locations).
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
            self.out.push(finding(if_expr.if_token.span, MSG.to_string()));
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

fn is_fixture(text: &str) -> bool {
    text.lines()
        .take(5)
        .any(|line| line.contains("dioxus-lint-fixture"))
}

impl<'tcx> LateLintPass<'tcx> for LetIfElse {
    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        let source_map = cx.sess().source_map();
        for file in source_map.files().iter() {
            // Only scan files that belong to the crate being linted: dependency
            // sources are loaded for diagnostics but are not ours to police.
            if file.cnum != LOCAL_CRATE {
                continue;
            }
            let Some(text) = file.src.as_deref().map(|text| text.as_str()) else {
                continue;
            };
            if !text.contains("rsx") {
                continue;
            }
            // UI fixtures are always scanned; everything else obeys the
            // generated-icon and changed-files filters.
            if !is_fixture(text) && !file_allowed(file) {
                continue;
            }
            for finding in scan_source(text) {
                let span = span_for(file, text, &finding);
                span_lint(cx, LET_IF_ELSE, span, finding.msg);
            }
        }
    }
}

impl Default for LetIfElse {
    fn default() -> Self {
        LetIfElse
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
