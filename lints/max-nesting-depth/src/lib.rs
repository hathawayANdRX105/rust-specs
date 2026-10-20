#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;

use clippy_utils::diagnostics::span_lint;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};

dylint_linting::declare_late_lint! {
    /// Checks for deeply nested control flow inside function bodies.
    ///
    /// When `if`/`match`/`loop` nest beyond `max_depth` (default 4), the body
    /// is hard to read and should be flattened by extracting helper functions
    /// or rewriting with combinators (map/filter/and_then).
    ///
    /// ### Why is this bad?
    ///
    /// Nesting beyond 4 levels forces the reader to track many open blocks at
    /// once. Extracting helpers or chaining Option/Result combinators reduces
    /// the depth and makes each level's intent explicit.
    ///
    /// ### Known problems
    ///
    /// Depth is measured syntactically on if/match/loop. `for`/`while`
    /// desugar to loop+match so they count as two levels.
    pub MAX_NESTING_DEPTH,
    Warn,
    "control flow nesting exceeds 4 levels; extract a helper or use combinators"
}

const MAX_DEPTH: usize = 4;

fn visit(cx: &LateContext<'_>, expr: &Expr<'_>, depth: usize) {
    let is_nesting = matches!(
        expr.kind,
        ExprKind::If(..) | ExprKind::Match(..) | ExprKind::Loop(..)
    );
    let depth = if is_nesting { depth + 1 } else { depth };
    if is_nesting && depth > MAX_DEPTH {
        span_lint(
            cx,
            MAX_NESTING_DEPTH,
            expr.span,
            "control flow nesting exceeds 4 levels; extract a helper or use combinators",
        );
    }
    // Recurse into child expressions and blocks.
    for child in expr_children(expr) {
        visit(cx, child, depth);
    }
}

fn expr_children<'tcx>(expr: &'tcx Expr<'_>) -> Vec<&'tcx Expr<'tcx>> {
    let mut out = Vec::new();
    match expr.kind {
        ExprKind::If(cond, then, els) => {
            out.push(cond);
            out.push(then);
            if let Some(e) = els {
                out.push(e);
            }
        }
        ExprKind::Match(scrut, arms, _) => {
            out.push(scrut);
            for arm in arms {
                if let Some(guard) = arm.guard {
                    out.push(guard);
                }
                out.push(arm.body);
            }
        }
        ExprKind::Loop(block, _, _, _) => {
            for stmt in block.stmts {
                if let Some(e) = stmt_expr(stmt) {
                    out.push(e);
                }
            }
            if let Some(e) = block.expr {
                out.push(e);
            }
        }
        ExprKind::Block(block, _) => {
            for stmt in block.stmts {
                if let Some(e) = stmt_expr(stmt) {
                    out.push(e);
                }
            }
            if let Some(e) = block.expr {
                out.push(e);
            }
        }
        _ => {}
    }
    out
}

fn stmt_expr<'tcx>(stmt: &'tcx rustc_hir::Stmt<'_>) -> Option<&'tcx Expr<'tcx>> {
    match stmt.kind {
        rustc_hir::StmtKind::Expr(e) => Some(e),
        rustc_hir::StmtKind::Let(local) => local.init,
        rustc_hir::StmtKind::Item(_) => None,
        rustc_hir::StmtKind::Semi(e) => Some(e),
    }
}

impl<'tcx> LateLintPass<'tcx> for MaxNestingDepth {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        visit(cx, expr, 0);
    }
}

impl Default for MaxNestingDepth {
    fn default() -> Self {
        MaxNestingDepth
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
