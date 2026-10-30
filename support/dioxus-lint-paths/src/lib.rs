//! Pure path predicates shared by the dioxus lint scope filter.
//!
//! Kept free of `rustc_private` so the behavior can be unit-tested by the
//! normal test harness; `dioxus-lint-scope` is the rustc-facing wrapper.

use std::path::Path;

/// Match a path against the `DIOXUS_LINT_FILES` diff list.
///
/// Entries are repo-relative paths; matching is suffix-based so an absolute
/// source-map path (`/repo/crates/ui-x/src/a.rs`) matches `crates/ui-x/src/a.rs`.
/// An empty list means "no diff filter" and accepts everything.
pub fn matches_file_list(path: &Path, list: &[String]) -> bool {
    if list.is_empty() {
        return true;
    }
    list.iter().any(|entry| path.ends_with(Path::new(entry)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str) -> String {
        path.to_owned()
    }

    #[test]
    fn empty_list_scans_everything() {
        assert!(matches_file_list(
            Path::new("/repo/crates/ui-x/src/a.rs"),
            &[]
        ));
    }

    #[test]
    fn suffix_matches_repo_relative_paths() {
        let path = Path::new("/repo/crates/ui-x/src/a.rs");
        assert!(matches_file_list(path, &[entry("crates/ui-x/src/a.rs")]));
        assert!(matches_file_list(path, &[entry("src/a.rs")]));
        assert!(!matches_file_list(path, &[entry("crates/ui-y/src/a.rs")]));
    }

    #[test]
    fn any_entry_matches() {
        let path = Path::new("/repo/crates/ui-x/src/a.rs");
        assert!(matches_file_list(
            path,
            &[entry("crates/ui-y/src/a.rs"), entry("crates/ui-x/src/a.rs")]
        ));
    }
}
