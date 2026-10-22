//! GitHub issue checks.
//!
//! Every check is a pure function taking the relevant input and returning a
//! `Vec<Finding>`, so the CLI and the integration tests share the same code.

use crate::util::{
    Finding, Severity, contains_cjk, count_unchecked_checkboxes, extract_sections, has_type_label,
};

/// Required issue body sections (IS-01), order is not enforced.
pub const REQUIRED_HEADINGS: &[&str] = &[
    "Goal",
    "Background",
    "Done when",
    "Suspected areas",
    "Out of scope",
    "How to observe success",
];

/// Keywords forbidden in the issue body (IS-16).
pub const FORBIDDEN_KEYWORDS: &[&str] = &["TODO", "TBD", "FIXME", "XXX"];

/// Full-width brackets forbidden in issue titles (IS-16-title).
pub const FORBIDDEN_BRACKETS: &[char] = &[
    '（', '）', '「', '」', '【', '】', '『', '』', '《', '》', '〈', '〉',
];

/// Input data for the issue checks.
#[derive(Debug, Clone, Default)]
pub struct IssueInput {
    pub title: String,
    pub body: String,
    /// `None` when no label data is available (check is skipped).
    pub labels: Option<Vec<String>>,
}

/// Runs all issue checks and returns every finding.
pub fn check_issue(input: &IssueInput) -> Vec<Finding> {
    let mut findings = Vec::new();
    findings.extend(check_title(&input.title));
    findings.extend(check_body(&input.body));
    findings.extend(check_labels(input.labels.as_deref()));
    findings
}

/// Title checks: IS-16-title (full-width brackets), IS-05 (must be Chinese).
pub fn check_title(title: &str) -> Vec<Finding> {
    let mut findings = Vec::new();

    // A title containing a full-width bracket gets a single IS-16-title finding.
    if let Some(bracket) = title.chars().find(|c| FORBIDDEN_BRACKETS.contains(c)) {
        findings.push(Finding::new(
            "IS-16-title",
            Severity::Fail,
            format!("title contains full-width bracket \"{bracket}\""),
        ));
    }

    // IS-05: the title must contain Chinese (technical identifiers are allowed).
    if !contains_cjk(title) {
        findings.push(Finding::new(
            "IS-05",
            Severity::Fail,
            "title does not contain CJK characters; issue titles must be in Chinese",
        ));
    }

    findings
}

/// Body checks: IS-01 (required headings), IS-07 (Chinese prose), IS-04
/// (Done when checkbox), IS-16 (forbidden keywords / garbled content),
/// IS-09 (`## Labels` section).
pub fn check_body(body: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let sections = extract_sections(body);

    // IS-01: every required heading must be present.
    for required in REQUIRED_HEADINGS {
        if !sections.iter().any(|(heading, _)| heading == required) {
            findings.push(Finding::new(
                "IS-01",
                Severity::Fail,
                format!("missing required heading \"{required}\""),
            ));
        }
    }

    // IS-07: the body must contain Chinese prose.
    if !contains_cjk(body) {
        findings.push(Finding::new(
            "IS-07",
            Severity::Fail,
            "body does not contain CJK characters; issue body must be in Chinese",
        ));
    }

    // IS-04: the Done when section must contain at least one `- [ ]`.
    if let Some((_, content)) = sections.iter().find(|(heading, _)| heading == "Done when") {
        if count_unchecked_checkboxes(content) == 0 {
            findings.push(Finding::new(
                "IS-04",
                Severity::Fail,
                "\"Done when\" section has no checkboxes; add at least one \"- [ ]\"",
            ));
        }
    }

    // IS-16: forbidden keywords in the body.
    for keyword in FORBIDDEN_KEYWORDS {
        if body.contains(keyword) {
            findings.push(Finding::new(
                "IS-16",
                Severity::Fail,
                format!("body contains forbidden keyword \"{keyword}\""),
            ));
        }
    }

    // IS-16: garbled content — a literal backslash-n or the U+FFFD replacement char.
    if body.contains("\\n") {
        findings.push(Finding::new(
            "IS-16",
            Severity::Fail,
            "body contains a literal \"\\n\" sequence",
        ));
    }
    if body.contains('\u{FFFD}') {
        findings.push(Finding::new(
            "IS-16",
            Severity::Fail,
            "body contains the U+FFFD replacement character",
        ));
    }

    // IS-09: labels must be managed with `gh label`, not written in the body.
    if sections.iter().any(|(heading, _)| heading == "Labels") {
        findings.push(Finding::new(
            "IS-09",
            Severity::Fail,
            "body contains a \"## Labels\" section; use gh labels instead",
        ));
    }

    findings
}

/// Type label check (IS-14). Skipped when no label data is available.
pub fn check_labels(labels: Option<&[String]>) -> Vec<Finding> {
    let Some(labels) = labels else {
        return Vec::new();
    };
    if has_type_label(labels) {
        Vec::new()
    } else {
        vec![Finding::new(
            "IS-14",
            Severity::Warn,
            "no type label found (expected one of bug, enhancement, feature, documentation, chore, refactor, tests, epic)",
        )]
    }
}
