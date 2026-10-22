#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;

use clippy_utils::diagnostics::span_lint;
use dioxus_lint_scope::span_allowed;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};

dylint_linting::declare_late_lint! {
    /// A shared component crate must not import business types.
    ///
    /// `ui-components` is the shared component crate: it takes props and emits
    /// events, and must not know about `ui_api`, `app_contract` or page DTOs.
    /// Business semantics belong to the page crate that composes the component.
    ///
    /// ### Why is this bad?
    ///
    /// A shared component importing a business crate couples every consumer to
    /// that business model. The component can no longer be reused across pages
    /// or apps, and the dependency direction page -> component is reversed.
    ///
    /// ### Known problems
    ///
    /// Only direct `use` items are checked; a fully-qualified path or a
    /// re-export is not caught. The crate scope is hard-coded to
    /// `ui_components`.
    ///
    /// ### Example
    ///
    /// ```rust
    /// // in crates/ui-components/src/card.rs
    /// use ui_api::dto::Channel;
    /// ```
    ///
    /// Use instead: define a plain props struct in `ui-components` and let the
    /// page map its DTO to those props.
    pub DIOXUS_OWNERSHIP_IMPORTS,
    Warn,
    "shared component crate imports a business crate (ui_api/app_contract)"
}

const MSG: &str =
    "共享组件 crate（ui-components）引用业务类型：业务语义留在页面层，共享组件只收 props / 发事件";

fn is_business_crate(name: &str) -> bool {
    matches!(name, "ui_api" | "app_contract")
}

impl<'tcx> LateLintPass<'tcx> for DioxusOwnershipImports {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if !span_allowed(cx, item.span) {
            return;
        }
        let crate_name = cx.tcx.crate_name(LOCAL_CRATE);
        if crate_name.as_str() != "ui_components" {
            return;
        }
        let ItemKind::Use(path, _) = item.kind else {
            return;
        };
        let first = path
            .segments
            .first()
            .map(|segment| segment.ident.name.as_str());
        if let Some(first) = first
            && is_business_crate(first)
        {
            span_lint(cx, DIOXUS_OWNERSHIP_IMPORTS, item.span, MSG);
        }
    }
}

impl Default for DioxusOwnershipImports {
    fn default() -> Self {
        DioxusOwnershipImports
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_business_crates() {
        assert!(is_business_crate("ui_api"));
        assert!(is_business_crate("app_contract"));
        assert!(!is_business_crate("ui_models"));
        assert!(!is_business_crate("ui_i18n"));
    }
}
