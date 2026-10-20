#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use clippy_utils::higher::ForLoop;
use rustc_hir::{Block, Expr, ExprKind, Stmt, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::Span;

dylint_linting::declare_late_lint! {
    /// Checks for three or more consecutive `for` loops that only push
    /// items into the same `Vec`. Such loops can be merged into a single
    /// iterator chain (`flat_map`/`chain` + `collect`).
    ///
    /// ### Why is this bad?
    ///
    /// Several loops with identical shape obscure the fact that they build
    /// one collection. A single chain expresses the intent at once and lets
    /// the compiler vectorize better.
    ///
    /// ### Known problems
    ///
    /// Only detects `for x in iter { out.push(expr); }` shape syntactically.
    /// Does not check whether the pushed expressions share a type; review
    /// each hit before merging.
    ///
    /// ### Example
    ///
    /// ```rust
    /// for m in messages {
    ///     out.push(encode(&m));
    /// }
    /// for e in events {
    ///     out.push(encode(&e));
    /// }
    /// ```
    pub CONSECUTIVE_SAME_SHAPE_LOOPS,
    Warn,
    "consecutive same-shape push loops; use an iterator chain + collect"
}

const MIN_CONSECUTIVE: usize = 3;

/// If `stmt` is a `for` loop whose body only pushes into a Vec, return info.
fn classify_push_loop<'a>(stmt: &'a Stmt<'_>) -> Option<(Span, &'a Expr<'a>)> {
    let (StmtKind::Expr(expr) | StmtKind::Semi(expr)) = stmt.kind else {
        return None;
    };
    let ForLoop { body, span, .. } = ForLoop::hir(expr)?;
    if is_single_push_block(body) {
        Some((span, body))
    } else {
        None
    }
}

fn is_single_push_block(body: &Expr<'_>) -> bool {
    let ExprKind::Block(block, _) = body.kind else {
        return false;
    };
    // Body must be exactly `out.push(...);` (single statement, no tail).
    if block.stmts.len() != 1 || block.expr.is_some() {
        return false;
    }
    let stmt = &block.stmts[0];
    let (StmtKind::Expr(e) | StmtKind::Semi(e)) = stmt.kind else {
        return false;
    };
    is_push_call(e)
}

fn is_push_call(expr: &Expr<'_>) -> bool {
    matches!(
        expr.kind,
        ExprKind::MethodCall(ref path, _, _, _) if path.ident.name.as_str() == "push"
    )
}

fn scan_stmts(cx: &LateContext<'_>, block: &Block<'_>) {
    let mut run = 0usize;
    let mut first_span = None;
    for stmt in block.stmts {
        match classify_push_loop(stmt) {
            Some((span, _)) => {
                if run == 0 {
                    first_span = Some(span);
                }
                run += 1;
            }
            None => {
                if run >= MIN_CONSECUTIVE {
                    if let Some(span) = first_span {
                        span_lint(
                            cx,
                            CONSECUTIVE_SAME_SHAPE_LOOPS,
                            span,
                            "consecutive same-shape push loops; use an iterator chain + collect",
                        );
                    }
                }
                run = 0;
                first_span = None;
            }
        }
    }
    if run >= MIN_CONSECUTIVE {
        if let Some(span) = first_span {
            span_lint(
                cx,
                CONSECUTIVE_SAME_SHAPE_LOOPS,
                span,
                "consecutive same-shape push loops; use an iterator chain + collect",
            );
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for ConsecutiveSameShapeLoops {
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        scan_stmts(cx, block);
    }
}

impl Default for ConsecutiveSameShapeLoops {
    fn default() -> Self {
        ConsecutiveSameShapeLoops
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
