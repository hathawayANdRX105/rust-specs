#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::diagnostics::{span_lint, span_lint_and_sugg};
use clippy_utils::eager_or_lazy::switch_to_eager_eval;
use clippy_utils::expr_adjustment_requires_coercion;
use clippy_utils::is_none_expr;
use clippy_utils::res::{MaybeDef as _, MaybeResPath as _};
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::sugg::Sugg;
use clippy_utils::visitors::for_each_expr_without_closures;
use rustc_errors::Applicability;
use rustc_hir::attrs::lang_items::LangItem::{self, OptionSome, ResultErr, ResultOk};
use rustc_hir::{Expr, ExprKind, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_span::symbol::sym;
use std::ops::ControlFlow;

dylint_linting::declare_late_lint! {
    /// Reports a `bool` `if`/`else` whose branches only wrap a value, i.e. the
    /// condition already is the `bool` the combinator chain starts from:
    ///
    /// ```rust
    /// # let ready = true;
    /// # let code = 7;
    /// let a = if ready { Some(1) } else { None };
    /// let b = if ready { Ok(1) } else { Err(code) };
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// # let ready = true;
    /// # let code = 7;
    /// let a = ready.then_some(1);
    /// let b = ready.then_some(1).ok_or(code);
    /// ```
    ///
    /// Four branch pairs are recognised, in both directions:
    ///
    /// - `if c { Some(v) } else { None }` → `c.then_some(v)`
    /// - `if c { None } else { Some(v) }` → `(!c).then_some(v)`
    /// - `if c { Ok(v) } else { Err(e) }` → `c.then_some(v).ok_or(e)`
    /// - `if c { Err(e) } else { Ok(v) }` → `(!c).then_some(v).ok_or(e)`
    ///
    /// `Result` takes the two-step chain because `bool::ok_or` returns
    /// `Result<(), E>`: it carries the error but not the `Ok` value, so the
    /// value has to travel through `Option` first. A `Result<(), E>` if/else
    /// has no value to carry and ends at the error alone:
    ///
    /// ```rust
    /// # let ready = true;
    /// # let code = 7;
    /// let c = if ready { Ok(()) } else { Err(code) };
    /// // ready.ok_or(code)
    /// ```
    ///
    /// `then_some` and `ok_or` evaluate their argument eagerly, so the
    /// suggestion switches to the lazy `then(|| v)` / `ok_or_else(|| e)` when
    /// the payload is not cheap (a call, an allocation). The rewrite then keeps
    /// the old evaluation timing instead of computing the payload every time.
    ///
    /// ### Why is this bad?
    ///
    /// The `if`/`else` spends five lines restating what one combinator chain
    /// says, and its branches carry no logic beyond the wrapper.
    ///
    /// ### Known problems
    ///
    /// - A lone `is_some`/`is_none`/`is_ok`/`is_err` condition is left to
    ///   `is-check-needs-chain`: there the receiver is already an
    ///   `Option`/`Result`, so `o.map(..)` beats `o.is_some().then_some(..)`.
    /// - Both branch blocks must be a single expression; a branch that binds
    ///   something first (`{ let n = ..; Some(n) }`) cannot move into a
    ///   combinator argument and is not reported.
    /// - `else if` chains are not reported as a whole; each leaf `if`/`else`
    ///   is its own expression and is checked on its own.
    /// - No suggestion is emitted when a payload needs a coercion at the
    ///   constructor call (unsize, deref): `then_some`/`ok_or` take it as a
    ///   generic argument, where that coercion would not apply.
    ///
    /// ### Example
    ///
    /// ```rust
    /// fn lookup(ready: bool) -> Option<u32> {
    ///     if ready { Some(1) } else { None }
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// fn lookup(ready: bool) -> Option<u32> {
    ///     ready.then_some(1)
    /// }
    /// ```
    pub BOOL_IF_ELSE_TO_COMBINATOR,
    Warn,
    "bool `if`/`else` that wraps a value in an `Option`/`Result`; use a combinator chain"
}

/// A recognised `if`/`else` that wraps a value in an `Option`/`Result`.
struct Shape<'hir> {
    /// `true` for `Result` (the chain ends in `ok_or`/`ok_or_else`), `false`
    /// for `Option` (the chain ends at `then_some`/`then`).
    result: bool,
    /// The `Some` value or the `Ok` value — the payload the chain carries.
    value: &'hir Expr<'hir>,
    /// The `Err` payload; `None` for the `Option` shapes.
    err: Option<&'hir Expr<'hir>>,
    /// The branches are swapped (`else` carries the value), so the condition
    /// has to be negated.
    negated: bool,
}

