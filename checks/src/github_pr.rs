//! GitHub pull request checks.
//!
//! Every check is a pure function taking the relevant input and returning a
//! `Vec<Finding>`, so the CLI and the integration tests share the same code.

use crate::util::{
    Finding, Severity, contains_cjk, count_unchecked_checkboxes, extract_sections, has_type_label,
};

/// Required PR body sections (PR-03), order is not enforced.
pub const REQUIRED_HEADINGS: &[&str] = &[
    "What",
    "Why",
    "Construction plan",
    "Delivery record",
    "How to test",
    "Checklist",
];

/// Conventional commit types accepted for PR titles (PR-02).
pub const CONVENTIONAL_TYPES: &[&str] = &[
    "feat", "fix", "chore", "docs", "style", "refactor", "test", "ci", "build", "perf", "revert",
];

/// Full-width brackets forbidden in PR titles (PR-16).
pub const FORBIDDEN_BRACKETS: &[char] = &[
    '（', '）', '「', '」', '【', '】', '『', '』', '《', '》', '〈', '〉',
];

/// Allowed branch prefixes (PR-08).
pub const ALLOWED_BRANCH_PREFIXES: &[&str] = &[
    "feat/",
    "fix/",
    "chore/",
    "refactor/",
    "epic/",
    "main",
    "master",
    "release/",
];

/// Minimum number of `- [ ]` checkboxes in the done-when sections (PR-07).
pub const MIN_CHECKBOXES: usize = 2;

/// Input data for the PR checks.
#[derive(Debug, Clone, Default)]
pub struct PrInput {
    pub title: String,
    pub body: String,
    /// `None` when no label data is available (check is skipped).
    pub labels: Option<Vec<String>>,
    /// `None` when no branch data is available (check is skipped).
    pub branch: Option<String>,
}

/// Runs all PR checks and returns every finding.
pub fn check_pr(input: &PrInput) -> Vec<Finding> {
    let mut findings = Vec::new();
    findings.extend(check_title(&input.title));
    findings.extend(check_body(&input.body));
    findings.extend(check_branch(input.branch.as_deref()));
    findings.extend(check_labels(input.labels.as_deref()));
    findings
}

/// Title checks: PR-16 (full-width brackets), PR-01 (CJK), PR-02 (conventional).
pub fn check_title(title: &str) -> Vec<Finding> {
    // PR-16 takes priority: a title containing a full-width bracket reports a
    // single PR-16 finding and skips the other title checks.
    if let Some(bracket) = title.chars().find(|c| FORBIDDEN_BRACKETS.contains(c)) {
        return vec![Finding::new(
            "PR-16",
            Severity::Fail,
            format!("title contains full-width bracket \"{bracket}\""),
        )];
    }

    let mut findings = Vec::new();
    if contains_cjk(title) {
        findings.push(Finding::new(
            "PR-01",
            Severity::Fail,
            "title contains CJK characters; PR titles must be in English",
        ));
    }
    if !is_conventional_title(title) {
        findings.push(Finding::new(
            "PR-02",
            Severity::Warn,
            "title is not conventional commit",
        ));
    }
    findings
}

fn is_conventional_title(title: &str) -> bool {
    let title = title.trim();
    CONVENTIONAL_TYPES.iter().any(|ty| {
        title.starts_with(&format!("{ty}:"))
            || title.starts_with(&format!("{ty}("))
            || title.starts_with(&format!("{ty}!"))
    })
}

/// Body checks: PR-03 (required headings), PR-04 (Chinese What), PR-07
/// (checkbox count), PR-05 (multiple `Fixes #`).
pub fn check_body(body: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let sections = extract_sections(body);

    // PR-03: every required heading must be present.
    for required in REQUIRED_HEADINGS {
        if !sections.iter().any(|(heading, _)| heading == required) {
            findings.push(Finding::new(
                "PR-03",
                Severity::Fail,
                format!("missing required heading \"{required}\""),
            ));
        }
    }

    // PR-04: the What section must contain Chinese prose.
    if let Some((_, content)) = sections.iter().find(|(heading, _)| heading == "What") {
        if !contains_cjk(content) {
            findings.push(Finding::new(
                "PR-04",
                Severity::Fail,
                "\"What\" section does not contain Chinese prose",
            ));
        }
    }

    // PR-07: Construction plan and Checklist each need at least two `- [ ]`.
    for section in ["Construction plan", "Checklist"] {
        if let Some((_, content)) = sections.iter().find(|(heading, _)| heading == section) {
            let count = count_unchecked_checkboxes(content);
            if count < MIN_CHECKBOXES {
                findings.push(Finding::new(
                    "PR-07",
                    Severity::Fail,
                    format!(
                        "\"{section}\" section has {count} checkboxes; expected at least {MIN_CHECKBOXES}"
                    ),
                ));
            }
        }
    }

    // PR-05: more than one `Fixes #` reference is a warning.
    let fixes_count = body.matches("Fixes #").count();
    if fixes_count > 1 {
        findings.push(Finding::new(
            "PR-05",
            Severity::Warn,
            format!("body contains {fixes_count} \"Fixes #\" references; expected exactly one"),
        ));
    }

    findings
}

/// Branch prefix check (PR-08). Skipped when `branch` is `None`.
pub fn check_branch(branch: Option<&str>) -> Vec<Finding> {
    let Some(branch) = branch else {
        return Vec::new();
    };
    let branch = branch.trim();
    let allowed = ALLOWED_BRANCH_PREFIXES.iter().any(|prefix| {
        if prefix.ends_with('/') {
            branch.starts_with(prefix)
        } else {
            branch == *prefix
        }
    });
    if allowed {
        Vec::new()
    } else {
        vec![Finding::new(
            "PR-08",
            Severity::Warn,
            format!(
                "branch \"{branch}\" is not allowed; expected one of feat/, fix/, chore/, refactor/, epic/, main, master, release/"
            ),
        )]
    }
}

/// Type label check (PR-06). Skipped when no label data is available.
pub fn check_labels(labels: Option<&[String]>) -> Vec<Finding> {
    let Some(labels) = labels else {
        return Vec::new();
    };
    if has_type_label(labels) {
        Vec::new()
    } else {
        vec![Finding::new(
            "PR-06",
            Severity::Warn,
            "no type label found (expected one of bug, enhancement, feature, documentation, chore, refactor, tests, epic)",
        )]
    }
}
