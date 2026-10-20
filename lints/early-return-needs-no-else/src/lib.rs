#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;

use clippy_utils::diagnostics::span_lint;
use rustc_hir::{Block, Expr, ExprKind, StmtKind};
use rustc_lint::{LateContext, LateLintPass};

dylint_linting::declare_late_lint! {
    /// Checks for `if cond { return ...; } else { ... }` where the `if`
    /// branch ends with an early exit (return/break/continue). The `else`
    /// branch is dead in that path, so the `else` is unnecessary: move the
    /// `else` body after the `if` and drop `else`.
    ///
    /// ### Why is this bad?
    ///
    /// An `else` after an early exit implies both branches are alternatives,
    /// but the `if` path never reaches the `else`. Removing the `else` makes
    /// the control flow linear and easier to read.
    ///
    /// ### Known problems
    ///
    /// Only the last statement of the `if` block is examined. If the early
    /// exit is nested deeper inside the block, this lint does not fire.
    ///
    /// ### Example
    ///
    /// ```rust
    /// if limit > MAX {
    ///     return Err(InvalidLimit);
    /// } else {
    ///     Ok(limit)
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// if limit > MAX {
    ///     return Err(InvalidLimit);
    /// }
    /// Ok(limit)
    /// ```
    pub EARLY_RETURN_NEEDS_NO_ELSE,
    Warn,
    "`if` branch ends with return/break/continue; drop the `else`"
}

fn block_ends_with_early_exit(block: &Block<'_>) -> bool {
    // The early exit can be a tail expression or the last statement.
    if let Some(tail) = block.expr
        && matches!(
            tail.kind,
            ExprKind::Ret(_) | ExprKind::Break(..) | ExprKind::Continue(..)
        )
    {
        return true;
    }
    if let Some(last) = block.stmts.last() {
        return match last.kind {
            StmtKind::Expr(e) | StmtKind::Semi(e) => matches!(
                e.kind,
                ExprKind::Ret(_) | ExprKind::Break(..) | ExprKind::Continue(..)
            ),
            StmtKind::Let(_) | StmtKind::Item(_) => false,
        };
    }
    false
}

impl<'tcx> LateLintPass<'tcx> for EarlyReturnNeedsNoElse {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::If(_, then, Some(else_expr)) = expr.kind else {
            return;
        };
        let ExprKind::Block(then_block, _) = then.kind else {
            return;
        };

        if block_ends_with_early_exit(then_block) {
            span_lint(
                cx,
                EARLY_RETURN_NEEDS_NO_ELSE,
                expr.span,
                "`if` branch ends with return/break/continue; drop the `else`",
            );
        }
        let _ = else_expr;
    }
}

impl Default for EarlyReturnNeedsNoElse {
    fn default() -> Self {
        EarlyReturnNeedsNoElse
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
