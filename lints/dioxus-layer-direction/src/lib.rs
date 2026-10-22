#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;

use clippy_utils::diagnostics::span_lint;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};

dylint_linting::declare_late_lint! {
    /// Enforces the web crate layering direction.
    ///
    /// - `components/`, `layouts/` and `utils/` must not import `views`
    ///   (`use crate::views::...` / `use super::views::...`);
    /// - a `ui_*` crate must not import server-side crates (`api_*`,
    ///   `gateway_*`, `app_db`).
    ///
    /// ### Why is this bad?
    ///
    /// Layering is one-way: views -> components/layouts/utils. A reverse
    /// dependency forces a low-level component to know page-level state, and
    /// the UI can no longer be split or reused. A UI crate importing a server
    /// crate drags the backend into the wasm build.
    ///
    /// ### Known problems
    ///
    /// The check is syntactic on `use` items: a re-exported path or a
    /// fully-qualified call (`crate::views::foo()`) is not caught.
    ///
    /// ### Example
    ///
    /// ```rust
    /// // in crates/ui-pages/src/components/card.rs
    /// use crate::views::dashboard::DashboardState;
    /// ```
    ///
    /// Use instead: pass the state through props.
    pub DIOXUS_LAYER_DIRECTION,
    Warn,
    "web layering: components/layouts/utils must not depend on views; ui-* must not import api_*/gateway_*/app_db"
}

const VIEW_MSG: &str =
    "components/layouts/utils 反向依赖 views：层级是单向的 views -> components/layouts/utils，改为通过 props 传状态";
const SERVER_MSG: &str =
    "ui-* crate 引用了服务端 crate（api_*/gateway_*/app_db）：会污染 wasm 构建，业务调用留在页面/宿主层";

fn is_layer_violation(segments: &[&str]) -> bool {
    matches!(segments, ["crate", "views", ..] | ["super", "views", ..])
}

fn is_server_crate(name: &str) -> bool {
    name.starts_with("api_") || name.starts_with("gateway_") || name.starts_with("app_db")
}

impl<'tcx> LateLintPass<'tcx> for DioxusLayerDirection {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let ItemKind::Use(path, _) = item.kind else {
            return;
        };
        let segments: Vec<&str> = path
            .segments
            .iter()
            .map(|segment| segment.ident.name.as_str())
            .collect();
        let file = cx
            .sess()
            .source_map()
            .lookup_char_pos(item.span.lo())
            .file
            .name
            .prefer_local_unconditionally()
            .to_string();
        let in_layer = file.contains("/components/")
            || file.contains("/layouts/")
            || file.contains("/utils/");
        if in_layer && is_layer_violation(&segments) {
            span_lint(cx, DIOXUS_LAYER_DIRECTION, item.span, VIEW_MSG);
        }
        let crate_name = cx.tcx.crate_name(LOCAL_CRATE);
        if crate_name.as_str().starts_with("ui_")
            && let Some(first) = segments.first()
            && is_server_crate(first)
        {
            span_lint(cx, DIOXUS_LAYER_DIRECTION, item.span, SERVER_MSG);
        }
    }
}

impl Default for DioxusLayerDirection {
    fn default() -> Self {
        DioxusLayerDirection
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
    fn detects_view_reverse_imports() {
        assert!(is_layer_violation(&["crate", "views", "dashboard"]));
        assert!(is_layer_violation(&["super", "views"]));
        assert!(!is_layer_violation(&["crate", "components", "card"]));
    }

    #[test]
    fn detects_server_crates() {
        assert!(is_server_crate("api_gateway"));
        assert!(is_server_crate("gateway_forward"));
        assert!(is_server_crate("app_db"));
        assert!(!is_server_crate("ui_api"));
    }
}
