#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;

use clippy_utils::diagnostics::span_lint;
use rustc_hir::{Expr, ExprKind, LoopSource, MatchSource, PatKind, Stmt, StmtKind};
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
    /// Depth counts *visual* nesting: a `for`/`while` counts as one level (its
    /// HIR desugaring is not counted), and an `else if` chain counts as one
    /// level (the chain is flat, not nested).
    pub MAX_NESTING_DEPTH,
    Warn,
    "control flow nesting exceeds 4 levels; extract a helper or use combinators"
}

const MAX_DEPTH: usize = 4;

const MSG: &str = "control flow nesting exceeds 4 levels; extract a helper or use combinators";

fn visit<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, depth: usize) {
    match expr.kind {
        // `for x in it { .. }` desugars to DropTemps(Match(iterexpr, [arm],
        // ForLoopDesugar)) whose arm body is a Loop that holds the `next()`
        // match plus the user's body. Count the loop as one level and skip the
        // desugaring.
        ExprKind::DropTemps(inner) => {
            if is_for_desugar(inner) {
                visit_for(cx, inner, depth);
            } else {
                visit(cx, inner, depth);
            }
        }
        ExprKind::Match(_, [_], MatchSource::ForLoopDesugar) => {
            visit_for(cx, expr, depth);
        }
        ExprKind::If(cond, then, els) => {
            let depth = depth + 1;
            report(cx, expr, depth);
            visit(cx, cond, depth);
            visit(cx, then, depth);
            if let Some(els) = els {
                // `} else if .. {` is a flat chain: the inner `if` shares the
                // level of the outer one.
                if matches!(els.kind, ExprKind::If(..)) {
                    visit(cx, els, depth - 1);
                } else {
                    visit(cx, els, depth);
                }
            }
        }
        ExprKind::Match(scrut, arms, _) => {
            let depth = depth + 1;
            report(cx, expr, depth);
            visit(cx, scrut, depth);
            for arm in arms {
                if let Some(guard) = arm.guard {
                    visit(cx, guard, depth);
                }
                visit(cx, arm.body, depth);
            }
        }
        ExprKind::Loop(block, _, source, _) => {
            match source {
                // The `for`/`while` level was already counted by the caller.
                LoopSource::ForLoop | LoopSource::While => {
                    visit_loop_body(cx, block, depth);
                }
                _ => {
                    let depth = depth + 1;
                    report(cx, expr, depth);
                    visit_loop_body(cx, block, depth);
                }
            }
        }
        ExprKind::Block(block, _) => {
            visit_block(cx, block, depth)
        }
        _ => {}
    }
}

fn report<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, depth: usize) {
    if depth > MAX_DEPTH {
        span_lint(cx, MAX_NESTING_DEPTH, expr.span, MSG);
    }
}

/// A `for` loop in HIR: `Match(iterexpr, [arm], ForLoopDesugar)`.
fn is_for_desugar(expr: &Expr<'_>) -> bool {
    matches!(
        expr.kind,
        ExprKind::Match(_, [_arm], MatchSource::ForLoopDesugar)
    )
}

/// Walk a `for` loop: one level, then the user's body only.
fn visit_for<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, depth: usize) {
    let ExprKind::Match(_, [arm], MatchSource::ForLoopDesugar) = expr.kind else {
        return;
    };
    let depth = depth + 1;
    // The arm body is the loop container; its first statement is the
    // `next()` match whose second arm holds the user's body.
    if let ExprKind::Loop(block, _, _, _) = arm.body.kind {
        for stmt in block.stmts.iter().skip(1) {
            if let Some(e) = stmt_expr(stmt) {
                visit(cx, e, depth);
            }
        }
        if let Some(e) = block.expr {
            visit(cx, e, depth);
        }
        if let Some(body) = for_user_body(&block.stmts) {
            visit(cx, body, depth);
        } else {
        }
    }
}

/// The user's `for` body: the second arm of the `next()` match.
fn for_user_body<'tcx>(stmts: &'tcx [Stmt<'tcx>]) -> Option<&'tcx Expr<'tcx>> {
    let first = stmt_expr(stmts.first()?)?;
    let ExprKind::Match(_, arms, _) = first.kind else {
        return None;
    };
    // The `Some` arm carries exactly one field (the loop binding); the
    // `None` arm is a unit variant with no field. Matching "any Struct"
    // would pick the `None` arm (whose body is `break`).
    let arm = arms
        .iter()
        .find(|arm| matches!(arm.pat.kind, PatKind::Struct(_, [_field], _)))?;
    Some(arm.body)
}

fn visit_loop_body<'tcx>(cx: &LateContext<'tcx>, block: &'tcx rustc_hir::Block<'tcx>, depth: usize) {
    for stmt in block.stmts {
        if let Some(e) = stmt_expr(stmt) {
            visit(cx, e, depth);
        }
    }
    if let Some(e) = block.expr {
        visit(cx, e, depth);
    }
}

fn visit_block<'tcx>(cx: &LateContext<'tcx>, block: &'tcx rustc_hir::Block<'tcx>, depth: usize) {
    visit_loop_body(cx, block, depth);
}

fn stmt_expr<'tcx>(stmt: &'tcx Stmt<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    match stmt.kind {
        StmtKind::Expr(e) => Some(e),
        StmtKind::Let(local) => local.init,
        StmtKind::Item(_) => None,
        StmtKind::Semi(e) => Some(e),
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
