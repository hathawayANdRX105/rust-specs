#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_attr_ir;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use dylint_linting::declare_late_lint;
use rustc_attr_ir::Attribute as HirAttribute;
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::Visibility;
use rustc_span::sym;

declare_late_lint! {
    /// Every `pub` item must carry a doc comment (`///` or `/** */`) explaining its
    /// purpose. Bare `//` comments do not count as documentation.
    ///
    /// ### Why is this bad?
    ///
    /// Undocumented public items leave consumers guessing about purpose, invariants,
    /// and usage; doc comments are the public contract of an item.
    ///
    /// ### Known problems
    ///
    /// The lint only checks for the presence of a doc attribute, not its quality or
    /// completeness. Items intentionally left undocumented can opt out with
    /// `#[allow(pub_items_have_docs)]`.
    ///
    /// ### Example
    ///
    /// ```rust
    /// pub fn helper() {}                      // bad: missing doc comment
    /// pub(crate) fn helper() {}               // ok: not plainly `pub`
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// /// Does the thing.
    /// pub fn helper() {}
    /// ```
    pub PUB_ITEMS_HAVE_DOCS,
    Warn,
    "`pub` item is missing a doc comment; add `/// ...` explaining purpose"
}

/// True when `attr` is `#[cfg(test)]` (possibly `#[cfg(any(test, ...))]`).
fn is_cfg_test(attr: &HirAttribute) -> bool {
    attr.path().last().map_or(false, |sym| *sym == sym::cfg)
        && format!("{:?}", attr).contains("test")
}

/// True when `attrs` contains a doc attribute (`///`, `/** */`, `#[doc = "..."]`).
fn has_doc_attr(attrs: &[HirAttribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().last().map_or(false, |sym| *sym == sym::doc)
    })
}

impl<'tcx> LateLintPass<'tcx> for PubItemsHaveDocs {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &Item<'tcx>) {
        // Skip items produced by macro expansion.
        if item.span.from_expansion() {
            return;
        }

        let attrs = cx.tcx.hir_attrs(item.hir_id());

        // Only the item kinds this lint covers.
        match item.kind {
            ItemKind::Fn { .. }
            | ItemKind::Struct(..)
            | ItemKind::Union(..)
            | ItemKind::Enum(..)
            | ItemKind::Trait { .. }
            | ItemKind::TyAlias(..)
            | ItemKind::Static(..)
            | ItemKind::Const(..)
            | ItemKind::Mod(..) => {}
            _ => return,
        }

        // `mod tests` with `#[cfg(test)]` — test modules are exempt.
        if let ItemKind::Mod(ident, _) = item.kind
            && ident.name.as_str() == "tests"
            && attrs.iter().any(is_cfg_test)
        {
            return;
        }

        // Plain `pub` only — `pub(crate)`, `pub(super)`, `pub(in path)` are skipped.
        let Visibility::Public = cx.tcx.visibility(item.owner_id.def_id) else {
            return;
        };

        // Missing doc comment: no attribute whose path's last segment is `doc`.
        if !has_doc_attr(&attrs) {
            span_lint(
                cx,
                PUB_ITEMS_HAVE_DOCS,
                item.span,
                "`pub` item is missing a doc comment; add `/// ...` explaining purpose",
            );
        }
    }
}

impl Default for PubItemsHaveDocs {
    fn default() -> Self {
        PubItemsHaveDocs
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
