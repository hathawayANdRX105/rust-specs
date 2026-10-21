#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use std::collections::HashSet;

use rustc_hir::{Expr, ExprKind, HirId, LoopSource, MatchSource, PatKind, Stmt, StmtKind};
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

fn visit<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>, depth: usize, seen: &mut HashSet<HirId>) {
    // The same HIR node can be reached through more than one path (a
    // desugared `while let` body, for instance). Measure every node once,
    // so the depth reflects the source, not the desugaring.
    if !seen.insert(expr.hir_id) {
        return;
    }
    match expr.kind {
        // `for x in it { .. }` desugars to DropTemps(Match(iterexpr, [arm],
        // ForLoopDesugar)) whose arm body is a Loop that holds the `next()`
        // match plus the user's body. Count the loop as one level and skip the
        // desugaring.
        ExprKind::DropTemps(inner) => {
            if is_for_desugar(inner) {
                visit_for(cx, inner, depth, seen);
            } else {
                visit(cx, inner, depth, seen);
            }
        }
        ExprKind::Match(_, [_], MatchSource::ForLoopDesugar) => {
            visit_for(cx, expr, depth, seen);
        }
        ExprKind::If(cond, then, els) => {
            let depth = depth + 1;
            report(cx, expr, depth);
            // The condition sits *beside* the `if`, not inside it.
            visit(cx, cond, depth - 1, seen);
            visit(cx, then, depth, seen);
            if let Some(els) = els {
                // `} else if .. {` is a flat chain: the inner `if` shares the
                // level of the outer one.
                if matches!(els.kind, ExprKind::If(..)) {
                    visit(cx, els, depth - 1, seen);
                } else {
                    visit(cx, els, depth, seen);
                }
            }
        }
        ExprKind::Match(scrut, arms, _) => {
            let depth = depth + 1;
            report(cx, expr, depth);
            // The scrutinee sits *beside* the `match`, not inside it.
            visit(cx, scrut, depth - 1, seen);
            for arm in arms {
                if let Some(guard) = arm.guard {
                    visit(cx, guard, depth, seen);
                }
                visit(cx, arm.body, depth, seen);
            }
        }
        ExprKind::Loop(block, _, source, _) => {
            match source {
                // `for` desugars to Match(ForLoopDesugar) + Loop, and the
                // caller (`visit_for`) already counted the level.
                LoopSource::ForLoop => {
                    visit_loop_body(cx, block, depth, seen);
                }
                // A `while` *is* this Loop node — nobody counted it yet, so it
                // is a level of its own. Its first statement is the
                // desugared `if !cond { break }`, which is not user nesting.
                LoopSource::While => {
                    let depth = depth + 1;
                    report(cx, expr, depth);
                    visit_while_body(cx, block, depth, seen);
                }
                _ => {
                    let depth = depth + 1;
                    report(cx, expr, depth);
                    visit_loop_body(cx, block, depth, seen);
                }
            }
        }
        ExprKind::Block(block, _) => {
            visit_block(cx, block, depth, seen)
        }
        _ => {}
    }
}

fn report<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>, depth: usize) {
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
fn visit_for<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    depth: usize,
    seen: &mut HashSet<HirId>,
) {
    let ExprKind::Match(_, [arm], MatchSource::ForLoopDesugar) = expr.kind else {
        return;
    };
    let depth = depth + 1;
    // The arm body is the loop container; its first statement is the
    // `next()` match whose second arm holds the user's body.
    if let ExprKind::Loop(block, _, _, _) = arm.body.kind {
        for stmt in block.stmts.iter().skip(1) {
            if let Some(e) = stmt_expr(stmt) {
                visit(cx, e, depth, seen);
            }
        }
        if let Some(e) = block.expr {
            visit(cx, e, depth, seen);
        }
        if let Some(body) = for_user_body(&block.stmts) {
            visit(cx, body, depth, seen);
        } else {
        }
    }
}

/// The user's `for` body: the second arm of the `next()` match.
fn for_user_body<'a, 'tcx>(stmts: &'a [Stmt<'tcx>]) -> Option<&'a Expr<'tcx>> {
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

/// A `while` body.
///
/// `while cond { .. }` desugars to `loop { if !cond { break } .. }` and
/// `while let pat = expr { .. }` to `loop { match expr { pat => body, _ => break } }`.
/// In both cases the first statement is the desugaring, and the user's body
/// lives inside it — so walk that arm, not the raw statement (walking the
/// raw statement would count the desugared `match`/`if` as a level and
/// measure the user's body twice).
fn visit_while_body<'tcx>(
    cx: &LateContext<'tcx>,
    block: &rustc_hir::Block<'tcx>,
    depth: usize,
    seen: &mut HashSet<HirId>,
) {
    // `while let pat = expr { body }` is
    //   Loop { block { stmts: [], expr: Some(If(Let(..), body, break)) } }
    // — the user's body is the `then` branch of that `if`, not a statement.
    if let Some(tail) = block.expr
        && let ExprKind::If(cond, then, _) = tail.kind
        && matches!(cond.kind, ExprKind::Let(..))
    {
        visit(cx, then, depth, seen);
        return;
    }
    // `while cond { body }` is
    //   Loop { block { stmts: [If(!cond, break)], expr: body } }
    // — the first statement is the desugared condition check.
    for stmt in block.stmts.iter().skip(1) {
        if let Some(e) = stmt_expr(stmt) {
            visit(cx, e, depth, seen);
        }
    }
    if let Some(e) = block.expr {
        visit(cx, e, depth, seen);
    }
}

fn visit_loop_body<'tcx>(
    cx: &LateContext<'tcx>,
    block: &rustc_hir::Block<'tcx>,
    depth: usize,
    seen: &mut HashSet<HirId>,
) {
    for stmt in block.stmts {
        if let Some(e) = stmt_expr(stmt) {
            visit(cx, e, depth, seen);
        }
    }
    if let Some(e) = block.expr {
        visit(cx, e, depth, seen);
    }
}

fn visit_block<'tcx>(
    cx: &LateContext<'tcx>,
    block: &rustc_hir::Block<'tcx>,
    depth: usize,
    seen: &mut HashSet<HirId>,
) {
    visit_loop_body(cx, block, depth, seen);
}

fn stmt_expr<'a, 'tcx>(stmt: &'a Stmt<'tcx>) -> Option<&'a Expr<'tcx>> {
    match stmt.kind {
        StmtKind::Expr(e) => Some(e),
        StmtKind::Let(local) => local.init,
        StmtKind::Item(_) => None,
        StmtKind::Semi(e) => Some(e),
    }
}

impl<'tcx> LateLintPass<'tcx> for MaxNestingDepth {
    /// Walk each function body once, from the top.
    ///
    /// Walking from every expression (`check_expr`) would re-count the same
    /// subtree from every node it contains, so the same `match` would be
    /// measured both as a nested level and as a fresh root — inflating the
    /// depth. One traversal per body keeps the number honest.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _kind: rustc_hir::intravisit::FnKind<'tcx>,
        _decl: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx rustc_hir::Body<'tcx>,
        _span: rustc_span::Span,
        _id: rustc_hir::def_id::LocalDefId,
    ) {
        let mut seen = HashSet::new();
        visit(cx, body.value, 0, &mut seen);
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
