#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_lint;
extern crate rustc_span;

use rustc_lint::{LateContext, LintContext};
use rustc_span::{FileName, SourceFile, Span};
use std::path::Path;
use std::sync::OnceLock;

/// Restrict the dioxus lints to a git-diff file set.
///
/// `DIOXUS_LINT_FILES` is a newline-separated list of repo-relative paths
/// (the output of `git diff --name-only`). When it is unset or empty every
/// file is scanned. Matching is suffix-based: a file passes when its path
/// ends with one of the listed entries.
pub fn should_scan(path: &Path) -> bool {
    match file_list() {
        Some(list) => matches(path, list),
        None => true,
    }
}

fn file_list() -> &'static Option<Vec<String>> {
    static LIST: OnceLock<Option<Vec<String>>> = OnceLock::new();
    LIST.get_or_init(|| {
        std::env::var("DIOXUS_LINT_FILES").ok().map(|raw| {
            raw.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect()
        })
    })
}

fn matches(path: &Path, list: &[String]) -> bool {
    if list.is_empty() {
        return true;
    }
    list.iter().any(|entry| path.ends_with(Path::new(entry)))
}

/// Filter a source-file scan loop (the `source_map().files()` style lints).
pub fn file_allowed(file: &SourceFile) -> bool {
    match &file.name {
        FileName::Real(name) => name.local_path().map(should_scan).unwrap_or(true),
        // Virtual files (macro expansions, anon sources) are never filtered:
        // a false negative here would silently skip a real finding.
        _ => true,
    }
}

/// Filter a HIR finding by the span's source file.
pub fn span_allowed<'tcx>(cx: &LateContext<'tcx>, span: Span) -> bool {
    let loc = cx.sess().source_map().lookup_char_pos(span.lo());
    match &loc.file.name {
        FileName::Real(name) => name.local_path().map(should_scan).unwrap_or(true),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn entry(path: &str) -> String {
        path.to_owned()
    }

    #[test]
    fn empty_list_scans_everything() {
        assert!(matches(Path::new("/repo/crates/ui-x/src/a.rs"), &[]));
    }

    #[test]
    fn suffix_matches_repo_relative_paths() {
        let path = Path::new("/repo/crates/ui-x/src/a.rs");
        assert!(matches(path, &[entry("crates/ui-x/src/a.rs")]));
        assert!(matches(path, &[entry("src/a.rs")]));
        assert!(!matches(path, &[entry("crates/ui-y/src/a.rs")]));
    }

    #[test]
    fn any_entry_matches() {
        let path = Path::new("/repo/crates/ui-x/src/a.rs");
        assert!(matches(
            path,
            &[entry("crates/ui-y/src/a.rs"), entry("crates/ui-x/src/a.rs")]
        ));
    }
}
