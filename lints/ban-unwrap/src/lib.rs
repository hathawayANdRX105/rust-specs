#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_span::symbol::sym;

dylint_linting::declare_late_lint! {
    /// This project bans `.unwrap()` on `Result`/`Option`. Replace every
    /// bare `unwrap` with `.expect("...")` where the message, written in
    /// English, explains why the unwrap is safe (the invariant that makes
    /// the failure unreachable). In a fallible function, prefer `?` to
    /// propagate the error instead.
    ///
    /// ### Why is this bad?
    ///
    /// `unwrap` hides why a panic can happen. `expect("reason")` forces the
    /// author to state the invariant; the project requires English messages.
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
    /// let y = v.expect("值必须存在");
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
    "unwrap is banned; use `expect(\"...\")` with an English reason, or `?`"
}

fn is_option_or_result(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    let Some(adt) = ty.ty_adt_def() else { return false };
    let did = adt.did();
    tcx.is_diagnostic_item(sym::Option, did) || tcx.is_diagnostic_item(sym::Result, did)
}

/// True when `s` contains any CJK Unified Ideograph (U+4E00..U+9FFF).
fn contains_cjk(s: &str) -> bool {
    s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
}

impl<'tcx> LateLintPass<'tcx> for BanUnwrap {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::MethodCall(path, receiver, args, _) = expr.kind else { return };
        let name = path.ident.name.as_str();

        let recv_ty = cx.typeck_results().expr_ty_adjusted(receiver);
        if !is_option_or_result(cx.tcx, recv_ty) {
            return;
        }

        if name == "unwrap" {
            span_lint_and_sugg(
                cx,
                BAN_UNWRAP,
                expr.span,
                "`unwrap` is banned in this project; it panics without explaining why",
                "use `expect(\"...\")` with an English reason explaining why the unwrap is safe",
                "expect(\"reason in english\")".to_string(),
                Applicability::HasPlaceholders,
            );
            return;
        }

        if name == "expect" {
            // First argument is the message string literal.
            if let Some(first) = args.first()
                && let ExprKind::Lit(lit) = first.kind
                && let rustc_ast::LitKind::Str(sym_str, _) = lit.node
                && contains_cjk(sym_str.as_str())
            {
                span_lint_and_sugg(
                    cx,
                    BAN_UNWRAP,
                    first.span,
                    "`expect` messages must be English; write why the unwrap is safe in English",
                    "rewrite the message in English",
                    "expect(\"reason in english\")".to_string(),
                    Applicability::HasPlaceholders,
                );
            }
            return;
        }
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
