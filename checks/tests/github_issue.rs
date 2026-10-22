//! Integration tests for the GitHub issue checks.
//!
//! The crate is a binary-only package, so the tests include the source
//! modules directly via `#[path]`.

#[path = "../src/github_issue.rs"]
mod github_issue;
#[path = "../src/util.rs"]
mod util;

use github_issue::*;
use util::{Finding, Severity};

fn has_rule(findings: &[Finding], rule: &str, severity: Severity) -> bool {
    findings
        .iter()
        .any(|f| f.rule == rule && f.severity == severity)
}

/// A fully valid issue body: all required headings, Chinese prose, and a
/// `- [ ]` checkbox in Done when.
fn valid_issue_body() -> String {
    r#"## Goal

目标：为 issue 标题增加语言检查。

## Background

目前缺少标题语言校验，非中文标题会混入。

## Done when

- [ ] 实现标题语言检查
- [ ] 补充测试用例

## Suspected areas

src/github_issue.rs

## Out of scope

样式与格式调整。

## How to observe success

运行 `cargo test` 全部通过。
"#
    .to_string()
}

#[test]
fn chinese_title_passes_is05() {
    let findings = check_title("支持 issue 标题语言检查");
    assert!(
        !has_rule(&findings, "IS-05", Severity::Fail),
        "IS-05 should not fire for a Chinese title: {findings:?}"
    );
    assert!(
        !has_rule(&findings, "IS-16-title", Severity::Fail),
        "IS-16-title should not fire without full-width brackets: {findings:?}"
    );
}

#[test]
fn english_title_fails_is05() {
    let findings = check_title("Support issue title checks");
    assert!(
        has_rule(&findings, "IS-05", Severity::Fail),
        "IS-05 should fail for a non-Chinese title: {findings:?}"
    );
}

#[test]
fn missing_required_headings_fail_is01() {
    let body = "## Goal\n\n只有一个小节。\n";
    let findings = check_body(body);
    for required in REQUIRED_HEADINGS {
        if *required == "Goal" {
            continue;
        }
        assert!(
            has_rule(&findings, "IS-01", Severity::Fail),
            "IS-01 should fire for missing heading {required:?}: {findings:?}"
        );
    }
}

#[test]
fn done_when_without_checkbox_fails_is04() {
    let body = r#"## Done when

这里没有勾选框，只有描述文字。
"#;
    let findings = check_body(body);
    assert!(
        has_rule(&findings, "IS-04", Severity::Fail),
        "IS-04 should fail when Done when has no checkbox: {findings:?}"
    );
}

#[test]
fn forbidden_keyword_fails_is16() {
    for keyword in ["TODO", "TBD", "FIXME", "XXX"] {
        let body = format!("## Goal\n\n正文包含 {keyword} 待办。\n");
        let findings = check_body(&body);
        assert!(
            has_rule(&findings, "IS-16", Severity::Fail),
            "IS-16 should fail for keyword {keyword}: {findings:?}"
        );
    }
}

#[test]
fn labels_section_fails_is09() {
    let body = "## Goal\n\n中文目标。\n\n## Labels\n\nbug\n";
    let findings = check_body(body);
    assert!(
        has_rule(&findings, "IS-09", Severity::Fail),
        "IS-09 should fail when body contains a Labels section: {findings:?}"
    );
}

#[test]
fn garbled_content_fails_is16() {
    let literal_backslash_n = "## Goal\n\n正文包含字面量 \\n 换行。\n";
    let findings = check_body(literal_backslash_n);
    assert!(
        has_rule(&findings, "IS-16", Severity::Fail),
        "IS-16 should fail for a literal backslash-n: {findings:?}"
    );

    let replacement_char = "## Goal\n\n正文包含乱码\u{FFFD}字符。\n";
    let findings = check_body(replacement_char);
    assert!(
        has_rule(&findings, "IS-16", Severity::Fail),
        "IS-16 should fail for U+FFFD: {findings:?}"
    );
}

#[test]
fn full_width_bracket_in_title_fails_is16_title() {
    let findings = check_title("支持（全角括号）标题");
    assert!(
        has_rule(&findings, "IS-16-title", Severity::Fail),
        "IS-16-title should fail for a full-width bracket: {findings:?}"
    );
}

#[test]
fn body_without_chinese_fails_is07() {
    let body = "## Goal\n\nThis body is entirely in English.\n";
    let findings = check_body(body);
    assert!(
        has_rule(&findings, "IS-07", Severity::Fail),
        "IS-07 should fail when the body has no Chinese: {findings:?}"
    );
}

#[test]
fn type_label_checks() {
    let with_type = check_labels(Some(&["bug".to_string(), "api".to_string()]));
    assert!(
        !has_rule(&with_type, "IS-14", Severity::Warn),
        "bug is a type label: {with_type:?}"
    );
    let without_type = check_labels(Some(&["api".to_string()]));
    assert!(
        has_rule(&without_type, "IS-14", Severity::Warn),
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
    let labels = util::parse_labels(Some("enhancement, api,  ")).unwrap();
    assert_eq!(labels, vec!["enhancement".to_string(), "api".to_string()]);
    assert!(util::parse_labels(None).is_none());
    assert!(util::parse_labels(Some("  ")).unwrap().is_empty());
}

#[test]
fn fully_valid_issue_has_no_findings() {
    let findings = check_issue(&IssueInput {
        title: "支持 issue 标题语言检查".to_string(),
        body: valid_issue_body(),
        labels: Some(vec!["enhancement".to_string()]),
    });
    assert!(
        findings.is_empty(),
        "a fully valid issue should have no findings: {findings:?}"
    );
}
