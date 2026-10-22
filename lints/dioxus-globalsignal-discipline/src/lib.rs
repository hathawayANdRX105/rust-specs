#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use rustc_hir::def::DefKind;
use rustc_hir::{
    Expr, ExprKind, HirId, Item, ItemKind, LetStmt, QPath, Ty, TyKind,
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::Ty as Mty;
use rustc_span::Span;

dylint_linting::declare_late_lint! {
    /// Enforces the `GlobalSignal` discipline.
    ///
    /// A `static X: GlobalSignal<T>` is a process-wide writable singleton; it
    /// is legitimate only for genuinely global state (locale, toast queue,
    /// theme). Two mechanical checks:
    ///
    /// 1. the static declaration must carry a reason comment within the eight
    ///    lines above it (the comment explains why it must be global);
    /// 2. `GlobalSignal` must not appear outside a `static` declaration — no
    ///    `GlobalSignal::new` in a function body, no local `let x: GlobalSignal`.
    ///
    /// ### Why is this bad?
    ///
    /// Every global signal widens the implicit dependency graph and the
    /// re-render surface. State that only one component tree needs belongs in
    /// `use_signal` / a page store, not in a global.
    ///
    /// ### Known problems
    ///
    /// The reason check only requires a comment, not a good one (review still
    /// judges quality). Reads of an existing static are not flagged; only
    /// declarations and non-static construction/local types are.
    ///
    /// ### Example
    ///
    /// ```rust
    /// static ENABLED: GlobalSignal<bool> = GlobalSignal::new(true);
    /// ```
    ///
    /// Use instead (with a reason comment):
    ///
    /// ```rust
    /// /// Global by design: every page needs the current locale.
    /// static LOCALE: GlobalSignal<Locale> = Signal::global(|| Locale::Zh);
    /// ```
    pub DIOXUS_GLOBALSIGNAL_DISCIPLINE,
    Warn,
    "GlobalSignal must be a documented static singleton; no local or fn-body construction"
}

const REASON_MSG: &str =
    "GlobalSignal 声明缺理由注释：仅限全站单例，在声明上方写明「为什么必须全局」（8 行内）";
const USE_MSG: &str =
    "GlobalSignal 出现在非 static 位置：全局单例只能从 static 读，逐个人审是否应降级为 use_signal";

fn is_global_signal_name(path: &str) -> bool {
    path.rsplit("::").next().is_some_and(|name| name == "GlobalSignal")
}

fn is_global_signal_ty(cx: &LateContext<'_>, ty: Mty<'_>) -> bool {
    let Some(adt) = ty.ty_adt_def() else {
        return false;
    };
    is_global_signal_name(&cx.tcx.def_path_str(adt.did()))
}

fn ty_mentions_global_signal(ty: &Ty<'_>) -> bool {
    let TyKind::Path(QPath::Resolved(_, path)) = ty.kind else {
        return false;
    };
    path.segments
        .last()
        .is_some_and(|segment| segment.ident.name.as_str() == "GlobalSignal")
}

/// The eight lines above `span` (not including the item's own line) must
/// contain a `//` comment.
fn has_reason_comment(cx: &LateContext<'_>, span: Span) -> bool {
    let loc = cx.sess().source_map().lookup_char_pos(span.lo());
    let Some(source) = loc.file.src.as_deref().map(|source| source.as_str()) else {
        return true;
    };
    let line = loc.line; // 1-based
    let end = line.saturating_sub(1); // 0-based index of the line above
    let start = end.saturating_sub(8);
    source
        .lines()
        .skip(start)
        .take(end - start)
        .any(|line| line.contains("//"))
}

fn inside_static(cx: &LateContext<'_>, hir_id: HirId) -> bool {
    let owner = cx.tcx.hir_enclosing_body_owner(hir_id);
    matches!(cx.tcx.def_kind(owner), DefKind::Static { .. })
}

impl<'tcx> LateLintPass<'tcx> for DioxusGlobalsignalDiscipline {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if !matches!(item.kind, ItemKind::Static(..)) {
            return;
        }
        let ty = cx.tcx.type_of(item.owner_id.to_def_id()).skip_binder();
        if !is_global_signal_ty(cx, ty) {
            return;
        }
        if !has_reason_comment(cx, item.span) {
            span_lint(cx, DIOXUS_GLOBALSIGNAL_DISCIPLINE, item.span, REASON_MSG);
        }
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if inside_static(cx, expr.hir_id) {
            return;
        }
        let ExprKind::Call(callee, _) = expr.kind else {
            return;
        };
        let ExprKind::Path(qpath) = callee.kind else {
            return;
        };
        let Some(did) = cx.qpath_res(&qpath, callee.hir_id).opt_def_id() else {
            return;
        };
        let is_new = cx.tcx.item_name(did).as_str() == "new";
        if is_new && is_global_signal_name(&cx.tcx.def_path_str(did)) {
            span_lint(cx, DIOXUS_GLOBALSIGNAL_DISCIPLINE, expr.span, USE_MSG);
        }
    }

    fn check_local(&mut self, cx: &LateContext<'tcx>, local: &'tcx LetStmt<'tcx>) {
        let Some(ty) = local.ty else {
            return;
        };
        if ty_mentions_global_signal(ty) {
            span_lint(cx, DIOXUS_GLOBALSIGNAL_DISCIPLINE, local.span, USE_MSG);
        }
    }
}

impl Default for DioxusGlobalsignalDiscipline {
    fn default() -> Self {
        DioxusGlobalsignalDiscipline
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
    fn detects_global_signal_type_names() {
        assert!(is_global_signal_name("dioxus_signals::GlobalSignal"));
        assert!(is_global_signal_name("GlobalSignal"));
        assert!(!is_global_signal_name("dioxus_signals::Signal"));
    }
}
