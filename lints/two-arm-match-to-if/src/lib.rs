#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use rustc_hir::{Expr, ExprKind, MatchSource, Pat, PatKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::kw;

dylint_linting::declare_late_lint! {
    /// A `match` with exactly two arms where one arm is a catch-all (`_`) and
    /// the other binds nothing is a binary choice and should be an `if`/`else`
    /// block instead. Reading `if`/`else` is more direct for a yes/no decision
    /// than a `match`, and it keeps the control flow flat.
    ///
    /// ### Why is this bad?
    ///
    /// A two-arm `match` with a `_` arm is a binary branch, which is exactly
    /// what `if`/`else` expresses. Using `match` here adds indirection without
    /// adding expressive power.
    ///
    /// ### Known problems
    ///
    /// Matches with guards, more than two arms, or a non-catch-all arm that
    /// binds a payload are left alone: they cannot be translated to a plain
    /// `if`/`else` without changing semantics.
    ///
    /// ### Example
    ///
    /// ```rust
    /// match v {
    ///     true => 1,
    ///     _ => 2,
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// if v { 1 } else { 2 }
    /// ```
    pub TWO_ARM_MATCH_TO_IF,
    Warn,
    "`match` with one specific arm and one `_` arm is a binary choice; use `if`/`else`"
}

/// True when `pat` (or any nested sub-pattern) contains a `PatKind::Binding`,
/// i.e. it binds a payload that cannot be moved into an `if`/`else`.
fn pat_has_binding(pat: &Pat<'_>) -> bool {
    match pat.kind {
        PatKind::Binding(..) => true,
        PatKind::Wild | PatKind::Never | PatKind::Missing | PatKind::Err(_) => false,
        PatKind::Expr(_) | PatKind::Range(_, _, _) => false,
        PatKind::Struct(_, fields, _) => fields.iter().any(|field| pat_has_binding(&field.pat)),
        PatKind::TupleStruct(_, subpats, _) => subpats.iter().any(pat_has_binding),
        PatKind::Or(subpats) => subpats.iter().any(pat_has_binding),
        PatKind::Tuple(subpats, _) => subpats.iter().any(pat_has_binding),
        PatKind::Box(p) | PatKind::Deref(p) => pat_has_binding(p),
        PatKind::Ref(p, _, _) => pat_has_binding(p),
        PatKind::Guard(p, _) => pat_has_binding(p),
        PatKind::Slice(before, mid, after) => {
            before.iter().any(pat_has_binding)
                || mid.map_or(false, pat_has_binding)
                || after.iter().any(pat_has_binding)
        }
    }
}

/// True when `pat` is a catch-all: a bare `_` or a binding named `_`.
fn is_catch_all(pat: &Pat<'_>) -> bool {
    matches!(pat.kind, PatKind::Wild)
        || matches!(pat.kind, PatKind::Binding(_, _, ident, _) if ident.name == kw::Underscore)
}

impl<'tcx> LateLintPass<'tcx> for TwoArmMatchToIf {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::Match(_, arms, MatchSource::Normal) = expr.kind else {
            return;
        };
        if arms.len() != 2 {
            return;
        }
        // An arm with a guard cannot be rewritten to a plain `if`/`else`.
        if arms.iter().any(|arm| arm.guard.is_some()) {
            return;
        }
        let catch_all_count = arms.iter().filter(|arm| is_catch_all(&arm.pat)).count();
        if catch_all_count != 1 {
            return; // both specific, or both catch-all
        }
        let Some(specific) = arms.iter().find(|arm| !is_catch_all(&arm.pat)) else {
            return;
        };
        // The specific arm must bind nothing: literals, unit, consts, variant
        // paths without subpatterns (`None`, `Color::Red`), bool literals, etc.
        if pat_has_binding(&specific.pat) {
            return;
        }
        span_lint(
            cx,
            TWO_ARM_MATCH_TO_IF,
            expr.span,
            "`match` with one specific arm and one `_` arm is a binary choice; use `if`/`else`",
        );
    }
}

impl Default for TwoArmMatchToIf {
    fn default() -> Self {
        TwoArmMatchToIf
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
