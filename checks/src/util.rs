//! Shared helpers for the GitHub PR / issue checks.

use std::fmt;

/// Severity of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Hard failure: makes `cargo xtask check` exit with code 1.
    Fail,
    /// Soft warning: reported but does not fail the check.
    Warn,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Fail => write!(f, "FAIL"),
            Severity::Warn => write!(f, "WARN"),
        }
    }
}

/// A single check finding, printed as `[FAIL] PR-03: <message>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub rule: String,
    pub severity: Severity,
    pub message: String,
}

impl Finding {
    pub fn new(rule: impl Into<String>, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            rule: rule.into(),
            severity,
            message: message.into(),
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.severity, self.rule, self.message)
    }
}

/// Returns `true` when `s` contains any CJK ideograph.
pub fn contains_cjk(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(c as u32,
            0x3400..=0x4DBF       // CJK Extension A
            | 0x4E00..=0x9FFF     // CJK Unified Ideographs
            | 0xF900..=0xFAFF     // CJK Compatibility Ideographs
            | 0x20000..=0x2A6DF   // Extension B
            | 0x2A700..=0x2B73F   // Extension C
            | 0x2B740..=0x2B81F   // Extension D
            | 0x2B820..=0x2CEAF   // Extension E
            | 0x2CEB0..=0x2EBEF   // Extension F
        )
    })
}

/// Splits a markdown body into level-2 sections: `(heading, content)`.
///
/// A section starts at a line beginning with `## ` and ends at the next such
/// line. Content excludes the heading line itself. Lines before the first
/// heading are ignored.
pub fn extract_sections(body: &str) -> Vec<(String, String)> {
    let mut sections: Vec<(String, Vec<String>)> = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("## ") {
            sections.push((heading.trim().to_string(), Vec::new()));
        } else if let Some(last) = sections.last_mut() {
            last.1.push(line.to_string());
        }
    }
    sections
        .into_iter()
        .map(|(heading, lines)| (heading, lines.join("\n")))
        .collect()
}

/// Counts all checkboxes (`- [ ]` unchecked or `- [x]` checked) in `content`.
pub fn count_checkboxes(content: &str) -> usize {
    content
        .lines()
        .filter(|line| {
            let t = line.trim_start();
            t.starts_with("- [ ]") || t.starts_with("- [x]")
        })
        .count()
}

/// Parses a comma-separated label list into individual trimmed labels.
pub fn parse_labels(labels: Option<&str>) -> Option<Vec<String>> {
    labels.map(|s| {
        s.split(',')
            .map(|label| label.trim().to_string())
            .filter(|label| !label.is_empty())
            .collect()
    })
}

/// Accepted type labels (PR-06 / IS-14).
pub const TYPE_LABELS: &[&str] = &[
    "bug",
    "enhancement",
    "feature",
    "documentation",
    "chore",
    "refactor",
    "tests",
    "epic",
];

/// Returns `true` when at least one label is a type label.
pub fn has_type_label(labels: &[String]) -> bool {
    labels
        .iter()
        .any(|label| TYPE_LABELS.contains(&label.as_str()))
}
