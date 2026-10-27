#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_attr_ir;
extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use dylint_linting::declare_late_lint;
use rustc_attr_ir::Attribute as HirAttribute;
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::symbol::sym;
use std::path::PathBuf;

declare_late_lint! {
    /// Test code must live in the `tests/` directory, not alongside source code in `src/`.
    ///
    /// This lint fires on `#[test]`, `#[cfg(test)]`, and `mod ..._tests;` declarations
    /// that appear inside a crate's `src/` tree (i.e. a directory that contains a
    /// `Cargo.toml` and a sibling `src/` directory).
    ///
    /// ### Why is this bad?
    ///
    /// Ferrite's rule is that all tests — unit, integration, and contract — live under
    /// `tests/`. Keeping test code in `src/` mixes concerns and makes the public API
    /// surface harder to audit.
    ///
    /// ### Example
    ///
    /// ```rust
    /// // in crates/foo/src/lib.rs  (BAD)
    /// #[cfg(test)]
    /// mod tests {
    ///     #[test]
    ///     fn it_works() {}
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// // in crates/foo/tests/unit.rs  (GOOD)
    /// #[test]
    /// fn it_works() {}
    /// ```
    pub TEST_PLACEMENT,
    Warn,
    "test code must live in tests/, not in src/"
}

/// Returns true when `span` points inside a crate's `src/` directory.
///
/// A directory is considered a crate root when it contains a `Cargo.toml`. The file
/// must be located under that crate root's `src/` subdirectory to be considered
/// in-crate source.
fn is_in_crate_src(cx: &LateContext<'_>, span: rustc_span::Span) -> bool {
    let source_map = cx.tcx.sess.source_map();
    let file_name = source_map.span_to_filename(span);
    let path_str = format!("{:?}", file_name);
    let path_buf = PathBuf::from(path_str.as_str());

    // Only Rust source files are relevant.
    if path_buf.extension().map_or(true, |ext| ext != "rs") {
        return false;
    }

    // Walk up the directory tree looking for a Cargo.toml. The first crate root
    // found wins; the file must be inside that root's `src/` subdirectory.
    let mut current = path_buf.parent();
    while let Some(dir) = current {
        if dir.join("Cargo.toml").exists() {
            let src_dir = dir.join("src");
            return path_buf.starts_with(&src_dir);
        }
        current = dir.parent();
    }

    false
}

/// `#[test]` attribute: path's last segment is `test`.
fn is_test_attr(attr: &HirAttribute) -> bool {
    attr.path().last().map_or(false, |sym| *sym == sym::test)
}

/// `#[cfg(test)]` attribute: path is `cfg` and the attribute debug string contains `test`.
fn is_cfg_test_attr(attr: &HirAttribute) -> bool {
    let is_cfg = attr.path().last().map_or(false, |sym| *sym == sym::cfg);
    if !is_cfg {
        return false;
    }
    let attr_str = format!("{:?}", attr);
    attr_str.contains("test")
}

impl<'tcx> LateLintPass<'tcx> for TestPlacement {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &Item<'tcx>) {
        // #[test] / #[cfg(test)] on any item
        for attr in cx.tcx.hir_attrs(item.hir_id()) {
            if is_test_attr(attr) || is_cfg_test_attr(attr) {
                if is_in_crate_src(cx, item.span) {
                    span_lint(
                        cx,
                        TEST_PLACEMENT,
                        item.span,
                        "test code must live in tests/, not in src/",
                    );
                }
                return; // one report per item
            }
        }

        // `mod foo_tests;` declaration — ItemKind::Mod(Ident, &Mod)
        if let ItemKind::Mod(ident, _mod) = item.kind {
            let name = ident.to_string();
            if name.ends_with("_tests") && is_in_crate_src(cx, item.span) {
                span_lint(
                    cx,
                    TEST_PLACEMENT,
                    item.span,
                    format!("`mod {}` must live in tests/, not in src/", name),
                );
            }
        }
    }
}

impl Default for TestPlacement {
    fn default() -> Self {
        TestPlacement
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
