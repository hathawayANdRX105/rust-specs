#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_middle;

use clippy_utils::diagnostics::span_lint;
use rustc_hir::def_id::DefId;
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::def::Res;
use rustc_hir::{Expr, ExprKind, HirId, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::Ty;
use std::collections::HashSet;

dylint_linting::declare_late_lint! {
    /// Detects `use_effect` closures that write a `Signal` they also read.
    ///
    /// A `use_effect` closure that reads a signal and writes the same signal
    /// creates a self-loop: the write schedules the effect again, and the
    /// component re-renders forever (or until the value stabilizes by
    /// accident). The read can be direct (`(signal)()`, `.read()`, `.cloned()`,
    /// `.peek()`) and the write can be `.set()`, `.write()` or `.with_mut()`,
    /// including inside a `spawn(async move { ... })` nested in the closure.
    ///
    /// ### Why is this bad?
    ///
    /// Dioxus re-runs an effect when a signal read during the effect changes.
    /// Writing that same signal from the effect closes the loop. Derive the
    /// value with `use_memo` instead, or move the write out of the effect
    /// (event handler / explicit action).
    ///
    /// ### Known problems
    ///
    /// Signals are tracked by resolving the receiver to a local or a `static`;
    /// a write through a re-borrowed field (`self.state`) is not tracked.
    /// Effects that intentionally read and write the same signal once (e.g.
    /// "reset to default if invalid") are reported too; allow the lint at the
    /// call site with a written reason.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use_effect(move || {
    ///     let current = value();
    ///     if current > 10 {
    ///         value.set(10);
    ///     }
    /// });
    /// ```
    pub DIOXUS_EFFECT_SELF_LOOP,
    Warn,
    "use_effect closure writes a Signal it also reads (self-loop)"
}

const MSG: &str =
    "use_effect 闭包读写了同一个 Signal `{name}`（自环）：用 use_memo 派生，或把写操作移出 effect";

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum SignalKey {
    Local(HirId),
    Def(DefId),
}

fn is_signal_ty(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let ty = cx.typeck_results().expr_ty_adjusted(expr);
    is_signal_ty_inner(cx, ty)
}

fn is_signal_ty_inner(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let Some(adt) = ty.ty_adt_def() else {
        return false;
    };
    let path = cx.tcx.def_path_str(adt.did());
    path.contains("Signal")
}

fn signal_key(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<SignalKey> {
    let ExprKind::Path(qpath) = expr.kind else {
        return None;
    };
    match cx.qpath_res(&qpath, expr.hir_id) {
        Res::Local(hir_id) => Some(SignalKey::Local(hir_id)),
        Res::Def(_, did) => Some(SignalKey::Def(did)),
        _ => None,
    }
}

fn signal_name(expr: &Expr<'_>) -> String {
    if let ExprKind::Path(QPath::Resolved(_, path)) = expr.kind
        && let Some(segment) = path.segments.last()
    {
        return segment.ident.name.to_string();
    }
    "signal".to_string()
}

fn is_use_effect(cx: &LateContext<'_>, callee: &Expr<'_>) -> bool {
    let ExprKind::Path(qpath) = callee.kind else {
        return false;
    };
    let Some(did) = cx.qpath_res(&qpath, callee.hir_id).opt_def_id() else {
        return false;
    };
    cx.tcx.item_name(did).as_str() == "use_effect"
}

struct SignalIo<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    reads: HashSet<SignalKey>,
    writes: HashSet<SignalKey>,
    names: Vec<(SignalKey, String)>,
}

impl<'tcx> Visitor<'tcx> for SignalIo<'_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        match expr.kind {
            ExprKind::MethodCall(path, receiver, _, _) => {
                if is_signal_ty(self.cx, receiver) {
                    let name = path.ident.name.as_str();
                    let key = signal_key(self.cx, receiver);
                    match name {
                        "read" | "cloned" | "peek" => {
                            if let Some(key) = key {
                                self.reads.insert(key);
                                self.names.push((key, signal_name(receiver)));
                            }
                        }
                        "set" | "write" | "with_mut" => {
                            if let Some(key) = key {
                                self.writes.insert(key);
                                self.names.push((key, signal_name(receiver)));
                            }
                        }
                        _ => {}
                    }
                }
            }
            ExprKind::Call(callee, _) => {
                // `(signal)()` reads the signal.
                if is_signal_ty(self.cx, callee)
                    && let Some(key) = signal_key(self.cx, callee)
                {
                    self.reads.insert(key);
                    self.names.push((key, signal_name(callee)));
                }
            }
            _ => {}
        }
        intravisit::walk_expr(self, expr);
    }
}

impl<'tcx> LateLintPass<'tcx> for DioxusEffectSelfLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let ExprKind::Call(callee, args) = expr.kind else {
            return;
        };
        if !is_use_effect(cx, callee) {
            return;
        }
        let Some(closure) = args.first() else {
            return;
        };
        let ExprKind::Closure(closure) = closure.kind else {
            return;
        };
        let body = cx.tcx.hir_body(closure.body);
        let mut io = SignalIo {
            cx,
            reads: HashSet::new(),
            writes: HashSet::new(),
            names: Vec::new(),
        };
        io.visit_expr(body.value);

        let mut reported = HashSet::new();
        for key in io.reads.intersection(&io.writes) {
            if !reported.insert(*key) {
                continue;
            }
            let name = io
                .names
                .iter()
                .find(|(candidate, _)| candidate == key)
                .map(|(_, name)| name.as_str())
                .unwrap_or("signal");
            let message = MSG.replace("{name}", name);
            span_lint(cx, DIOXUS_EFFECT_SELF_LOOP, expr.span, message);
        }
    }
}

impl Default for DioxusEffectSelfLoop {
    fn default() -> Self {
        DioxusEffectSelfLoop
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
