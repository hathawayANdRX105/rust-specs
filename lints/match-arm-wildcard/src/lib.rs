#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;

use clippy_utils::diagnostics::span_lint;
use rustc_hir::{Arm, PatKind};
use rustc_lint::{LateContext, LateLintPass};

dylint_linting::declare_late_lint! {
    /// A `match` arm whose whole pattern is the bare wildcard `_` throws away
    /// the matched value without giving it a name. Bind it instead (e.g.
    /// `other => ...`) so the payload is visible to readers and can be logged,
    /// forwarded or asserted on when the arm is debugged.
    ///
    /// ### Why is this bad?
    ///
    /// A bare `_` hides what the fallback arm actually received. A named
    /// binding documents the value and makes future changes (e.g. logging the
    /// unexpected variant) a one-line edit instead of a restructuring of the
    /// match.
    ///
    /// ### Known problems
    ///
    /// Only the whole-pattern wildcard is reported. Wildcards nested inside
    /// patterns, such as `Some(_)` or `Foo(_, _)`, are intentionally allowed,
    /// as are arms coming from macro expansions.
    ///
    /// ### Example
    ///
    /// ```rust
    /// match value {
    ///     Some(x) => use_it(x),
    ///     _ => (),
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// match value {
    ///     Some(x) => use_it(x),
    ///     other => log::debug!("unexpected: {other:?}"),
    /// }
    /// ```
    pub MATCH_ARM_WILDCARD,
    Warn,
    "match arm uses bare `_`; use a named binding (e.g. `other`) so the payload has a name"
}

impl<'tcx> LateLintPass<'tcx> for MatchArmWildcard {
    fn check_arm(&mut self, cx: &LateContext<'tcx>, arm: &'tcx Arm<'tcx>) {
        if arm.span.from_expansion() {
            return;
        }
        if let PatKind::Wild = arm.pat.kind {
            span_lint(
                cx,
                MATCH_ARM_WILDCARD,
                arm.pat.span,
                "match arm uses bare `_`; use a named binding (e.g. `other`) so the payload has a name",
            );
        }
    }
}

impl Default for MatchArmWildcard {
    fn default() -> Self {
        MatchArmWildcard
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
