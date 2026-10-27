//! Integration tests for the structure rule implementations.
//!
//! The crate is a binary-only package, so the tests include the source
//! modules directly via `#[path]`.

#[path = "../src/spec_rules.rs"]
mod spec_rules;
#[path = "../src/util.rs"]
mod util;

use std::collections::{HashMap, HashSet};

use spec_rules::{
    CssVarMirrorParams, CssVarMirrorRule, DepEdgeDeny, DepEdgeParams, DepEdgeRule, DepGraph,
};
use util::{Finding, Severity};

fn graph_with(edges: &[(&str, &str, &str, &str)]) -> DepGraph {
    let mut dir_of = HashMap::new();
    let mut g_edges: HashMap<String, HashSet<String>> = HashMap::new();
    for (src, src_dir, dst, dst_dir) in edges {
        dir_of.entry(src.to_string()).or_insert_with(|| src_dir.to_string());
        dir_of.entry(dst.to_string()).or_insert_with(|| dst_dir.to_string());
        g_edges.entry(src.to_string()).or_default().insert(dst.to_string());
        g_edges.entry(dst.to_string()).or_default();
    }
    DepGraph { dir_of, edges: g_edges }
}

fn has_rule(findings: &[Finding], rule: &str, severity: Severity) -> bool {
    findings
        .iter()
        .any(|f| f.rule == rule && f.severity == severity)
}

/// A harness crate depending on a coding-harness crate must be reported by
/// `dep-edge.debt` with Info severity (existing debt, never blocking).
#[test]
fn dep_edge_debt_reports_harness_to_coding_harness_as_info() {
    let graph = graph_with(&[
        ("harness-daemon", "crates/harness-daemon", "coding-harness-session", "crates/coding-harness-session"),
    ]);
    let params = DepEdgeParams {
        edges_deny: vec![DepEdgeDeny {
            from: "crates/harness-*".into(),
            to: "crates/coding-harness-*".into(),
            message: Some("harness 不得依赖 coding-harness".into()),
        }],
    };
    let rule = DepEdgeRule::new("dep-edge.debt".into(), &params, Severity::Info).expect("rule");
    let findings = rule.check(&graph);
    assert!(
        has_rule(&findings, "dep-edge.debt", Severity::Info),
        "expected an INFO finding, got {findings:?}"
    );
    assert!(
        !findings.iter().any(|f| f.severity == Severity::Fail),
        "debt must never be Fail: {findings:?}"
    );
}

/// Edges outside the deny globs must produce no findings.
#[test]
fn dep_edge_debt_ignores_unrelated_edges() {
    let graph = graph_with(&[
        ("ui-components", "crates/ui-components", "ui-pages", "crates/ui-pages"),
        ("web", "bin/web", "ui-pages", "crates/ui-pages"),
    ]);
    let params = DepEdgeParams {
        edges_deny: vec![DepEdgeDeny {
            from: "crates/harness-*".into(),
            to: "crates/coding-harness-*".into(),
            message: None,
        }],
    };
    let rule = DepEdgeRule::new("dep-edge.debt".into(), &params, Severity::Info).expect("rule");
    let findings = rule.check(&graph);
    assert!(findings.is_empty(), "expected no findings, got {findings:?}");
}

const THEME_CSS: &str = r#":root {
  --background: #000000;
  --foreground: #f7f8f8;
}

.dark,
[data-theme="dark"] {
  color-scheme: dark;
  --background: #000000;
  --foreground: #f7f8f8;
}
"#;

fn mirror_rule() -> CssVarMirrorRule {
    CssVarMirrorRule::new(
        "css-var-mirror.theme-dark".into(),
        &CssVarMirrorParams {
            paths_include: vec!["crates/ui-*/**/theme.css".into()],
            message: None,
        },
        Severity::Fail,
    )
    .expect("rule")
}

/// A `.dark` block mirroring every `:root` token must produce no findings.
#[test]
fn css_var_mirror_accepts_identical_token_sets() {
    let path = std::path::Path::new("crates/ui-styles/assets/theme.css");
    let findings = mirror_rule().check_file(path, THEME_CSS);
    assert!(findings.is_empty(), "expected no findings, got {findings:?}");
}

/// A `.dark` block missing a `:root` token must be reported as a mirror gap.
#[test]
fn css_var_mirror_reports_missing_dark_token() {
    let css = THEME_CSS.replace(
        "color-scheme: dark;\n  --background: #000000;\n  --foreground: #f7f8f8;",
        "color-scheme: dark;\n  --background: #000000;",
    );
    let path = std::path::Path::new("crates/ui-styles/assets/theme.css");
    let findings = mirror_rule().check_file(path, &css);
    assert!(
        has_rule(&findings, "css-var-mirror.theme-dark", Severity::Fail),
        "expected a FAIL finding for missing dark token, got {findings:?}"
    );
}

/// A `.dark` block carrying a token absent from `:root` must be reported.
#[test]
fn css_var_mirror_reports_extra_dark_token() {
    let css = THEME_CSS.replace(
        "color-scheme: dark;\n  --background: #000000;",
        "color-scheme: dark;\n  --background: #000000;\n  --extra-token: #123456;",
    );
    let path = std::path::Path::new("crates/ui-styles/assets/theme.css");
    let findings = mirror_rule().check_file(path, &css);
    assert!(
        has_rule(&findings, "css-var-mirror.theme-dark", Severity::Fail),
        "expected a FAIL finding for extra dark token, got {findings:?}"
    );
}