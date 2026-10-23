//! Profile schema and assembly: how a TOML file becomes ordered chapters.
//!
//! A profile file (`agents/profiles/<name>.toml`) has three parts:
//!
//! ```toml
//! [profile]            # identity + where the generated file lands
//! [profile.preamble]   # optional intro text + tables
//! [[chapter]]          # ordered chapters, each with [[chapter.item]] points
//! ```
//!
//! Assembly ("free assembly") selects a subset at two granularities:
//! whole chapters (`include_chapters`, `--chapter`) and single points
//! (`exclude_items`, `--item`, `--drop`), where a point is addressed as
//! `chapter.item` (a bare `item` id works when that id is unique in the
//! profile).

use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Errors surfaced to the CLI as `error: <msg>` (exit code 2).
pub type Result<T> = std::result::Result<T, String>;

/// A whole profile file.
#[derive(Debug, Deserialize)]
pub struct ProfileFile {
    pub profile: ProfileMeta,
    #[serde(default, rename = "chapter")]
    pub chapters: Vec<Chapter>,
}

/// `[profile]` — identity and target location.
#[derive(Debug, Deserialize)]
pub struct ProfileMeta {
    /// Profile name; the file must be `profiles/<name>.toml`.
    pub name: String,
    /// Document title, rendered as the single H1.
    pub title: String,
    /// Repository root: absolute, `~/…`, or relative to the projects base.
    #[serde(default)]
    pub repo: Option<String>,
    /// Target file relative to the repo root (default `AGENTS.md`).
    #[serde(default = "default_file")]
    pub file: String,
    /// Optional preamble rendered right under the title (`[profile.preamble]`).
    #[serde(default)]
    pub preamble: Option<Preamble>,
    /// Chapters to include, in profile order. Empty = every chapter.
    #[serde(default)]
    pub include_chapters: Vec<String>,
    /// Points to drop, addressed as `chapter.item` (bare `item` when unique).
    #[serde(default)]
    pub exclude_items: Vec<String>,
}

fn default_file() -> String {
    "AGENTS.md".to_string()
}

/// `[profile.preamble]` — intro rendered right under the title.
#[derive(Debug, Deserialize, Default)]
pub struct Preamble {
    #[serde(default)]
    pub text: String,
    #[serde(default, rename = "table")]
    pub tables: Vec<Table>,
}

/// A `[[chapter]]` — one top-level section (`##`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default, rename = "item")]
    pub items: Vec<Item>,
}

/// A `[[chapter.item]]` — one attention point inside a chapter.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Item {
    pub id: String,
    /// Rendered as an `###` heading; omit to inline the body under the chapter.
    #[serde(default)]
    pub title: Option<String>,
    /// Free markdown paragraph.
    #[serde(default)]
    pub text: String,
    /// Rendered as a `- ` bullet list.
    #[serde(default)]
    pub bullets: Vec<String>,
    /// Rendered as a `1. ` numbered list.
    #[serde(default)]
    pub numbered: Vec<String>,
    /// Rendered as a fenced code block.
    #[serde(default, rename = "code")]
    pub code: Option<CodeBlock>,
    #[serde(default, rename = "table")]
    pub tables: Vec<Table>,
}

/// A fenced code block (`[[chapter.item.code]]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CodeBlock {
    /// Fence language (default: plain fence).
    #[serde(default)]
    pub lang: Option<String>,
    pub text: String,
}

/// A markdown table (`[[…table]]`), reusable in the preamble and in items.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Table {
    #[serde(default)]
    pub caption: Option<String>,
    pub headers: Vec<String>,
    #[serde(default)]
    pub rows: Vec<Vec<String>>,
}

/// The selection applied on top of the profile defaults.
///
/// `None` means "as declared in the profile"; `Some(vec)` overrides it.
#[derive(Debug, Default, Clone)]
pub struct Selection {
    /// Chapter ids to keep (profile order preserved).
    pub chapters: Option<Vec<String>>,
    /// Point keys to keep, as `chapter.item` (bare `item` when unique).
    pub items: Option<Vec<String>>,
    /// Point keys to drop, as `chapter.item` (bare `item` when unique).
    pub drop: Vec<String>,
}

