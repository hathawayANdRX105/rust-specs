#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use dioxus_lint_scope::file_allowed;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{BytePos, SourceFile, Span};

dylint_linting::declare_late_lint! {
    /// A `#[component]` function body must contain at most one content `rsx!`.
    ///
    /// The returned rsx is the component's view. A second content `rsx!` in
    /// the same body means the function is assembling multiple views inline;
    /// extract a child component or pass the fragment as a slot prop.
    ///
    /// ### Why is this bad?
    ///
    /// One component, one view: a single rsx keeps the render tree scannable
    /// and lets the component be tested/rendered as a unit. Multiple inline
    /// rsx calls hide the structure and make the diff boundaries unclear.
    ///
    /// ### Known problems
    ///
    /// Source-text based; files that fail to parse are skipped. Only content
    /// `rsx!` calls count: a direct statement/tail expression, a direct
    /// `let x = rsx! { .. };`, or a direct `return rsx! { .. };`. Fragments
    /// nested in `if`/`match` branches, loops, closures (`map`, `then`), prop
    /// values, or any other expression are not counted — hoisting a
    /// conditional into `let node = if .. { rsx!{} } else { rsx!{} };` does
    /// not trigger this lint.
    ///
    /// ### Example
    ///
    /// ```rust
    /// #[component]
    /// fn Card() -> Element {
    ///     let header = rsx! { h1 { "title" } };
    ///     rsx! { div { {header} } }
    /// }
    /// ```
    ///
    /// Use instead: extract `Header` as a child component.
    pub DIOXUS_RSX_ONE_PER_COMPONENT,
    Warn,
    "component function contains more than one rsx!; extract a child component"
}

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
    let mut out = Vec::new();
    walk_items(&file.items, &mut out);
    out
}

fn walk_items(items: &[syn::Item], out: &mut Vec<Finding>) {
    for item in items {
        match item {
            syn::Item::Fn(func) => check_fn(&func.attrs, &func.sig.ident, &func.block, out),
            syn::Item::Impl(imp) => {
                for inner in &imp.items {
                    if let syn::ImplItem::Fn(func) = inner {
                        check_fn(&func.attrs, &func.sig.ident, &func.block, out);
                    }
                }
            }
            syn::Item::Mod(module) => {
                if let Some((_, items)) = &module.content {
                    walk_items(items, out);
                }
            }
            _ => {}
        }
    }
}

fn check_fn(
    attrs: &[syn::Attribute],
    ident: &syn::Ident,
    block: &syn::Block,
    out: &mut Vec<Finding>,
) {
    let is_component = attrs.iter().any(|attr| {
        attr.path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "component")
    });
    if !is_component {
        return;
    }
    let count = count_rsx(block);
    if count > 1 {
        let name = ident.to_string();
        out.push(finding(
            ident.span(),
            name.len(),
            format!("组件函数 {name} 含 {count} 个 rsx!，应为 1 个（R1.3）"),
        ));
    }
}

/// Count content `rsx!` calls only.
///
/// Content calls are the ones that make up the component's view:
/// - a direct statement or tail expression: `rsx! { .. }` / `rsx! { .. };`
/// - a direct `let` initializer: `let header = rsx! { .. };`
/// - a direct `return`: `return rsx! { .. };`
///
/// Fragments nested in `if`/`match` branches, loops, closures (`map`,
/// `then`), prop values, or any other expression are not content views and
/// are skipped. This keeps `let node = if c { rsx!{} } else { rsx!{} };`
/// compatible with the one-content-rsx rule.
fn count_rsx(block: &syn::Block) -> usize {
    block
        .stmts
        .iter()
        .filter(|stmt| stmt_is_content_rsx(stmt))
        .count()
}

fn stmt_is_content_rsx(stmt: &syn::Stmt) -> bool {
    match stmt {
        syn::Stmt::Expr(expr, _) => match expr {
            syn::Expr::Macro(mac) => is_rsx_macro(&mac.mac),
            syn::Expr::Return(ret) => ret.expr.as_deref().is_some_and(expr_is_direct_rsx),
            _ => false,
        },
        // A bare `rsx! { .. }` statement is parsed as `Stmt::Macro`, not
        // `Stmt::Expr(Expr::Macro)`.
        syn::Stmt::Macro(stmt_macro) => is_rsx_macro(&stmt_macro.mac),
        syn::Stmt::Local(local) => local
            .init
            .as_ref()
            .is_some_and(|init| init.diverge.is_none() && expr_is_direct_rsx(&init.expr)),
        _ => false,
    }
}

