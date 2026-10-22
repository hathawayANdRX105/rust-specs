#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{Block, Expr, ExprKind, LoopSource, MatchSource, Pat, PatKind, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_span::symbol::sym;

dylint_linting::declare_late_lint! {
    /// Detect `if let` and `for` bodies that contain exactly one expression and
    /// no control-flow escape, i.e. candidates for chaining into
    /// `Option::is_some_and` / `Iterator::for_each`.
    ///
    /// This rule is **detection-only** for now: it reports candidates and is
    /// used to collect statistics, it does not emit a rewrite suggestion.
    /// A rewrite is only safe when the body contains no `?`, `return`, `break`,
    /// `continue` or `.await`, and (for `if let`) when there is no `else`.
    ///
    /// `let _ = expr;` bodies are excluded by design: the `let _` is the
    /// explicit "deliberately ignore this result" signal, and folding it into
    /// `is_some_and(|x| ...is_ok())` hides that intent inside a closure.
    ///
    /// ### Why is this reported?
    ///
    /// `if let Some(x) = expr { f(x); }` and `for x in iter { f(x); }` can often
    /// be written as one chain (`expr.is_some_and(|x| ...)` /
    /// `iter.into_iter().for_each(|x| ...)`), which keeps the flow linear and
    /// avoids a nesting level.
    ///
    /// ### Known problems
    ///
    /// The body must not contain an escape (`?`, `return`, `break`,
    /// `continue`, `.await`): inside a closure those would change meaning.
    /// Nested control flow (`if`/`match`/loop) is also skipped because it reads
    /// worse as a closure. The rule does not check whether the rewrite is
    /// type-correct (e.g. the body may return a non-`bool` value).
    pub CHAINABLE_SINGLE_STATEMENT,
    Warn,
    "single-expression body that could be chained into is_some_and/for_each"
}

impl<'tcx> LateLintPass<'tcx> for ChainableSingleStatement {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // `if let` without `else` has type `()`, so it is safe to detect in any
        // position (statement or tail expression); the chain rewrite just needs
        // a trailing `;` when it replaces a tail expression.
        if let ExprKind::If(cond, then, None) = expr.kind {
            if let ExprKind::Let(let_expr) = cond.kind {
                // Only `Some`/`Ok`/`Err` on a non-reference Option/Result: those
                // are the cases with a clean `is_some_and` / `is_ok_and` /
                // `is_err_and` rewrite. Other patterns/enums have no method.
                if is_option_or_result_pat(let_expr.pat)
                    && is_option_or_result_ty(cx.tcx, cx.typeck_results().expr_ty(let_expr.init))
                {
                    if let ExprKind::Block(then_block, None) = then.kind {
                        if let Some(body) = single_body_expr(then_block) {
                            // `is_some_and` needs a `bool`; a `Result` body can
                            // be turned into `.is_ok()`/`.is_err()`. Anything
                            // else (usually `()`) would need `{ body; true }`,
                            // which reads worse than the `if let`.
                            let body_ty = cx.typeck_results().expr_ty(body);
                            if (body_ty.is_bool() || is_result_ty(cx.tcx, body_ty))
                                && !matches!(
                                    body.kind,
                                    ExprKind::Assign(..) | ExprKind::AssignOp(..)
                                )
                                && is_chainable_body(body)
                            {
                                span_lint(
                                    cx,
                                    CHAINABLE_SINGLE_STATEMENT,
                                    expr.span,
                                    "if-let body is a single expression; could be chained",
                                );
                            }
                        }
                    }
                }
            }
            return;
        }
        let ExprKind::Loop(loop_body, _, LoopSource::ForLoop, _) = expr.kind else {
            return;
        };
        // `for` is desugared to `Loop { stmts: [Match(.., ForLoopDesugar)] }`;
        // the first arm's body block is the user-written loop body.
        let Some(stmt) = loop_body.stmts.first() else {
            return;
        };
        let StmtKind::Expr(match_expr) = stmt.kind else {
            return;
        };
        let ExprKind::Match(_, arms, MatchSource::ForLoopDesugar) = match_expr.kind else {
            return;
        };
        // The desugared match is `match next(&mut iter) { None => break,
        // Some(pat) => <user body> }`; pick the arm that is not the `break`.
        let Some(arm) = arms
            .iter()
            .find(|arm| !matches!(arm.body.kind, ExprKind::Break(..)))
        else {
            return;
        };
        let ExprKind::Block(user_body, None) = arm.body.kind else {
            return;
        };
        let Some(body) = single_body_expr(user_body) else {
            return;
        };
        // An assignment to a captured variable would move it into the closure
        // and not compile (`for_each` takes `FnMut`); skip those.
        if matches!(body.kind, ExprKind::Assign(..) | ExprKind::AssignOp(..)) {
            return;
        }
        if !is_chainable_body(body) {
            return;
        }
        span_lint(
            cx,
            CHAINABLE_SINGLE_STATEMENT,
            expr.span,
            "for-loop body is a single expression; could be chained",
        );
    }
}

