#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_attr_ir;
extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use rustc_attr_ir::{Attribute as HirAttribute, AttributeKind};
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::symbol::{Symbol, sym};

dylint_linting::declare_late_lint! {
    /// Every test function must carry a doc comment that says what it verifies.
    ///
    /// A bare `#[test] fn check_something() {}` leaves the next reader guessing which
    /// behavior the test protects. The project requires a `///` line in English stating
    /// the assertion, for top-level tests as well as for tests nested in a
    /// `#[cfg(test)]` module next to the code they exercise.
    ///
    /// ### Why is this bad?
    ///
    /// Undocumented tests are hard to review and hard to keep: when an assertion changes,
    /// there is no statement of intent to check the change against. The cost is one line
    /// per test.
    ///
    /// ### Known problems
    ///
    /// Only `fn` items with a `#[test]`, `#[core::test]`, `#[std::test]`, `#[test::test]`
    /// or `#[tokio::test]` attribute are checked. Other test frameworks with their own
    /// attributes are not recognized. Macro-generated functions are skipped, so a macro
    /// that emits test functions must document them at the call site or in its own docs.
    ///
    /// ### Example
    ///
    /// ```rust
    /// #[test]
    /// fn handles_empty_input() { /* ... */ }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// /// Empty input produces an empty result instead of panicking.
    /// #[test]
    /// fn handles_empty_input() { /* ... */ }
    /// ```
    pub TEST_FNS_HAVE_DOCS,
    Warn,
    "test function is missing a doc comment; add `///` explaining what it verifies"
}

/// Test functions detected in HIR:
///
/// - `#[test]`, `#[core::test]`, `#[std::test]`, `#[test::test]`, `#[tokio::test]` —
///   last segment `test`, optional first segment `core`/`std`/`test`/`tokio`.
/// - `#[rustc_test_entrypoint_marker]` — rustc replaces `#[test]` (and `#[tokio::test]`
///   expanded through it) with this marker when the crate is compiled with `--test`;
///   without `--test`, `#[test]` functions are dropped from HIR entirely, so this
///   marker is what remains to detect them.
fn is_test_attr(attr: &HirAttribute) -> bool {
    if matches!(
        attr,
        HirAttribute::Parsed(AttributeKind::RustcTestEntrypointMarker)
    ) {
        return true;
    }
    let path = attr.path();
    let Some(last) = path.last() else {
        return false;
    };
    if *last != sym::test {
        return false;
    }
    if path.len() == 1 {
        return true;
    }
    let first = path[0];
    first == sym::core || first == sym::std || first == sym::test || first == Symbol::intern("tokio")
}

/// User-written doc comments are lowered to `#[doc = "..."]` attributes. Rustc's
/// entrypoint marker reports its path as `doc`, so it must not be mistaken for one.
fn is_doc_attr(attr: &HirAttribute) -> bool {
    attr.path().last().is_some_and(|last| *last == sym::doc)
        && !matches!(
            attr,
            HirAttribute::Parsed(AttributeKind::RustcTestEntrypointMarker)
        )
}

impl<'tcx> LateLintPass<'tcx> for TestFnsHaveDocs {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &Item<'tcx>) {
        if !matches!(item.kind, ItemKind::Fn { .. }) || item.span.from_expansion() {
            return;
        }

        let attrs = cx.tcx.hir_attrs(item.hir_id());
        if attrs.iter().any(is_doc_attr) {
            return;
        }
        if attrs.iter().any(is_test_attr) {
            span_lint(
                cx,
                TEST_FNS_HAVE_DOCS,
                item.span,
                "test function is missing a doc comment; add `///` explaining what it verifies",
            );
        }
    }
}

impl Default for TestFnsHaveDocs {
    fn default() -> Self {
        TestFnsHaveDocs
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
