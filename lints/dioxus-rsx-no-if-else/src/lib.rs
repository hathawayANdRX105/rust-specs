#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;

use clippy_utils::diagnostics::span_lint;
use dioxus_lint_scope::file_allowed;
use dioxus_rsx::BodyNode;
use dioxus_rsx_scan::{Finding, finding, parse_rsx_calls, span_for, walk_nodes};
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_lint::{LateContext, LateLintPass, LintContext};

dylint_linting::declare_late_lint! {
    /// Bans `if` / `if-else` inside `rsx!` bodies.
    ///
    /// `if` as an rsx child hides a rendering branch from the component
    /// boundary. Move the branch above the `rsx!` call (returning one of two
    /// `rsx!` values), or extract a child component / slot. `for` is allowed;
    /// `match` / `while` / `loop` are handled by
    /// `dioxus-rsx-avoid-control-flow` at WARN.
    ///
    /// ### Why is this bad?
    ///
    /// rsx describes a view. Rust control flow embedded in it makes the view
    /// hard to scan and works against the "one rsx per component" discipline.
    ///
    /// ### Known problems
    ///
    /// The lint scans source text with `syn` + `dioxus-rsx`; files that fail
    /// to parse are skipped. It only sees rsx child nodes, so `if` inside a
    /// closure or an attribute expression (ordinary Rust) is not reported.
    ///
    /// ### Example
    ///
    /// ```rust
    /// rsx! {
    ///     div {
    ///         if show { span { "visible" } }
    ///     }
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// let body = if show { rsx! { span { "visible" } } } else { rsx! {} };
    /// rsx! { div { {body} } }
    /// ```
    pub DIOXUS_RSX_NO_IF_ELSE,
    Warn,
    "if/if-else inside rsx! is not acceptable"
}

const IF_MSG: &str = "rsx 中禁止 if/if-else（FAIL 级）：把条件渲染移到 rsx 外，或抽成组件/插槽";

fn scan_source(text: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    for call in parse_rsx_calls(text) {
        walk_nodes(&call.body, &mut |node| {
            if let BodyNode::IfChain(chain) = node {
                out.push(finding(chain.if_token.span, IF_MSG.to_string()));
            }
        });
    }
    out
}

fn is_ui_crate(name: &str) -> bool {
    name.starts_with("ui_") || name == "web"
}

fn is_fixture(text: &str) -> bool {
    text.lines()
        .take(5)
        .any(|line| line.contains("dioxus-lint-fixture"))
}

impl<'tcx> LateLintPass<'tcx> for DioxusRsxNoIfElse {
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
                span_lint(cx, DIOXUS_RSX_NO_IF_ELSE, span, finding.msg);
            }
        }
    }
}

impl Default for DioxusRsxNoIfElse {
    fn default() -> Self {
        DioxusRsxNoIfElse
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
    fn flags_if_in_rsx() {
        let source = r#"
fn view() {
    rsx! {
        div {
            if show { span { "visible" } }
        }
    }
}
"#;
        let found = findings(source);
        assert!(
            found.iter().any(|f| f.msg.contains("if/if-else")),
            "{found:?}"
        );
    }

    #[test]
    fn flags_if_else_chain_once() {
        let source = r#"
fn view() {
    rsx! {
        div {
            if show { span { "a" } } else { span { "b" } }
        }
    }
}
"#;
        let found = findings(source);
        assert_eq!(
            found
                .iter()
                .filter(|f| f.msg.contains("if/if-else"))
                .count(),
            1,
            "{found:?}"
        );
    }

    #[test]
    fn flags_nested_if_in_for_body() {
        let source = r#"
fn view() {
    rsx! {
        div {
            for item in items {
                if item.active { span { "on" } }
            }
        }
    }
}
"#;
        let found = findings(source);
        assert_eq!(found.len(), 1, "{found:?}");
    }

    #[test]
    fn for_is_allowed() {
        let source = r#"
fn view() {
    rsx! {
        div {
            for item in items {
                span { "{item}" }
            }
        }
    }
}
"#;
        assert!(findings(source).is_empty(), "{:?}", findings(source));
    }

    #[test]
    fn match_while_loop_are_not_flagged() {
        let source = r#"
fn view() {
    rsx! {
        div {
            { match state { _ => "idle" } }
            { while running { tick() } }
            { loop { break } }
        }
    }
}
"#;
        assert!(findings(source).is_empty(), "{:?}", findings(source));
    }

    #[test]
    fn closure_if_is_not_flagged() {
        let source = r#"
fn view() {
    rsx! {
        div {
            { items.iter().map(|x| if *x > 0 { 1 } else { 0 }).sum::<i32>() }
        }
    }
}
"#;
        assert!(findings(source).is_empty(), "{:?}", findings(source));
    }

    #[test]
    fn attribute_expression_is_not_flagged() {
        let source = r#"
fn view() {
    rsx! {
        div {
            onclick: move |_| if enabled { save() } else { cancel() },
            "click"
        }
    }
}
"#;
        assert!(findings(source).is_empty(), "{:?}", findings(source));
    }
}