fn expr_is_direct_rsx(expr: &syn::Expr) -> bool {
    matches!(expr, syn::Expr::Macro(mac) if is_rsx_macro(&mac.mac))
}

fn is_rsx_macro(mac: &syn::Macro) -> bool {
    mac.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "rsx")
}

fn finding(span: proc_macro2::Span, len: usize, msg: String) -> Finding {
    let start = span.start();
    Finding {
        line: start.line,
        column: start.column,
        len,
        msg,
    }
}

fn is_ui_crate(name: &str) -> bool {
    name.starts_with("ui_") || name == "web"
}

fn is_fixture(text: &str) -> bool {
    text.lines()
        .take(5)
        .any(|line| line.contains("dioxus-lint-fixture"))
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

fn span_for(file: &SourceFile, text: &str, finding: &Finding) -> Span {
    let offset = byte_offset(text, finding.line, finding.column);
    let lo = file.start_pos + BytePos(offset as u32);
    let hi = lo + BytePos(finding.len.max(1) as u32);
    Span::with_root_ctxt(lo, hi)
}

impl<'tcx> LateLintPass<'tcx> for DioxusRsxOnePerComponent {
    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        let crate_name = cx.tcx.crate_name(LOCAL_CRATE);
        let scoped = is_ui_crate(crate_name.as_str());
        let source_map = cx.sess().source_map();
        for file in source_map.files().iter() {
            let Some(text) = file.src.as_deref().map(|text| text.as_str()) else {
                continue;
            };
            if !scoped && !is_fixture(text) {
                continue;
            }
            if !text.contains("rsx") {
                continue;
            }
            if !file_allowed(file) {
                continue;
            }
            for finding in scan_source(text) {
                let span = span_for(file, text, &finding);
                span_lint(cx, DIOXUS_RSX_ONE_PER_COMPONENT, span, finding.msg);
            }
        }
    }
}

impl Default for DioxusRsxOnePerComponent {
    fn default() -> Self {
        DioxusRsxOnePerComponent
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
    fn allows_one_rsx() {
        let source = r#"
#[component]
fn Card() -> Element {
    rsx! { div { "one" } }
}
"#;
        assert!(scan_source(source).is_empty());
    }

    #[test]
    fn flags_two_rsx() {
        let source = r#"
#[component]
fn Card() -> Element {
    let header = rsx! { h1 { "title" } };
    rsx! { div { {header} } }
}
"#;
        assert!(!scan_source(source).is_empty());
    }

    #[test]
    fn ignores_conditional_fragment() {
        let source = r#"
#[component]
fn Card(c: bool) -> Element {
    let node = if c { rsx! { div { "a" } } } else { rsx! { div { "b" } } };
    rsx! { div { {node} } }
}
"#;
        assert!(scan_source(source).is_empty());
    }

    #[test]
    fn ignores_closure_fragment() {
        let source = r#"
#[component]
fn Card(items: Vec<u8>) -> Element {
    let nodes = items.iter().map(|x| rsx! { div { {x} } });
    rsx! { div { {nodes} } }
}
"#;
        assert!(scan_source(source).is_empty());
    }

    #[test]
    fn flags_direct_let_fragment() {
        let source = r#"
#[component]
fn Card() -> Element {
    let header = rsx! { h1 { "title" } };
    rsx! { div { {header} } }
}
"#;
        assert!(!scan_source(source).is_empty());
    }

    #[test]
    fn flags_two_top_level_rsx() {
        let source = r#"
#[component]
fn Card() -> Element {
    rsx! { div { "a" } }
    rsx! { div { "b" } }
}
"#;
        assert!(!scan_source(source).is_empty());
    }

    #[test]
    fn ignores_non_component_functions() {
        let source = r#"
fn helper() -> Element {
    let a = rsx! { div { "a" } };
    rsx! { div { {a} } }
}
"#;
        assert!(scan_source(source).is_empty());
    }
}
