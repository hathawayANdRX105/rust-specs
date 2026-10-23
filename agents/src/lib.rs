//! `agents` — per-project AGENTS.md assembly library.
//!
//! A profile (`agents/profiles/<name>.toml`) declares ordered chapters and
//! small, individually addressable points ("items"). Rendering is free
//! assembly: any chapter or point can be included, dropped, or reordered by
//! the profile itself (`include_chapters`, `exclude_items`) or by CLI flags
//! (`--chapter`, `--item`, `--drop`).

pub mod profile;
pub mod render;

/// Stamp line written by `push` (and ignored by drift checks).
pub fn stamp(profile_name: &str) -> String {
    render::stamp(profile_name)
}