/// Returns the body expression when the block contains exactly one expression
/// statement (or exactly one tail expression).
fn single_body_expr<'hir>(block: &'hir Block<'hir>) -> Option<&'hir Expr<'hir>> {
    if block.stmts.len() == 1 && block.expr.is_none() {
        match block.stmts[0].kind {
            StmtKind::Expr(expr) | StmtKind::Semi(expr) => Some(unwrap_single_block(expr)),
            _ => None,
        }
    } else if block.stmts.is_empty() {
        block.expr.map(unwrap_single_block)
    } else {
        None
    }
}

/// `println!`/`vec!`/`format!` expand to a block with a single statement; look
/// through it so the macro call counts as the body expression.
fn unwrap_single_block<'hir>(expr: &'hir Expr<'hir>) -> &'hir Expr<'hir> {
    if let ExprKind::Block(block, None) = expr.kind {
        if block.stmts.len() == 1 && block.expr.is_none() {
            if let StmtKind::Expr(inner) | StmtKind::Semi(inner) = block.stmts[0].kind {
                return unwrap_single_block(inner);
            }
        }
        if block.stmts.is_empty() {
            if let Some(inner) = block.expr {
                return unwrap_single_block(inner);
            }
        }
    }
    expr
}

/// The body can be moved into a closure only when it has no control-flow escape
/// and is not itself control flow (a nested `if`/`match`/loop reads worse as a
/// closure).
fn is_chainable_body(expr: &Expr<'_>) -> bool {
    if matches!(
        expr.kind,
        ExprKind::If(..) | ExprKind::Match(..) | ExprKind::Loop(..) | ExprKind::Block(..)
    ) {
        return false;
    }
    !has_escape(expr)
}

fn is_option_or_result_ty(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    let Some(adt) = ty.ty_adt_def() else {
        return false;
    };
    let did = adt.did();
    tcx.is_diagnostic_item(sym::Option, did) || tcx.is_diagnostic_item(sym::Result, did)
}

/// The scrutinee type is already checked to be `Option`/`Result`; a tuple
/// struct pattern on it is `Some(..)` / `Ok(..)` / `Err(..)`.
fn is_result_ty(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    let Some(adt) = ty.ty_adt_def() else {
        return false;
    };
    tcx.is_diagnostic_item(sym::Result, adt.did())
}

fn is_option_or_result_pat(pat: &Pat<'_>) -> bool {
    matches!(pat.kind, PatKind::TupleStruct(..))
}

fn has_escape(expr: &Expr<'_>) -> bool {
    let mut visitor = EscapeVisitor { found: false };
    visitor.visit_expr(expr);
    visitor.found
}

struct EscapeVisitor {
    found: bool,
}

impl<'v> Visitor<'v> for EscapeVisitor {
    fn visit_expr(&mut self, expr: &'v Expr<'v>) {
        if self.found {
            return;
        }
        match expr.kind {
            ExprKind::Ret(..) | ExprKind::Break(..) | ExprKind::Continue(..) => {
                self.found = true;
                return;
            }
            ExprKind::Match(_, _, MatchSource::TryDesugar(_) | MatchSource::AwaitDesugar) => {
                self.found = true;
                return;
            }
            _ => {}
        }
        intravisit::walk_expr(self, expr);
    }
}

impl Default for ChainableSingleStatement {
    fn default() -> Self {
        ChainableSingleStatement
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
