#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use proc_macro2::Span;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{BytePos, SourceFile};
use std::collections::HashSet;
use syn::spanned::Spanned;
use syn::visit::Visit;

dylint_linting::declare_late_lint! {
    /// Reports `ui_util::pick(..)` and `ui_util::pick_with(..)` calls outside
    /// an `rsx!` body.
    ///
    /// `pick` exists for one reason: `dioxus-rsx-no-if-else` bans `if` /
    /// `else` as an `rsx!` child, so a branch between two elements has to be
    /// a single expression. Outside `rsx!` there is no such restriction.
    ///
    /// ```rust
    /// // reported
    /// let class = pick(active, "row-active", "row");
    ///
    /// // preferred
    /// let class = if active { "row-active" } else { "row" };
    /// ```
    ///
    /// ### Why is this bad?
    ///
    /// A helper call hides a two-branch decision behind a name the reader has
    /// to look up, and it makes both branches eager. The plain `if` / `else`
    /// says what it does at the call site.
    ///
    /// ### Known problems
    ///
    /// Source-text based (`syn`), so there is no type information: files that
    /// fail to parse are skipped. The check is positional — a call counts as
    /// inside `rsx!` when it appears within an `rsx!` macro invocation.
    pub PICK_OUTSIDE_RSX,
    Warn,
    "pick outside rsx; use a plain if/else expression"
}

const MSG: &str = "pick 只能用于 rsx 内部的分支选择；rsx 之外用 if/else 表达式";

/// One finding produced from source text (line/column are 1-based line and
/// 0-based column, matching `proc_macro2`'s span-locations).
#[derive(Debug, PartialEq, Eq)]
struct Finding {
    line: usize,
    column: usize,
    len: usize,
    msg: String,
}

fn scan_source(text: &str) -> Vec<Finding> {
    let Ok(file) = syn::parse_file(text) else {
        return Vec::new();
    };
    let mut visitor = PickVisitor {
        out: Vec::new(),
        shadowed: shadowed_names(&file),
    };
    visitor.visit_file(&file);
    visitor.out
}

/// Names bound as function parameters or local variables, which shadow an
/// imported `pick` helper.
///
/// A bare `pick(..)` call is only the helper when nothing else in the file
/// binds that name: a parameter named `pick` is a different function.
fn shadowed_names(file: &syn::File) -> HashSet<String> {
    let mut collector = ShadowCollector {
        names: HashSet::new(),
    };
    collector.visit_file(file);
    collector.names
}

struct ShadowCollector {
    names: HashSet<String>,
}

impl<'ast> Visit<'ast> for ShadowCollector {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        for arg in &item.sig.inputs {
            self.collect_fn_arg(arg);
        }
        syn::visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        for arg in &item.sig.inputs {
            self.collect_fn_arg(arg);
        }
        syn::visit::visit_impl_item_fn(self, item);
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        self.collect_pattern(&local.pat);
        syn::visit::visit_local(self, local);
    }

    fn visit_expr_closure(&mut self, expr: &'ast syn::ExprClosure) {
        for arg in &expr.inputs {
            self.collect_pattern(arg);
        }
        syn::visit::visit_expr_closure(self, expr);
    }
}

impl ShadowCollector {
    fn collect_fn_arg(&mut self, arg: &syn::FnArg) {
        if let syn::FnArg::Typed(typed) = arg {
            self.collect_pattern(&typed.pat);
        }
    }

    fn collect_pattern(&mut self, pat: &syn::Pat) {
        let mut finder = BindingFinder {
            names: &mut self.names,
        };
        finder.visit_pat(pat);
    }
}

struct BindingFinder<'a> {
    names: &'a mut HashSet<String>,
}

impl<'ast> Visit<'ast> for BindingFinder<'_> {
    fn visit_pat_ident(&mut self, ident: &'ast syn::PatIdent) {
        self.names.insert(ident.ident.to_string());
        syn::visit::visit_pat_ident(self, ident);
    }
}

/// Walks the AST looking for `pick` / `pick_with` calls that are not part of
/// an `rsx!` macro invocation.
struct PickVisitor {
    out: Vec<Finding>,
    shadowed: HashSet<String>,
}

impl<'ast> Visit<'ast> for PickVisitor {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if is_rsx_macro(mac) {
            // Everything inside the rsx body is exempt.
            return;
        }
        syn::visit::visit_macro(self, mac);
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let Some((name, span)) = pick_call_name(&call.func)
            && !self.shadowed.contains(&name)
            && !call_has_rsx_branch(call)
        {
            self.out.push(finding(span, name.len(), MSG));
        }
        syn::visit::visit_expr_call(self, call);
    }
}

/// True when either branch argument builds an `rsx!` element.
///
/// A choice between two rsx branches stays `pick` wherever it is written:
/// `dioxus-rsx-no-if-else` bans `if` / `else` as an rsx child, so `pick` is
/// the only way to express it, and hoisting the result into a `let` is
/// sometimes forced (a `#[cfg]` attribute is not accepted in an rsx child
/// position). Only a choice between plain values has a better form.
fn call_has_rsx_branch(call: &syn::ExprCall) -> bool {
    let mut finder = RsxFinder { found: false };
    for arg in &call.args {
        finder.visit_expr(arg);
        if finder.found {
            return true;
        }
    }
    false
}

struct RsxFinder {
    found: bool,
}

impl<'ast> Visit<'ast> for RsxFinder {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if mac
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "rsx")
        {
            self.found = true;
        }
        syn::visit::visit_macro(self, mac);
    }
}

