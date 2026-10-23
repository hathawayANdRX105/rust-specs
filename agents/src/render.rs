//! Markdown rendering: profile + assembly → the AGENTS document.
//!
//! Layout:
//!
//! ```text
//! # <profile.title>            the single H1
//! <preamble text / tables>
//! ## 1. <chapter.title>        chapters are numbered H2 sections
//! <summary>
//! ### <item.title>             points with a title become H3
//! <item body>                  text / bullets / numbered / code / tables
//! ```

use crate::profile::{Assembly, Item, ProfileFile, Table};

/// Stamp line written by `push` (and ignored by drift checks).
pub fn stamp(profile_name: &str) -> String {
    format!(
        "<!-- managed by rust-specs agents; edit agents/profiles/{profile_name}.toml, not this file -->"
    )
}

/// Renders the full document for `profile` (no stamp; `push` adds it).
pub fn render(profile: &ProfileFile, assembly: &Assembly) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n", profile.profile.title));

    if let Some(preamble) = &profile.profile.preamble {
        if !preamble.text.trim().is_empty() {
            out.push('\n');
            out.push_str(preamble.text.trim_end());
            out.push('\n');
        }
        for table in &preamble.tables {
            out.push('\n');
            render_table(&mut out, table);
        }
    }

    for (index, chapter) in assembly.chapters.iter().enumerate() {
        out.push_str(&format!("\n## {}. {}\n", index + 1, chapter.title));
        if let Some(summary) = &chapter.summary {
            if !summary.trim().is_empty() {
                out.push('\n');
                out.push_str(summary.trim_end());
                out.push('\n');
            }
        }
        for item in &chapter.items {
            render_item(&mut out, item);
        }
    }
    out
}

/// Renders one point under its chapter.
fn render_item(out: &mut String, item: &Item) {
    if let Some(title) = &item.title {
        out.push_str(&format!("\n### {title}\n"));
    }
    if !item.text.trim().is_empty() {
        out.push('\n');
        out.push_str(item.text.trim_end());
        out.push('\n');
    }
    if !item.numbered.is_empty() {
        out.push('\n');
        for (i, line) in item.numbered.iter().enumerate() {
            let mut first = true;
            for part in line.split('\n') {
                if first {
                    out.push_str(&format!("{}. {part}\n", i + 1));
                    first = false;
                } else {
                    out.push_str(&format!("   {part}\n"));
                }
            }
        }
    }
    if !item.bullets.is_empty() {
        out.push('\n');
        for bullet in &item.bullets {
            let mut first = true;
            for part in bullet.split('\n') {
                if first {
                    out.push_str(&format!("- {part}\n"));
                    first = false;
                } else {
                    out.push_str(&format!("  {part}\n"));
                }
            }
        }
    }
    if let Some(code) = &item.code {
        out.push_str(&format!("\n```{}\n", code.lang.as_deref().unwrap_or("")));
        out.push_str(code.text.trim_end());
        out.push('\n');
        out.push_str("```\n");
    }
    for table in &item.tables {
        out.push('\n');
        render_table(out, table);
    }
}

/// Renders a markdown table with a padded header separator.
fn render_table(out: &mut String, table: &Table) {
    if let Some(caption) = &table.caption {
        out.push_str(caption.trim_end());
        out.push_str("\n\n");
    }
    out.push_str("| ");
    out.push_str(&table.headers.join(" | "));
    out.push_str(" |\n");
    out.push('|');
    for header in &table.headers {
        out.push_str(&format!(" {} |", "-".repeat(header.len().max(3))));
    }
    out.push('\n');
    for row in &table.rows {
        out.push_str("| ");
        out.push_str(&row.join(" | "));
        out.push_str(" |\n");
    }
}
