#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_span::symbol::sym;

dylint_linting::declare_late_lint! {
    /// Checks for any `.unwrap()` call on a `Result`/`Option` receiver.
    /// `unwrap` panics without explaining the failure mode; use
    /// `.expect("reason")` to document the invariant, or `?` to propagate
    /// the error in fallible functions.
    ///
    /// ### Why is this bad?
    ///
    /// `unwrap` hides why a panic can happen. `expect("reason")` forces the
    /// author to state the invariant; `?` lets the caller handle the error.
    ///
    /// ### Known problems
    ///
    /// This lint does not distinguish tests/examples; add `#[allow(ban_unwrap)]`
    /// where a bare unwrap is acceptable, or configure paths to exclude them.
    ///
    /// ### Example
    ///
    /// ```rust
    /// let x = v.unwrap();
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// let x = v.expect("value must be present");
    /// // or, in a fallible function:
    /// let x = v?;
    /// ```
    pub BAN_UNWRAP,
    Warn,
    "bare `unwrap` panics without a reason; use `expect(\"...\")` or `?`"
}

fn is_option_or_result(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    let Some(adt) = ty.ty_adt_def() else { return false };
    let did = adt.did();
    tcx.is_diagnostic_item(sym::Option, did) || tcx.is_diagnostic_item(sym::Result, did)
}

impl<'tcx> LateLintPass<'tcx> for BanUnwrap {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::MethodCall(path, receiver, _, _) = expr.kind else { return };
        let name = path.ident.name.as_str();
        if name != "unwrap" { return; }

        let recv_ty = cx.typeck_results().expr_ty_adjusted(receiver);
        if !is_option_or_result(cx.tcx, recv_ty) { return; }

        span_lint(
            cx,
            BAN_UNWRAP,
            expr.span,
            "bare `unwrap` panics without a reason; use `expect(\"...\")` or `?`",
        );
    }
}

impl Default for BanUnwrap {
    fn default() -> Self {
        BanUnwrap
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
