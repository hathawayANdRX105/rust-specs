#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;

use clippy_utils::diagnostics::span_lint;
use dioxus_lint_scope::file_allowed;
use dioxus_rsx::{BodyNode, ExprNode};
use dioxus_rsx_scan::{Finding, finding, parse_rsx_calls, span_for, walk_nodes};
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use syn::parse::Parser;

dylint_linting::declare_late_lint! {
    /// Avoids `match`, `while`, `loop` and `{ let ...; ... }` blocks inside
    /// `rsx!` bodies.
    ///
    /// `if` / `if-else` is a separate hard rule: `dioxus-rsx-no-if-else`
    /// (FAIL via `-D`). `for` is allowed by design; only its body is still
    /// inspected for nested control flow.
    ///
    /// ### Why is this bad?
    ///
    /// rsx describes a view. Branching and statement blocks embedded in it
    /// hide rendering logic from the component boundary and make the view
    /// hard to scan. Move the logic above the `rsx!` call, or extract a child
    /// component / slot.
    ///
    /// ### Known problems
    ///
    /// The lint scans source text with `syn` + `dioxus-rsx`; files that fail
    /// to parse are skipped. Only the top level of a raw expression is judged:
    /// Rust inside closures (`.map(|x| match ...)`) and attribute expressions
    /// is not inspected.
    ///
    /// ### Example
    ///
    /// ```rust
    /// rsx! {
    ///     div {
    ///         { match state { _ => "idle" } }
    ///     }
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// let label = match state { _ => "idle" };
    /// rsx! { div { {label} } }
    /// ```
    pub DIOXUS_RSX_AVOID_CONTROL_FLOW,
    Warn,
    "avoid match/while/loop/let blocks inside rsx!"
}

const MATCH_MSG: &str = "rsx 中避免 match（WARN）：把分支逻辑移到 rsx 外，或抽成组件/插槽";
const WHILE_MSG: &str = "rsx 中避免 while（WARN）：把循环逻辑移到 rsx 外，或抽成组件/插槽";
const LOOP_MSG: &str = "rsx 中避免 loop（WARN）：把循环逻辑移到 rsx 外，或抽成组件/插槽";
const LET_MSG: &str = "rsx 中避免 let 语句块（WARN）：把计算移到 rsx 外，或抽成组件/插槽";

fn scan_source(text: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    for call in parse_rsx_calls(text) {
        walk_nodes(&call.body, &mut |node| {
            if let BodyNode::RawExpr(expr) = node {
                check_raw_expr(expr, &mut out);
            }
        });
    }
    out
}

/// Inspect a `{ ... }` raw expression. Only the top level is judged: nested
/// closures (`.map(|x| match ...)`) and attribute expressions are left alone.
fn check_raw_expr(node: &ExprNode, out: &mut Vec<Finding>) {
    let partial = &node.expr;
    if partial.brace.is_some() {
        // Braced raw expressions store the *contents* of the braces, so parse
        // them as block statements rather than as a single expression.
        let Ok(statements) = syn::Block::parse_within.parse2(partial.expr.clone()) else {
            return;
        };
        for statement in &statements {
            match statement {
                syn::Stmt::Local(local) => out.push(finding(local.let_token.span, 3, LET_MSG)),
                syn::Stmt::Expr(expr, _) => check_top_expr(expr, out),
                syn::Stmt::Macro(_) | syn::Stmt::Item(_) => {}
            }
        }
    } else if let Ok(expr) = syn::parse2::<syn::Expr>(partial.expr.clone()) {
        check_top_expr(&expr, out);
    }
}

fn check_top_expr(expr: &syn::Expr, out: &mut Vec<Finding>) {
    match expr {
        syn::Expr::Match(expr) => out.push(finding(expr.match_token.span, 5, MATCH_MSG)),
        syn::Expr::While(expr) => out.push(finding(expr.while_token.span, 5, WHILE_MSG)),
        syn::Expr::Loop(expr) => out.push(finding(expr.loop_token.span, 4, LOOP_MSG)),
        _ => {}
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

impl<'tcx> LateLintPass<'tcx> for DioxusRsxAvoidControlFlow {
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
                span_lint(cx, DIOXUS_RSX_AVOID_CONTROL_FLOW, span, finding.msg);
            }
        }
    }
}

impl Default for DioxusRsxAvoidControlFlow {
    fn default() -> Self {
        DioxusRsxAvoidControlFlow
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
    fn flags_match_while_loop() {
        let source = r#"
fn view() {
    rsx! {
        div {
            { match state { _ => "idle" } }
            { while running { tick() } }
            { loop { break } }
        }
    }
}
"#;
        let found = findings(source);
        assert!(found.iter().any(|f| f.msg.contains("match")), "{found:?}");
        assert!(found.iter().any(|f| f.msg.contains("while")), "{found:?}");
        assert!(found.iter().any(|f| f.msg.contains("loop")), "{found:?}");
    }

    #[test]
    fn flags_let_statement_block() {
        let source = r#"
fn view() {
    rsx! {
        div {
            { let label = compute(); label }
        }
    }
}
"#;
        let found = findings(source);
        assert!(found.iter().any(|f| f.msg.contains("let")), "{found:?}");
    }

    #[test]
    fn if_is_not_flagged() {
        let source = r#"
fn view() {
    rsx! {
        div {
            if show { span { "visible" } }
        }
    }
}
"#;
        assert!(findings(source).is_empty(), "{:?}", findings(source));
    }

    #[test]
    fn for_is_allowed() {
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
        assert!(findings(source).is_empty(), "{:?}", findings(source));
    }

    #[test]
    fn closure_match_is_not_flagged() {
        let source = r#"
fn view() {
    rsx! {
        div {
            { items.iter().map(|x| match x { _ => 0 }).sum::<i32>() }
        }
    }
}
"#;
        assert!(findings(source).is_empty(), "{:?}", findings(source));
    }

    #[test]
    fn attribute_expression_is_not_flagged() {
        let source = r#"
fn view() {
    rsx! {
        div {
            onclick: move |_| match enabled { _ => save() },
            "click"
        }
    }
}
"#;
        assert!(findings(source).is_empty(), "{:?}", findings(source));
    }
}
