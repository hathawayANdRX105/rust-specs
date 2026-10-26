#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use proc_macro2::Span;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{BytePos, SourceFile};
use syn::spanned::Spanned;
use syn::visit::Visit;

dylint_linting::declare_late_lint! {
    /// Bans a redundant pair of no-arg closures used as the two branches of a
    /// `let` initializer.
    ///
    /// Agents sometimes rewrite a conditional value into a pair of closures,
    /// adding a layer that the call site does not need:
    ///
    /// ```rust
    /// let node = pick_with(cond, || rsx! { span { "a" } }, || rsx! { span { "b" } });
    /// ```
    ///
    /// or:
    ///
    /// ```rust
    /// let node = if cond { || rsx! { span { "a" } } } else { || rsx! { span { "b" } } };
    /// ```
    ///
    /// Both branches already produce the selected value, so the closures are
    /// an extra shell. Pass the values directly — which form depends on what
    /// the branches build:
    ///
    /// ```rust
    /// // rsx branches: pick is the only single-expression form allowed
    /// let node = pick(cond, rsx! { span { "a" } }, rsx! { span { "b" } });
    ///
    /// // plain values: a plain if/else, not pick (see pick-outside-rsx)
    /// let n = if cond { 1 } else { 2 };
    /// ```
    ///
    /// ### Why is this bad?
    ///
    /// The extra closure layer hides the actual branch values: the reader has
    /// to open two closures to see what is selected. Dropping the layer keeps
    /// the choice visible, and for plain values it also keeps the untaken
    /// branch unevaluated.
    ///
    /// ### Known problems
    ///
    /// Source-text based (`syn`), so there is no type information: files that
    /// fail to parse are skipped. Only `let` initializers are inspected, and a
    /// `pick_with` is reported when it has exactly three arguments whose two
    /// branch arguments are no-arg closures.
    pub REDUNDANT_BRANCH_CLOSURES,
    Warn,
    "redundant no-arg closures on both branches; drop the closure layer"
}

const MSG: &str = "两个分支都是无参闭包且返回同一类型，闭包是多余的：rsx 分支用 pick(cond, a, b)，普通值用 if/else";

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
    let mut visitor = BranchVisitor { out: Vec::new() };
    visitor.visit_file(&file);
    visitor.out
}

struct BranchVisitor {
    out: Vec<Finding>,
}

impl<'ast> Visit<'ast> for BranchVisitor {
    fn visit_local(&mut self, local: &'ast syn::Local) {
        if let Some(init) = &local.init {
            match &*init.expr {
                syn::Expr::Call(call) if is_pick_with_branch_closures(call) => {
                    self.out
                        .push(finding(call.func.as_ref().span(), "pick_with".len(), MSG));
                }
                syn::Expr::If(if_expr) if is_if_with_closure_branches(if_expr) => {
                    self.out.push(finding(if_expr.if_token.span, 2, MSG));
                }
                _ => {}
            }
        }
        syn::visit::visit_local(self, local);
    }
}

/// `let x = pick_with(cond, || a, || b);`
fn is_pick_with_branch_closures(call: &syn::ExprCall) -> bool {
    let syn::Expr::Path(func) = &*call.func else {
        return false;
    };
    func.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "pick_with")
        && call.args.len() == 3
        && call.args.iter().skip(1).all(is_no_arg_closure)
}

/// `let x = if c { || a } else { || b };`
fn is_if_with_closure_branches(expr: &syn::ExprIf) -> bool {
    let Some((_, else_branch)) = &expr.else_branch else {
        return false;
    };
    let syn::Expr::Block(else_block) = &**else_branch else {
        return false;
    };
    block_tail_is_no_arg_closure(&expr.then_branch)
        && block_tail_is_no_arg_closure(&else_block.block)
}

fn block_tail_is_no_arg_closure(block: &syn::Block) -> bool {
    matches!(
        block.stmts.last(),
        Some(syn::Stmt::Expr(expr, None)) if is_no_arg_closure(expr)
    )
}

fn is_no_arg_closure(expr: &syn::Expr) -> bool {
    matches!(
        expr,
        syn::Expr::Closure(closure)
            if closure.inputs.is_empty() && closure.asyncness.is_none()
    )
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

impl<'tcx> LateLintPass<'tcx> for RedundantBranchClosures {
    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        let source_map = cx.sess().source_map();
        for file in source_map.files().iter() {
            // Dependencies are loaded for diagnostics; only the crate being
            // linted is our business.
            if file.cnum != LOCAL_CRATE {
                continue;
            }
            let Some(text) = file.src.as_deref().map(|text| text.as_str()) else {
                continue;
            };
            // Every candidate shape contains `||`; parsing unrelated files is
            // the expensive part of the source-map walk.
            if !text.contains("||") {
                continue;
            }
            for finding in scan_source(text) {
                let span = span_for(file, text, &finding);
                span_lint(cx, REDUNDANT_BRANCH_CLOSURES, span, finding.msg);
            }
        }
    }
}

impl Default for RedundantBranchClosures {
    fn default() -> Self {
        RedundantBranchClosures
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings(source: &str) -> Vec<Finding> {
        scan_source(source)
    }

    #[test]
    fn flags_pick_with_closures() {
        let source = r#"
fn view(c: bool) {
    let node = pick_with(c, || 1, || 2);
    let _ = node;
}
"#;
        assert_eq!(findings(source).len(), 1);
    }

    #[test]
    fn flags_if_else_closures() {
        let source = r#"
fn view(c: bool) {
    let node = if c { || 1 } else { || 2 };
    let _ = node;
}
"#;
        assert_eq!(findings(source).len(), 1);
    }

    #[test]
    fn flags_move_closures() {
        let source = r#"
fn view(c: bool) {
    let node = pick_with(c, move || 1, move || 2);
    let _ = node;
}
"#;
        assert_eq!(findings(source).len(), 1);
    }

    #[test]
    fn ignores_plain_let_if() {
        let source = r#"
fn view(c: bool) {
    let n = if c { 1 } else { 2 };
    let _ = n;
}
"#;
        assert!(findings(source).is_empty());
    }

    #[test]
    fn ignores_plain_pick() {
        let source = r#"
fn view(c: bool) {
    let n = pick(c, 1, 2);
    let _ = n;
}
"#;
        assert!(findings(source).is_empty());
    }

    #[test]
    fn ignores_closures_with_arguments() {
        let source = r#"
fn view(c: bool) {
    let n = pick_with(c, |x| x + 1, |x| x + 2);
    let _ = n;
}
"#;
        assert!(findings(source).is_empty());
    }

    #[test]
    fn ignores_one_closure_branch() {
        let source = r#"
fn view(c: bool) {
    let n = if c { 1 } else { 2 };
    let _ = n;
    let f = pick_with(c, || 1, value);
    let _ = f;
}
"#;
        assert!(findings(source).is_empty());
    }
}
