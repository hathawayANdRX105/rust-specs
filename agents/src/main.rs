//! `agents` — CLI over the [`agents`] library: build / push / check AGENTS.md
//! documents from TOML profiles.
//!
//! Usage:
//!   cargo agents list
//!   cargo agents build <profile> [--chapter id …] [--item ch.item …]
//!                     [--drop ch.item …] [--out <file>]
//!   cargo agents push  <profile> [--force] [selection flags]
//!   cargo agents check <profile> [selection flags]

use agents::profile::{ProfileFile, Selection};
use agents::render;
use clap::{Args, Parser, Subcommand};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Directory holding the profile TOML files. Overridable for tests.
fn profile_dir() -> PathBuf {
    match std::env::var("AGENTS_PROFILE_DIR") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("profiles"),
    }
}

#[derive(Parser)]
#[command(
    name = "agents",
    version,
    about = "Assemble per-project AGENTS.md documents from TOML profiles"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List profiles with their chapters and points
    List,
    /// Render a profile to stdout (or `--out <file>`)
    Build {
        /// Profile name (a file `profiles/<name>.toml`)
        profile: String,
        #[command(flatten)]
        select: SelectArgs,
        /// Write the rendered document to this file instead of stdout
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Write the generated document to the profile's target file
    Push {
        /// Profile name (a file `profiles/<name>.toml`)
        profile: String,
        /// Overwrite even when the target was hand-edited
        #[arg(long)]
        force: bool,
        #[command(flatten)]
        select: SelectArgs,
    },
    /// Exit 0 when the target file matches the generated document
    Check {
        /// Profile name (a file `profiles/<name>.toml`)
        profile: String,
        #[command(flatten)]
        select: SelectArgs,
    },
}

/// Free-assembly flags shared by build / push / check.
#[derive(Args, Default)]
struct SelectArgs {
    /// Keep only these chapters (comma-separated ids)
    #[arg(long, value_delimiter = ',')]
    chapter: Vec<String>,
    /// Keep only these points (comma-separated `chapter.item` keys)
    #[arg(long, value_delimiter = ',')]
    item: Vec<String>,
    /// Drop these points (comma-separated `chapter.item` keys)
    #[arg(long, value_delimiter = ',')]
    drop: Vec<String>,
}

impl SelectArgs {
    fn selection(&self) -> Selection {
        Selection {
            chapters: (!self.chapter.is_empty()).then(|| self.chapter.clone()),
            items: (!self.item.is_empty()).then(|| self.item.clone()),
            drop: self.drop.clone(),
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::List => list(),
        Command::Build {
            profile,
            select,
            out,
        } => {
            let document = render_document(&profile, &select.selection())?;
            match out {
                Some(path) => write_document(&path, &document, false, &profile),
                None => std::io::stdout()
                    .write_all(document.as_bytes())
                    .map_err(|e| e.to_string()),
            }
        }
        Command::Push {
            profile,
            force,
            select,
        } => {
            let file = load_profile(&profile)?;
            let document = render_document(&profile, &select.selection())?;
            let target = file.target_path()?;
            write_document(&target, &document, force, &profile)
        }
        Command::Check { profile, select } => {
            let file = load_profile(&profile)?;
            let document = render_document(&profile, &select.selection())?;
            let target = file.target_path()?;
            let on_disk = std::fs::read_to_string(&target)
                .map_err(|e| format!("failed to read {}: {e}", target.display()))?;
            if strip_stamp(&on_disk) == strip_stamp(&document) {
                println!("ok: {} is in sync", target.display());
                Ok(())
            } else {
                // Exit 1 = the check ran and found drift (2 is for errors).
                eprintln!(
                    "drift: {} differs from the generated document",
                    target.display()
                );
                std::process::exit(1);
            }
        }
    }
}

fn load_profile(name: &str) -> Result<ProfileFile, String> {
    let path = profile_dir().join(format!("{name}.toml"));
    ProfileFile::load(&path)
}

fn render_document(name: &str, selection: &Selection) -> Result<String, String> {
    let file = load_profile(name)?;
    let assembly = file.assemble(selection)?;
    Ok(render::render(&file, &assembly))
}

/// Writes `document` to `path`. Stamped files are generated artifacts;
/// hand-edited targets are only overwritten with `--force`.
fn write_document(path: &Path, document: &str, force: bool, profile_name: &str) -> Result<(), String> {
    let stamped = format!("{}\n{document}", render::stamp(profile_name));
    if let Ok(old) = std::fs::read_to_string(path) {
        if strip_stamp(&old) == strip_stamp(&stamped) {
            println!("ok: {} is already in sync", path.display());
            return Ok(());
        }
        if !force {
            eprintln!(
                "warning: {} differs from the generated document (hand-edited?)",
                path.display()
            );
            return Err("refusing to overwrite; pass --force to replace".to_string());
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, &stamped).map_err(|e| e.to_string())?;
    println!("wrote {}", path.display());
    Ok(())
}

/// Compares documents ignoring the managed-by stamp line.
fn strip_stamp(document: &str) -> String {
    document
        .lines()
        .filter(|line| !(line.starts_with("<!--") && line.contains("managed by")))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn list() -> Result<(), String> {
    let dir = profile_dir();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("failed to read {}: {e}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().map(|ext| ext == "toml").unwrap_or(false))
        .collect();
    entries.sort();
    if entries.is_empty() {
        println!("no profiles in {}", dir.display());
        return Ok(());
    }
    for path in entries {
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_string();
        let file = match ProfileFile::load(&path) {
            Ok(file) => file,
            Err(e) => {
                println!("{name}: INVALID ({e})");
                continue;
            }
        };
        println!(
            "{name}: {} chapters, {} points → {}",
            file.chapters.len(),
            file.all_item_keys().len(),
            file.target_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|e| format!("<no target: {e}>")),
        );
        for chapter in &file.chapters {
            let ids: Vec<String> = chapter
                .items
                .iter()
                .map(|item| format!("{}.{}", chapter.id, item.id))
                .collect();
            println!("  {:<12} {}  [{}]", chapter.id, chapter.title, ids.join(" "));
        }
    }
    Ok(())
}
