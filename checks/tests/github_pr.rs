//! Integration tests for the GitHub PR checks.
//!
//! The crate is a binary-only package, so the tests include the source
//! modules directly via `#[path]`.

#[path = "../src/github_pr.rs"]
mod github_pr;
#[path = "../src/util.rs"]
mod util;

use github_pr::*;
use util::{Finding, Severity};

fn has_rule(findings: &[Finding], rule: &str, severity: Severity) -> bool {
    findings
        .iter()
        .any(|f| f.rule == rule && f.severity == severity)
}

/// A fully valid PR body: all required headings, Chinese What prose, and at
/// least two `- [ ]` checkboxes in Construction plan and Checklist.
fn valid_pr_body() -> String {
    r#"## What

这个 PR 修复了解析器在遇到空输入时的崩溃。

## Why

空输入会导致进程崩溃，影响稳定性。

## Construction plan

- [ ] 添加空输入检查
- [ ] 补充回归测试

## Delivery record

- Delivered: 已实现修复
- Verification: cargo test 通过

## How to test

1. 运行 `cargo test`。

## Checklist

- [ ] 已关联主 Issue
- [ ] 已运行测试
"#
    .to_string()
}

#[test]
fn english_conventional_title_passes() {
    let findings = check_title("fix: resolve parser panic on empty input");
    assert!(
        !has_rule(&findings, "PR-01", Severity::Fail),
        "PR-01 should not fire for an English title: {findings:?}"
    );
    assert!(
        !has_rule(&findings, "PR-02", Severity::Warn),
        "PR-02 should not fire for a conventional title: {findings:?}"
    );
    assert!(
        !has_rule(&findings, "PR-16", Severity::Fail),
        "PR-16 should not fire without full-width brackets: {findings:?}"
    );
}

#[test]
fn chinese_title_fails_pr01() {
    let findings = check_title("修复解析器崩溃");
    assert!(
        has_rule(&findings, "PR-01", Severity::Fail),
        "PR-01 should fail for a CJK title: {findings:?}"
    );
}

#[test]
fn non_conventional_title_warns_pr02() {
    let findings = check_title("random title without prefix");
    assert!(
        has_rule(&findings, "PR-02", Severity::Warn),
        "PR-02 should warn for a non-conventional title: {findings:?}"
    );
}

#[test]
fn conventional_prefixes_are_accepted() {
    for title in [
        "feat: add widget",
        "fix(core): handle empty input",
        "docs!: rewrite readme",
    ] {
        let findings = check_title(title);
        assert!(
            !has_rule(&findings, "PR-02", Severity::Warn),
            "PR-02 should not fire for {title:?}: {findings:?}"
        );
    }
}

#[test]
fn missing_required_headings_fail_pr03() {
    let body = "## What\n\n只有一个小节。\n";
    let findings = check_body(body);
    for required in REQUIRED_HEADINGS {
        if *required == "What" {
            continue;
        }
        assert!(
            has_rule(&findings, "PR-03", Severity::Fail),
            "PR-03 should fire for missing heading {required:?}: {findings:?}"
        );
    }
}

#[test]
fn what_without_chinese_fails_pr04() {
    let body = "## What\n\nThis section is written in English only.\n\n## Why\n\n中文背景。\n";
    let findings = check_body(body);
    assert!(
        has_rule(&findings, "PR-04", Severity::Fail),
        "PR-04 should fail when What has no Chinese: {findings:?}"
    );
}

#[test]
fn too_few_checkboxes_fail_pr07() {
    let body = r#"## Construction plan

- [ ] 只有一个勾选框

## Checklist

- [ ] 也只有一个
"#;
    let findings = check_body(body);
    assert!(
        has_rule(&findings, "PR-07", Severity::Fail),
        "PR-07 should fail when a done-when section has fewer than 2 checkboxes: {findings:?}"
    );
}

#[test]
fn checked_checkboxes_count_for_pr07() {
    // 合并前勾选后是 `- [x]`，PR-07 仍应统计数量（>0 且 >=2），不能因为全勾了就报 0 个框。
    let body = r#"## Construction plan

- [x] 第一项
- [x] 第二项

## Checklist

- [x] 检查 A
- [x] 检查 B
"#;
    let findings = check_body(body);
    assert!(
        !has_rule(&findings, "PR-07", Severity::Fail),
        "PR-07 should pass when sections have 2 checked checkboxes: {findings:?}"
    );
}

#[test]
fn full_width_bracket_in_title_fails_pr16() {
    let findings = check_title("fix: 支持（括号）标题");
    assert!(
        has_rule(&findings, "PR-16", Severity::Fail),
        "PR-16 should fail for a full-width bracket: {findings:?}"
    );
    // PR-16 takes priority: only one finding is reported for the title.
    assert_eq!(
        findings.len(),
        1,
        "expected a single PR-16 finding: {findings:?}"
    );
}

#[test]
fn multiple_fixes_warn_pr05() {
    let body = format!("## What\n\n中文。\n\nFixes #1\nFixes #2\n");
    let findings = check_body(&body);
    assert!(
        has_rule(&findings, "PR-05", Severity::Warn),
        "PR-05 should warn on multiple Fixes references: {findings:?}"
    );
}

#[test]
fn branch_prefix_checks() {
    let good = check_branch(Some("feat/xtask-checks"));
    assert!(
        !has_rule(&good, "PR-08", Severity::Warn),
        "feat/ prefix should pass: {good:?}"
    );
    let bad = check_branch(Some("random/branch"));
    assert!(
        has_rule(&bad, "PR-08", Severity::Warn),
        "random/ prefix should warn: {bad:?}"
    );
    let skipped = check_branch(None);
    assert!(
        skipped.is_empty(),
        "no branch data means no finding: {skipped:?}"
    );
}

#[test]
fn type_label_checks() {
    let with_type = check_labels(Some(&["bug".to_string(), "api".to_string()]));
    assert!(
        !has_rule(&with_type, "PR-06", Severity::Warn),
        "bug is a type label: {with_type:?}"
    );
    let without_type = check_labels(Some(&["api".to_string(), "ui".to_string()]));
    assert!(
        has_rule(&without_type, "PR-06", Severity::Warn),
        "no type label should warn: {without_type:?}"
    );
    let skipped = check_labels(None);
    assert!(
        skipped.is_empty(),
        "no label data means no finding: {skipped:?}"
    );
}

#[test]
fn parse_labels_splits_commas() {
    let labels = util::parse_labels(Some("bug, api,  ")).unwrap();
    assert_eq!(labels, vec!["bug".to_string(), "api".to_string()]);
    assert!(util::parse_labels(None).is_none());
    assert!(util::parse_labels(Some("  ")).unwrap().is_empty());
}

#[test]
fn fully_valid_pr_has_no_findings() {
    let findings = check_pr(&PrInput {
        title: "fix: resolve parser panic".to_string(),
        body: valid_pr_body(),
        labels: Some(vec!["bug".to_string()]),
        branch: Some("fix/parser-panic".to_string()),
    });
    assert!(
        findings.is_empty(),
        "a fully valid PR should have no findings: {findings:?}"
    );
}
