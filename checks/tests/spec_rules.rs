//! Integration tests for the structure rule implementations.
//!
//! The crate is a binary-only package, so the tests include the source
//! modules directly via `#[path]`.

#[path = "../src/spec_rules.rs"]
mod spec_rules;
#[path = "../src/util.rs"]
mod util;

use std::collections::{HashMap, HashSet};

use spec_rules::{DepEdgeDeny, DepEdgeParams, DepEdgeRule, DepGraph};
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