/// The render input: chapters with their points already filtered.
#[derive(Debug, PartialEq)]
pub struct Assembly {
    pub chapters: Vec<Chapter>,
}

impl ProfileFile {
    /// Loads and validates a profile from `path`.
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read profile {}: {e}", path.display()))?;
        let parsed: ProfileFile = toml::from_str(&raw)
            .map_err(|e| format!("failed to parse profile {}: {e}", path.display()))?;
        parsed.validate()?;
        Ok(parsed)
    }

    /// Structural validation: unknown references and malformed tables fail
    /// here, before anything is rendered.
    pub fn validate(&self) -> Result<()> {
        if self.chapters.is_empty() {
            return Err("profile has no [[chapter]]".to_string());
        }
        let mut chapter_ids: Vec<&str> = Vec::new();
        for ch in &self.chapters {
            if ch.title.trim().is_empty() {
                return Err(format!("chapter `{}` has an empty title", ch.id));
            }
            if chapter_ids.iter().any(|id| *id == ch.id) {
                return Err(format!("duplicate chapter id `{}`", ch.id));
            }
            chapter_ids.push(&ch.id);

            if ch.items.is_empty() {
                return Err(format!("chapter `{}` has no [[chapter.item]]", ch.id));
            }
            let mut item_ids: Vec<&str> = Vec::new();
            for item in &ch.items {
                if item_ids.iter().any(|id| *id == item.id) {
                    return Err(format!("duplicate item id `{}.{}`", ch.id, item.id));
                }
                item_ids.push(&item.id);
                for table in &item.tables {
                    check_table(table, &format!("{}.{}` tables", ch.id, item.id))?;
                }
            }
        }
        for key in &self.profile.include_chapters {
            if !chapter_ids.iter().any(|id| id == key) {
                return Err(format!(
                    "include_chapters references unknown chapter `{key}` (known: {})",
                    chapter_ids.join(", ")
                ));
            }
        }
        if let Some(preamble) = &self.profile.preamble {
            for table in &preamble.tables {
                check_table(table, "profile.preamble tables")?;
            }
        }
        Ok(())
    }

    /// `chapter.item` keys of every point in the profile.
    pub fn all_item_keys(&self) -> Vec<String> {
        self.chapters
            .iter()
            .flat_map(|c| c.items.iter().map(move |i| format!("{}.{}", c.id, i.id)))
            .collect()
    }

    /// Applies the profile defaults plus the CLI [`Selection`].
    pub fn assemble(&self, sel: &Selection) -> Result<Assembly> {
        let all_keys = self.all_item_keys();

        // Chapter filter: the CLI wins over the profile default.
        let chapter_filter: Option<Vec<String>> = match (&sel.chapters, &self.profile.include_chapters)
        {
            (Some(cli), _) if !cli.is_empty() => {
                for id in cli {
                    if !self.chapters.iter().any(|c| &c.id == id) {
                        return Err(format!(
                            "unknown chapter `{id}` (known: {})",
                            self.chapters
                                .iter()
                                .map(|c| c.id.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                }
                Some(cli.clone())
            }
            (None, declared) if !declared.is_empty() => Some(declared.clone()),
            _ => None,
        };
        let picked: Vec<&Chapter> = match &chapter_filter {
            // The profile's declared order wins over CLI order.
            Some(ids) => self
                .chapters
                .iter()
                .filter(|c| ids.iter().any(|id| &c.id == id))
                .collect(),
            None => self.chapters.iter().collect(),
        };

        // Point filters.
        let mut dropped: Vec<String> = self.profile.exclude_items.clone();
        dropped.extend(sel.drop.iter().cloned());
        for key in &dropped {
            ensure_known_key(&all_keys, key, "--drop")?;
        }
        let kept: Option<&Vec<String>> = match &sel.items {
            Some(keys) if !keys.is_empty() => {
                for key in keys {
                    ensure_known_key(&all_keys, key, "--item")?;
                }
                Some(keys)
            }
            _ => None,
        };

        let mut out: Vec<Chapter> = Vec::new();
        for ch in picked {
            let items: Vec<Item> = ch
                .items
                .iter()
                .filter(|item| {
                    let key = format!("{}.{}", ch.id, item.id);
                    if dropped.iter().any(|d| key_matches(&all_keys, &key, d)) {
                        return false;
                    }
                    match kept {
                        Some(keys) => keys.iter().any(|k| key_matches(&all_keys, &key, k)),
                        None => true,
                    }
                })
                .cloned()
                .collect();
            // A chapter whose points were all filtered out is skipped entirely.
            if !items.is_empty() {
                out.push(Chapter {
                    id: ch.id.clone(),
                    title: ch.title.clone(),
                    summary: ch.summary.clone(),
                    items,
                });
            }
        }
        if out.is_empty() {
            return Err("selection matched no chapters with items".to_string());
        }
        Ok(Assembly { chapters: out })
    }

    /// Absolute path of the generated file (repo root + `file`).
    pub fn target_path(&self) -> Result<PathBuf> {
        let repo = self.profile.repo.as_deref().ok_or_else(|| {
            "profile has no `profile.repo`; set it to the target repository".to_string()
        })?;
        Ok(resolve_repo(repo)?.join(&self.profile.file))
    }
}

/// Fails when `key` addresses no point of the profile.
fn ensure_known_key(all_keys: &[String], key: &str, flag: &str) -> Result<()> {
    if all_keys.iter().any(|k| key_matches(all_keys, k, key)) {
        return Ok(());
    }
    Err(format!("{flag} references unknown point `{key}` (known: {})", all_keys.join(", ")))
}

/// `key` (`chapter.item`) selects `candidate`; a bare `item` id selects it
/// only when that id is unambiguous across the whole profile.
fn key_matches(all_keys: &[String], candidate: &str, key: &str) -> bool {
    if candidate == key {
        return true;
    }
    let Some((_, item)) = candidate.split_once('.') else {
        return false;
    };
    if key != item {
        return false;
    }
    // Unique only when exactly one chapter has this item id.
    all_keys
        .iter()
        .filter(|k| k.split_once('.').map(|(_, i)| i) == Some(item))
        .count()
        == 1
}

/// True when every row has exactly `headers.len()` cells.
fn check_table(table: &Table, at: &str) -> Result<()> {
    if table.headers.is_empty() {
        return Err(format!("{at}: table has no headers"));
    }
    for (i, row) in table.rows.iter().enumerate() {
        if row.len() != table.headers.len() {
            return Err(format!(
                "{at}: table row {i} has {} cells, expected {}",
                row.len(),
                table.headers.len()
            ));
        }
    }
    Ok(())
}

/// Resolves a `repo` declaration: absolute as-is, `~/…` against `$HOME`,
/// otherwise relative to `AGENTS_PROJECTS_BASE` (default `~/projects`).
pub fn resolve_repo(repo: &str) -> Result<PathBuf> {
    let expanded = if let Some(rest) = repo.strip_prefix("~/") {
        let home = std::env::var("HOME").map_err(|_| "`HOME` is not set".to_string())?;
        PathBuf::from(home).join(rest)
    } else {
        PathBuf::from(repo)
    };
    if expanded.is_absolute() {
        return Ok(expanded);
    }
    let base = match std::env::var("AGENTS_PROJECTS_BASE") {
        Ok(v) if !v.is_empty() => {
            let p = PathBuf::from(v);
            if p.is_absolute() {
                p
            } else {
                let home = std::env::var("HOME").map_err(|_| "`HOME` is not set".to_string())?;
                PathBuf::from(home).join(p)
            }
        }
        _ => {
            let home = std::env::var("HOME").map_err(|_| "`HOME` is not set".to_string())?;
            PathBuf::from(home).join("projects")
        }
    };
    Ok(base.join(expanded))
}