fn is_rsx_macro(mac: &syn::Macro) -> bool {
    mac.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "rsx")
}

/// Recognizes the call target as `pick` or `pick_with`.
///
/// Accepted shapes:
/// - `pick(..)` — imported from `ui_util::pick`
/// - `ui_util::pick(..)`
/// - `ui_util::pick::pick(..)`
///
/// Any other function whose last path segment is `pick` is left alone: this
/// check has no type information, so it cannot tell `ui_util::pick` from an
/// unrelated method named `pick`.
fn pick_call_name(func: &syn::Expr) -> Option<(String, Span)> {
    let syn::Expr::Path(path) = func else {
        return None;
    };
    let segments: Vec<String> = path
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    let last = segments.last()?;
    if last != "pick" && last != "pick_with" {
        return None;
    }
    // `foo::pick` with more than the two accepted prefixes is another API.
    let accepted = match segments.len() {
        1 => true,
        2 => segments[0] == "ui_util",
        3 => segments[0] == "ui_util" && segments[1] == "pick",
        _ => false,
    };
    if !accepted {
        return None;
    }
    Some((last.clone(), path.span()))
}

fn finding(span: Span, len: usize, msg: &str) -> Finding {
    let start = span.start();
    Finding {
        line: start.line,
        column: start.column,
        len,
        msg: msg.to_string(),
    }
}

fn byte_offset(text: &str, line: usize, column: usize) -> usize {
    let mut offset = 0;
    for (index, current) in text.split_inclusive('\n').enumerate() {
        if index + 1 == line {
            let line_len = current.trim_end_matches('\n').len();
            return offset + column.min(line_len);
        }
        offset += current.len();
    }
    offset
}

fn span_for(file: &SourceFile, text: &str, finding: &Finding) -> rustc_span::Span {
    let offset = byte_offset(text, finding.line, finding.column);
    let lo = file.start_pos + BytePos(offset as u32);
    let hi = lo + BytePos(finding.len.max(1) as u32);
    rustc_span::Span::with_root_ctxt(lo, hi)
}

impl<'tcx> LateLintPass<'tcx> for PickOutsideRsx {
    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        let source_map = cx.sess().source_map();
        for file in source_map.files().iter() {
            // Only scan files that belong to the crate being linted: dependency
            // sources are loaded for diagnostics but are not ours to police.
            if file.cnum != LOCAL_CRATE {
                continue;
            }
            let Some(text) = file.src.as_deref().map(|text| text.as_str()) else {
                continue;
            };
            if !text.contains("pick") {
                continue;
            }
            for finding in scan_source(text) {
                let span = span_for(file, text, &finding);
                span_lint(cx, PICK_OUTSIDE_RSX, span, finding.msg);
            }
        }
    }
}

impl Default for PickOutsideRsx {
    fn default() -> Self {
        PickOutsideRsx
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

#[cfg(test)]
mod tests {
    use super::{scan_source, shadowed_names, PickVisitor};

    fn findings(source: &str) -> Vec<String> {
        let file = syn::parse_file(source).unwrap();
        let mut visitor = PickVisitor {
            out: Vec::new(),
            shadowed: shadowed_names(&file),
        };
        syn::visit::Visit::visit_file(&mut visitor, &file);
        visitor.out.into_iter().map(|finding| finding.msg).collect()
    }

    #[test]
    fn reports_bare_pick_outside_rsx() {
        let found = findings("fn f() { let x = pick(a, 1, 2); }");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn reports_qualified_pick_outside_rsx() {
        let found = findings("fn f() { let x = ui_util::pick(a, 1, 2); }");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn reports_pick_with_outside_rsx() {
        let found = findings("fn f() { let x = pick_with(a, || 1, || 2); }");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn accepts_pick_inside_rsx() {
        let found = findings("fn f() { rsx! { div { { pick(a, rsx! { span {} }, rsx! {}) } } } }");
        assert!(found.is_empty());
    }

    #[test]
    fn accepts_pick_with_rsx_branches_outside_rsx() {
        // A choice between two elements cannot become an if/else: the rsx
        // lint bans conditional rendering inside rsx children.
        let found = findings("fn f() { let n = pick(a, rsx! { span {} }, rsx! {}); }");
        assert!(found.is_empty());
    }

    #[test]
    fn reports_pick_with_plain_branches_outside_rsx() {
        let found = findings("fn f() { let n = pick_with(a, || 1, || 2); }");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn accepts_unrelated_pick_method() {
        let found = findings("fn f() { let x = selector.pick(&refs); }");
        assert!(found.is_empty());
    }

    #[test]
    fn accepts_other_module_pick() {
        let found = findings("fn f() { let x = other::pick(a, 1, 2); }");
        assert!(found.is_empty());
    }

    #[test]
    fn accepts_parameter_named_pick() {
        // A parameter named `pick` shadows the imported helper.
        let found = findings("fn f(pick: fn(i32) -> i32) { let x = pick(1); }");
        assert!(found.is_empty());
    }

    #[test]
    fn accepts_local_named_pick() {
        let found = findings("fn f() { let pick = |x: i32| x; let y = pick(1); }");
        assert!(found.is_empty());
    }

    #[test]
    fn accepts_closure_parameter_named_pick() {
        let found = findings("fn f() { let g = |pick: fn(i32) -> i32| pick(1); }");
        assert!(found.is_empty());
    }
}
