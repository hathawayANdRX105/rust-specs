#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use rustc_hir::def::Res;
use rustc_hir::{Item, ItemKind, PrimTy, QPath, Ty, TyKind};
use rustc_lint::{LateContext, LateLintPass};

dylint_linting::declare_late_lint! {
    /// Checks the props contract of Dioxus components.
    ///
    /// For every `#[derive(Props)]` struct:
    /// - a field named `handle_*` is an implementation word, not a contract
    ///   word: rename it to `on_*` (`on_change`, `on_open_change`, ...);
    /// - a `render_*: bool` / `show_*: bool` field is a switch that bloats the
    ///   API: use a closed style variant enum instead;
    /// - more than 7 fields: split the component or use a parameter struct.
    ///
    /// ### Why is this bad?
    ///
    /// Props are the contract consumers depend on. `handle_*` leaks the
    /// implementation into the contract, `render_x: bool` turns a closed set of
    /// styles into an open boolean matrix, and a wide props struct is a sign
    /// the component does too much.
    ///
    /// ### Known problems
    ///
    /// Only `#[derive(Props)]` structs are inspected. A props struct built by
    /// another derive (or hand-written) is not.
    ///
    /// ### Example
    ///
    /// ```rust
    /// #[derive(Props, PartialEq, Clone)]
    /// struct CardProps {
    ///     handle_close: EventHandler<()>,
    ///     render_footer: bool,
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// #[derive(Props, PartialEq, Clone)]
    /// struct CardProps {
    ///     on_close: EventHandler<()>,
    ///     footer: Option<Element>,
    /// }
    /// ```
    pub DIOXUS_PROPS_CONTRACT,
    Warn,
    "props contract: on_* callbacks, no render_x/show_x bool, field count <= 7"
}

const HANDLE_MSG: &str = "props 字段 `handle_*` 是实现词，不是契约词：改名为 `on_*`（如 on_change / on_open_change）";
const BOOL_MSG: &str = "props 字段 `render_*` / `show_*: bool` 是开关膨胀：用封闭的 variant 枚举表达样式集合";
const BLOAT_MSG: &str = "props 字段超过 7 个：拆子组件或收敛成参数结构体";

fn has_props_derive(cx: &LateContext<'_>, item: &Item<'_>) -> bool {
    cx.tcx.hir_attrs(item.hir_id()).iter().any(|attr| {
        attr.has_name(rustc_span::sym::derive)
            && attr.meta_item_list().is_some_and(|list| {
                list.iter().any(|nested| {
                    nested
                        .ident()
                        .is_some_and(|ident| ident.name.as_str() == "Props")
                })
            })
    })
}

fn is_bool_ty(ty: &Ty<'_>) -> bool {
    matches!(
        ty.kind,
        TyKind::Path(QPath::Resolved(_, path))
            if matches!(path.res, Res::PrimTy(PrimTy::Bool))
    )
}

/// Returns the reason a field name violates the contract, if any.
fn field_problem(name: &str) -> Option<&'static str> {
    if name.starts_with("handle_") {
        return Some(HANDLE_MSG);
    }
    if name.starts_with("render_") || name.starts_with("show_") {
        return Some(BOOL_MSG);
    }
    None
}

impl<'tcx> LateLintPass<'tcx> for DioxusPropsContract {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let ItemKind::Struct(ident, _, data) = item.kind else {
            return;
        };
        if !has_props_derive(cx, item) {
            return;
        }
        let fields = data.fields();
        if fields.len() > 7 {
            span_lint(cx, DIOXUS_PROPS_CONTRACT, ident.span, BLOAT_MSG);
        }
        for field in fields {
            let ident = field.ident;
            let name = ident.name.as_str();
            if name.is_empty() {
                continue;
            }
            if name.starts_with("render_") || name.starts_with("show_") {
                if is_bool_ty(field.ty) {
                    span_lint(cx, DIOXUS_PROPS_CONTRACT, ident.span, BOOL_MSG);
                }
                continue;
            }
            if let Some(message) = field_problem(name) {
                span_lint(cx, DIOXUS_PROPS_CONTRACT, ident.span, message);
            }
        }
    }
}

impl Default for DioxusPropsContract {
    fn default() -> Self {
        DioxusPropsContract
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
    fn flags_handle_prefix() {
        assert_eq!(field_problem("handle_close"), Some(HANDLE_MSG));
        assert_eq!(field_problem("handler"), None);
        assert_eq!(field_problem("on_close"), None);
    }

    #[test]
    fn flags_render_show_prefix() {
        assert_eq!(field_problem("render_footer"), Some(BOOL_MSG));
        assert_eq!(field_problem("show_badge"), Some(BOOL_MSG));
        assert_eq!(field_problem("rendered_at"), None);
    }
}
