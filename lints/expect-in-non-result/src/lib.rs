#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use rustc_hir::def::DefKind;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_span::symbol::sym;

dylint_linting::declare_late_lint! {
    /// In a function that returns `Result`/`Option`, prefer `?` over
    /// `.expect("...")` so failures propagate instead of panicking. When a
    /// function does NOT return `Result`/`Option`, `?` is unavailable and
    /// `.expect("...")` panics on the failure path with no way for the caller
    /// to recover — consider returning `Result`/`Option` (or handling the
    /// error explicitly) instead.
    ///
    /// ### Why is this bad?
    ///
    /// `.expect("...")` panics when the value is absent. If the enclosing
    /// function cannot propagate the error (`?` is unavailable because the
    /// return type is not `Result`/`Option`), the panic escapes to the
    /// caller with no recovery path.
    ///
    /// ### Known problems
    ///
    /// The check uses the enclosing function body's return type. Closures are
    /// skipped (`fn_sig` would panic on them and their body is usually not the
    /// function whose callers must recover). `main`, event handlers and other
    /// infallible callbacks will be flagged; add `#[allow(expect_in_non_result)]`
    /// where returning `Result` is not appropriate.
    ///
    /// ### Example
    ///
    /// ```rust
    /// fn load(path: &str) -> String {
    ///     std::fs::read_to_string(path).expect("config file must exist")
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// fn load(path: &str) -> std::io::Result<String> {
    ///     std::fs::read_to_string(path)
    /// }
    /// ```
    pub EXPECT_IN_NON_RESULT,
    Warn,
    "`expect` is used in a function that does not return Result/Option; `?` is unavailable here"
}

fn is_option_or_result(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    let Some(adt) = ty.ty_adt_def() else {
        return false;
    };
    let did = adt.did();
    tcx.is_diagnostic_item(sym::Option, did) || tcx.is_diagnostic_item(sym::Result, did)
}

impl<'tcx> LateLintPass<'tcx> for ExpectInNonResult {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::MethodCall(path, receiver, _, _) = expr.kind else {
            return;
        };
        if path.ident.name.as_str() != "expect" {
            return;
        }

        // Only `Result`/`Option` receivers matter; `.expect` on other types is
        // a different pattern (e.g. a string slice has no fallible unwrap).
        let recv_ty = cx.typeck_results().expr_ty_adjusted(receiver);
        if !is_option_or_result(cx.tcx, recv_ty) {
            return;
        }

        // Only function bodies (including methods) have a signature. Closures
        // are out of scope: `fn_sig` would panic on them, and their body is
        // usually not the function whose callers must recover.
        let def_id = cx.tcx.hir_enclosing_body_owner(expr.hir_id).to_def_id();
        match cx.tcx.def_kind(def_id) {
            DefKind::Fn | DefKind::AssocFn => {}
            _ => return,
        }

        // If the function returns `Result`/`Option`, `?` is available and the
        // `.expect` should be replaced with it — that is a different smell
        // (`unwrap_in_fallible`), not this rule.
        let sig = cx.tcx.fn_sig(def_id).skip_binder();
        let output = sig.output().skip_binder();
        if is_option_or_result(cx.tcx, output) {
            return;
        }

        span_lint(
            cx,
            EXPECT_IN_NON_RESULT,
            expr.span,
            "`expect` is used in a function that does not return Result/Option; `?` is unavailable here",
        );
    }
}

impl Default for ExpectInNonResult {
    fn default() -> Self {
        ExpectInNonResult
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}