/// The payload of a one-argument constructor call (`Some(v)` / `Ok(v)` /
/// `Err(v)`) of the given language item.
fn ctor_payload<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    item: LangItem,
) -> Option<&'tcx Expr<'tcx>> {
    let ExprKind::Call(callee, [arg]) = expr.kind else {
        return None;
    };
    callee
        .basic_res()
        .ctor_parent(cx)
        .is_lang_item(cx, item)
        .then_some(arg)
}

/// The four branch pairs that only wrap a value, in both directions.
fn shape<'tcx>(
    cx: &LateContext<'tcx>,
    then: &'tcx Expr<'tcx>,
    els: &'tcx Expr<'tcx>,
) -> Option<Shape<'tcx>> {
    // Option: `Some(v)` on one side, `None` on the other.
    if let Some(value) = ctor_payload(cx, then, OptionSome)
        && is_none_expr(cx, els)
    {
        return Some(Shape { result: false, value, err: None, negated: false });
    }
    if let Some(value) = ctor_payload(cx, els, OptionSome)
        && is_none_expr(cx, then)
    {
        return Some(Shape { result: false, value, err: None, negated: true });
    }
    // Result: `Ok(v)` on one side, `Err(e)` on the other; both payloads travel
    // into the chain.
    if let Some(value) = ctor_payload(cx, then, ResultOk)
        && let Some(err) = ctor_payload(cx, els, ResultErr)
    {
        return Some(Shape { result: true, value, err: Some(err), negated: false });
    }
    if let Some(value) = ctor_payload(cx, els, ResultOk)
        && let Some(err) = ctor_payload(cx, then, ResultErr)
    {
        return Some(Shape { result: true, value, err: Some(err), negated: true });
    }
    None
}

fn is_option_or_result(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    let Some(adt) = ty.peel_refs().ty_adt_def() else {
        return false;
    };
    let did = adt.did();
    tcx.is_diagnostic_item(sym::Option, did) || tcx.is_diagnostic_item(sym::Result, did)
}

/// A lone `is_some`/`is_none`/`is_ok`/`is_err` on an `Option`/`Result`. The
/// receiver is already the target shape, so `is-check-needs-chain` owns this
/// form and its `map`/`map_or` rewrite.
fn is_check_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let ExprKind::MethodCall(path, receiver, [], _) = expr.kind else {
        return false;
    };
    ["is_some", "is_none", "is_ok", "is_err"].contains(&&*path.ident.name.as_str())
        && is_option_or_result(cx.tcx, cx.typeck_results().expr_ty_adjusted(receiver))
}

/// A condition that contains a `let` (`if let ..`, let-chains) type-checks as
/// `bool` but is not an expression that can move behind a method call.
fn has_let_condition(expr: &Expr<'_>) -> bool {
    for_each_expr_without_closures(expr, |e| {
        if matches!(e.kind, ExprKind::Let(..)) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })
    .is_some()
}

impl<'tcx> LateLintPass<'tcx> for BoolIfElseToCombinator {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion() {
            return;
        }
        // `if cond { A } else { B }`; an `else if` chain is not a single
        // if/else and is left to its leaves.
        let ExprKind::If(cond, then, Some(els)) = expr.kind else {
            return;
        };
        let ExprKind::Block(then_block, None) = then.kind else {
            return;
        };
        let ExprKind::Block(else_block, None) = els.kind else {
            return;
        };
        // A combinator takes one argument: a branch with statements cannot move.
        if !then_block.stmts.is_empty() || !else_block.stmts.is_empty() {
            return;
        }
        let (Some(then_expr), Some(else_expr)) = (then_block.expr, else_block.expr) else {
            return;
        };
        // `if let ..` / let-chains type-check as `bool` but are not an
        // expression that can move behind a method call; those belong to
        // `chainable-single-statement`.
        if has_let_condition(cond) {
            return;
        }
        if is_check_call(cx, cond) {
            return;
        }
        let Some(shape) = shape(cx, then_expr, else_expr) else {
            return;
        };
        // The branches only wrap a value when the whole if/else resolves to
        // that ADT; a plain binary value choice is the repo default and is
        // never reported.
        let expected = if shape.result { sym::Result } else { sym::Option };
        if !cx
            .typeck_results()
            .expr_ty(expr)
            .peel_refs()
            .ty_adt_def()
            .is_some_and(|adt| cx.tcx.is_diagnostic_item(expected, adt.did()))
        {
            return;
        }

