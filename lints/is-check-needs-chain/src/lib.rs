#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use clippy_utils::get_parent_expr;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_span::symbol::sym;

dylint_linting::declare_late_lint! {
    /// When a lone `is_some` / `is_none` / `is_ok` / `is_err` check on an
    /// `Option`/`Result` is followed by an expression that is itself an
    /// `Option`/`Result`, the check must be folded into a combinator chain on
    /// the receiver (`map` / `map_or` / `then_some` / `filter` / `and_then`),
    /// not routed through `bool` or a hand-written `if`/`else`.
    ///
    /// Two shapes are reported:
    ///
    /// ```rust
    /// # let o = Some(1);
    /// // Shape A: re-entering Option through bool
    /// let a: Option<u32> = o.is_some().then(|| 1);
    /// let b: Option<u32> = o.is_some().then_some(1);
    /// ```
    ///
    /// ```rust
    /// # let o = Some(1);
    /// // Shape B: hand-written if/else over the predicate
    /// let c: Option<u32> = if o.is_some() { Some(1) } else { None };
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// # let o = Some(1);
    /// let a: Option<u32> = o.map(|_| 1);
    /// let b: Option<u32> = o.map(|_| 1);
    /// let c: Option<u32> = o.map(|_| 1);
    /// ```
    ///
    /// `Result` receivers are handled the same way (`is_ok` / `is_err`).
    ///
    /// ### Why is this bad?
    ///
    /// The receiver is already `Option`/`Result`; asking `is_*` first and
    /// rebuilding throws away the value the chain could have carried, and
    /// reads as two decisions where one combinator says it all.
    ///
    /// ### Known problems
    ///
    /// - The `is_*` call must be a lone predicate: `o.is_some() && flag` is
    ///   not reported.
    /// - `then_some(..)` / `then(..)` followed directly by
    ///   `.unwrap_or(..)` / `.unwrap_or_else(..)` / `.unwrap_or_default()` is
    ///   deliberately **not** reported: that chain terminates in a plain
    ///   value, which is the sanctioned boolean form.
    /// - Shape B reports any `if <lone is_*> { .. } else { .. }` whose whole
    ///   expression is `Option`/`Result`, even when a branch carries logic
    ///   beyond `Some`/`None`; the combinator rewrite is still the point.
    /// - Suggestions are directional; the exact combinator (`map` vs `map_or`
    ///   vs `filter` vs `and_then`) depends on what the branches do and is
    ///   left to the author.
    ///
    /// ### Example
    ///
    /// ```rust
    /// fn count(o: Option<u32>) -> Option<u32> {
    ///     if o.is_some() { Some(1) } else { None }
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// fn count(o: Option<u32>) -> Option<u32> {
    ///     o.map(|_| 1)
    /// }
    /// ```
    pub IS_CHECK_NEEDS_CHAIN,
    Warn,
    "lone `is_*` check on Option/Result re-wraps it — chain combinators (`map`/`map_or`/`then_some`/`filter`) on the receiver instead of going through bool or `if`/`else`"
}

/// Predicate method names this lint treats as single checks.
const CHECKS: [&str; 4] = ["is_some", "is_none", "is_ok", "is_err"];

/// Chain-continuation methods that re-enter `Option` through a `bool`.
const THEN_METHODS: [&str; 2] = ["then", "then_some"];

fn is_option_or_result(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    // `is_some`/`is_ok` take `&self`, so the adjusted receiver type is a
    // reference; the ADT sits under the borrows.
    let Some(adt) = ty.peel_refs().ty_adt_def() else {
        return false;
    };
    let did = adt.did();
    tcx.is_diagnostic_item(sym::Option, did) || tcx.is_diagnostic_item(sym::Result, did)
}

/// Matches a lone, zero-argument `is_some`/`is_none`/`is_ok`/`is_err` call
/// whose receiver is an `Option`/`Result`.
fn is_check_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let ExprKind::MethodCall(path, receiver, [], _) = expr.kind else {
        return false;
    };
    CHECKS.contains(&&*path.ident.name.as_str())
        && is_option_or_result(cx.tcx, cx.typeck_results().expr_ty_adjusted(receiver))
}

/// `then`/`then_some` followed directly by an `unwrap_or*` call terminate in a
/// plain value — the sanctioned boolean form, never reported.
fn terminated_by_unwrap_or(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    get_parent_expr(cx, expr).is_some_and(|parent| {
        matches!(parent.kind, ExprKind::MethodCall(path, receiver, _, _)
            if receiver.hir_id == expr.hir_id && path.ident.name.as_str().starts_with("unwrap_or"))
    })
}

impl<'tcx> LateLintPass<'tcx> for IsCheckNeedsChain {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion() {
            return;
        }
        match expr.kind {
            // Shape A: `opt.is_some().then(..)` / `.then_some(..)` — the
            // predicate is re-wrapped into an Option through bool.
            ExprKind::MethodCall(path, receiver, _, _)
                if THEN_METHODS.contains(&&*path.ident.name.as_str())
                    && is_check_call(cx, receiver)
                    && !terminated_by_unwrap_or(cx, expr) =>
            {
                span_lint(cx, IS_CHECK_NEEDS_CHAIN, expr.span, IS_CHECK_NEEDS_CHAIN.desc);
            }
            // Shape B: `if opt.is_some() { .. } else { .. }` where the whole
            // if/else yields the same Option/Result kind.
            ExprKind::If(cond, _, Some(_))
                if is_check_call(cx, cond)
                    && is_option_or_result(cx.tcx, cx.typeck_results().expr_ty(expr)) =>
            {
                span_lint(cx, IS_CHECK_NEEDS_CHAIN, expr.span, IS_CHECK_NEEDS_CHAIN.desc);
            }
            _ => {}
        }
    }
}

impl Default for IsCheckNeedsChain {
    fn default() -> Self {
        IsCheckNeedsChain
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
