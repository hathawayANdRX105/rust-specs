#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use rustc_hir::{Expr, ExprKind};
use rustc_hir::def::DefKind;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_span::symbol::sym;

dylint_linting::declare_late_lint! {
    /// Checks for `.unwrap()` / `.expect()` calls inside functions whose return
    /// type is `Result` or `Option`. Such a call panics on failure instead of
    /// propagating the error with `?`.
    pub UNWRAP_IN_FALLIBLE,
    Warn,
    "unwrap/expect inside a Result/Option-returning function; use `?`"
}

fn is_option_or_result(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    let Some(adt) = ty.ty_adt_def() else { return false };
    let did = adt.did();
    tcx.is_diagnostic_item(sym::Option, did) || tcx.is_diagnostic_item(sym::Result, did)
}

impl<'tcx> LateLintPass<'tcx> for UnwrapInFallible {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::MethodCall(path, receiver, _, _) = expr.kind else { return };
        let name = path.ident.name.as_str();
        if name != "unwrap" && name != "expect" { return; }

        let recv_ty = cx.typeck_results().expr_ty_adjusted(receiver);
        if !is_option_or_result(cx.tcx, recv_ty) { return; }

        // The enclosing function must return Result/Option.
        let Some(body_id) = cx.enclosing_body else { return };
        let owner = cx.tcx.hir_enclosing_body_owner(body_id.hir_id);
        // Only functions/methods have a fn_sig; closures panic if queried this way.
        if !matches!(cx.tcx.def_kind(owner), DefKind::Fn | DefKind::AssocFn) {
            return;
        }
        let fn_sig = cx.tcx.type_of(owner).skip_binder().kind().fn_sig(cx.tcx).skip_binder();
        let ret_ty = fn_sig.output();
        if !is_option_or_result(cx.tcx, ret_ty) { return; }

        span_lint(
            cx,
            UNWRAP_IN_FALLIBLE,
            expr.span,
            "`unwrap`/`expect` inside a fallible function; use `?` instead",
        );
    }
}

impl Default for UnwrapInFallible {
    fn default() -> Self {
        UnwrapInFallible
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
