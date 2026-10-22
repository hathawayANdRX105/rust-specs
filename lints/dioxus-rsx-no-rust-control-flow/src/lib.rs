#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use dioxus_rsx::{BodyNode, CallBody, ExprNode, TemplateBody};
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{BytePos, SourceFile, Span};
use syn::parse::Parser;
use syn::visit::Visit;

dylint_linting::declare_late_lint! {
    /// Bans Rust control flow inside `rsx!` bodies.
    ///
    /// `if` / `if-else` is not acceptable in rsx (CI treats it as FAIL by
    /// running the lint with `-D dioxus_rsx_no_rust_control_flow`).
    /// `match`, `while` and `loop` are WARN for now; `for` is allowed.
    /// A `{ let ...; ... }` raw expression block is WARN. Closures and
    /// attribute expressions are not inspected, so Rust logic inside
    /// `.map(|x| ...)` or `class: ...` does not false-positive.
    ///
    /// ### Why is this bad?
    ///
    /// rsx describes a view. Rust control flow embedded in it hides rendering
    /// branches from the component boundary, makes the view hard to scan, and
    /// works against the "one rsx per component" discipline. Move the branch
    /// above the `rsx!` call, or extract a child component/slot.
    ///
    /// ### Known problems
    ///
    /// The lint scans source text with `syn` + `dioxus-rsx`; files that fail to
    /// parse are skipped. Files are scanned only in `ui_*` / `web` crates, or
    /// in fixtures marked with a `dioxus-lint-fixture` comment.
    ///
    /// ### Example
    ///
    /// ```rust
    /// rsx! {
    ///     div {
    ///         if show { span { "visible" } }
    ///     }
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// let body = if show { rsx! { span { "visible" } } } else { rsx! {} };
    /// rsx! { div { {body} } }
    /// ```
    pub DIOXUS_RSX_NO_RUST_CONTROL_FLOW,
    Warn,
    "Rust control flow inside rsx! (if/if-else is not acceptable)"
}

const IF_MSG: &str =
    "rsx 中禁止 if/if-else 控制流（FAIL 级）：把条件渲染移到 rsx 外，或抽成组件/插槽";
const MATCH_MSG: &str =
    "rsx 中避免 match（WARN）：把分支逻辑移到 rsx 外，或抽成组件/插槽";
const WHILE_MSG: &str =
    "rsx 中避免 while（WARN）：把循环逻辑移到 rsx 外，或抽成组件/插槽";
const LOOP_MSG: &str =
    "rsx 中避免 loop（WARN）：把循环逻辑移到 rsx 外，或抽成组件/插槽";
const LET_MSG: &str =
    "rsx 中禁止 let 语句（WARN）：把计算移到 rsx 外，或抽成组件/插槽";

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
        BodyNode::Element(element) => {
            for child in &element.children {
                walk_node(child, out);
            }
        }
        BodyNode::Component(component) => walk_body(&component.children, out),
        // `for` is allowed; still inspect its body for nested control flow.
        BodyNode::ForLoop(looped) => walk_body(&looped.body, out),
        BodyNode::IfChain(chain) => {
            out.push(finding(chain.if_token.span, 2, IF_MSG));
            chain.for_each_branch(&mut |branch| walk_body(branch, out));
        }
        BodyNode::RawExpr(expr) => check_raw_expr(expr, out),
        BodyNode::Text(_) => {}
    }
}

/// Inspect a `{ ... }` raw expression. Only the top level is judged: nested
/// closures (`.map(|x| if ...)`) and attribute expressions are left alone.
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
        syn::Expr::If(expr) => out.push(finding(expr.if_token.span, 2, IF_MSG)),
        syn::Expr::Match(expr) => out.push(finding(expr.match_token.span, 5, MATCH_MSG)),
        syn::Expr::While(expr) => out.push(finding(expr.while_token.span, 5, WHILE_MSG)),
        syn::Expr::Loop(expr) => out.push(finding(expr.loop_token.span, 4, LOOP_MSG)),
        _ => {}
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

impl<'tcx> LateLintPass<'tcx> for DioxusRsxNoRustControlFlow {
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
                span_lint(
                    cx,
                    DIOXUS_RSX_NO_RUST_CONTROL_FLOW,
                    span,
                    finding.msg,
                );
            }
        }
    }
}

impl Default for DioxusRsxNoRustControlFlow {
    fn default() -> Self {
        DioxusRsxNoRustControlFlow
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
    fn flags_if_in_rsx() {
        let source = r#"
fn view() {
    rsx! {
        div {
            if show { span { "visible" } }
        }
    }
}
"#;
        let found = findings(source);
        assert!(
            found.iter().any(|f| f.msg.contains("if/if-else")),
            "{found:?}"
        );
    }

    #[test]
    fn flags_if_else_chain_once() {
        let source = r#"
fn view() {
    rsx! {
        div {
            if show { span { "a" } } else { span { "b" } }
        }
    }
}
"#;
        let found = findings(source);
        assert_eq!(
            found.iter().filter(|f| f.msg.contains("if/if-else")).count(),
            1,
            "{found:?}"
        );
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
    fn closure_if_is_not_flagged() {
        let source = r#"
fn view() {
    rsx! {
        div {
            { items.iter().map(|x| if *x > 0 { 1 } else { 0 }).sum::<i32>() }
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
            onclick: move |_| if enabled { save() } else { cancel() },
            "click"
        }
    }
}
"#;
        assert!(findings(source).is_empty(), "{:?}", findings(source));
    }
}