        // `then_some`/`ok_or` take their payload as a generic argument, so a
        // coercion at the constructor call (unsize, deref) would not apply
        // there; the shape is still reported, without a rewrite.
        if expr_adjustment_requires_coercion(cx, shape.value)
            || shape.err.is_some_and(|err| expr_adjustment_requires_coercion(cx, err))
        {
            span_lint(
                cx,
                BOOL_IF_ELSE_TO_COMBINATOR,
                expr.span,
                "bool `if`/`else` wraps a value in `Option`/`Result` but a payload needs a coercion the combinator cannot express",
            );
            return;
        }

        // `then_some`/`ok_or` evaluate their payload eagerly, so use them only
        // when the payload is cheap; otherwise the lazy `then`/`ok_or_else`
        // keeps the old evaluation timing.
        let mut app = Applicability::MachineApplicable;
        let ctxt = expr.span.ctxt();
        let cond_sugg = Sugg::hir_with_context(cx, cond, ctxt, "..", &mut app);
        // The negated form needs its own parentheses: `!c.then_some(v)` parses
        // as `!(..)`. A negated shape over `!c` folds both negations away.
        let cond_str = if shape.negated {
            match cond.kind {
                ExprKind::Unary(UnOp::Not, inner) => Sugg::hir_with_context(cx, inner, ctxt, "..", &mut app)
                    .maybe_paren()
                    .to_string(),
                _ => Sugg::UnOp(UnOp::Not, Box::new(cond_sugg))
                    .maybe_paren()
                    .to_string(),
            }
        } else {
            cond_sugg.maybe_paren().to_string()
        };

        // `bool::ok_or` returns `Result<(), E>`: it carries the error but not
        // the `Ok` value. A `Result<(), E>` if/else therefore ends at
        // `ok_or`/`ok_or_else`; a value-carrying `Ok(v)` has to travel through
        // `Option` first (`then_some(v).ok_or(e)`).
        let unit_value = matches!(shape.value.kind, ExprKind::Tup([]));

        // `then_some`/`ok_or` evaluate their payload eagerly, so use them only
        // when the payload is cheap; otherwise the lazy `then`/`ok_or_else`
        // keeps the old evaluation timing.
        let head_name = if shape.result && unit_value {
            ""
        } else if switch_to_eager_eval(cx, shape.value) {
            "then_some"
        } else {
            "then"
        };
        let head = match head_name {
            "then_some" => Some(format!(
                "then_some({})",
                snippet_with_applicability(cx, shape.value.span, "..", &mut app)
            )),
            "then" => Some(format!(
                "then(|| {})",
                snippet_with_applicability(cx, shape.value.span, "..", &mut app)
            )),
            _ => None,
        };

        let (tail, err_name) = match shape.err {
            Some(err) => {
                let err_snip = snippet_with_applicability(cx, err.span, "..", &mut app);
                if switch_to_eager_eval(cx, err) {
                    (Some(format!("ok_or({err_snip})")), "ok_or")
                } else {
                    (Some(format!("ok_or_else(|| {err_snip})")), "ok_or_else")
                }
            }
            None => (None, ""),
        };

        let suggestion = match (head, tail) {
            (Some(head), Some(tail)) => format!("{cond_str}.{head}.{tail}"),
            (Some(head), None) => format!("{cond_str}.{head}"),
            (None, Some(tail)) => format!("{cond_str}.{tail}"),
            // Unreachable: a unit `Ok` payload only appears in the `Result`
            // shape, which always carries its error.
            (None, None) => return,
        };

        let msg = match (shape.result, head_name.is_empty()) {
            (true, false) => format!(
                "bool `if`/`else` wraps a value in `Result`; use `bool::{head_name}(..).{err_name}(..)`"
            ),
            (true, true) => {
                format!("bool `if`/`else` wraps a value in `Result`; use `bool::{err_name}(..)`")
            }
            (false, _) => {
                format!("bool `if`/`else` wraps a value in `Option`; use `bool::{head_name}(..)`")
            }
        };

        span_lint_and_sugg(
            cx,
            BOOL_IF_ELSE_TO_COMBINATOR,
            expr.span,
            msg,
            "replace with",
            suggestion,
            app,
        );
    }
}

impl Default for BoolIfElseToCombinator {
    fn default() -> Self {
        BoolIfElseToCombinator
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}