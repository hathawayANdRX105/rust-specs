//! Renderer and assembly tests over inline TOML profiles.

use agents::profile::{ProfileFile, Selection};
use agents::render;

/// Loads a profile from an inline TOML string.
fn load(toml_text: &str) -> ProfileFile {
    let file: ProfileFile = toml::from_str(toml_text).expect("profile parses");
    file.validate().expect("profile validates");
    file
}

const MINIMAL: &str = r#"
[profile]
name = "demo"
title = "demo conventions"
include_chapters = []

[[chapter]]
id = "alpha"
title = "Alpha chapter"
summary = "alpha summary"

[[chapter.item]]
id = "one"
title = "Item one"
text = "body one"
bullets = ["b1", "b2"]

[[chapter.item]]
id = "two"
numbered = ["n1", "n2"]

[[chapter]]
id = "beta"
title = "Beta chapter"

[[chapter.item]]
id = "one"
title = "Beta item"
code.lang = "bash"
code.text = "echo hi"
[[chapter.item.table]]
headers = ["H1", "H2"]
rows = [["a", "b"]]
"#;

#[test]
fn renders_chapters_items_and_blocks() {
    let file = load(MINIMAL);
    let assembly = file.assemble(&Selection::default()).unwrap();
    let doc = render::render(&file, &assembly);

    assert!(doc.starts_with("# demo conventions\n"));
    assert!(doc.contains("## 1. Alpha chapter\n"));
    assert!(doc.contains("## 2. Beta chapter\n"));
    assert!(doc.contains("alpha summary"));
    assert!(doc.contains("### Item one\n"));
    assert!(doc.contains("### Beta item\n"));
    // Bodies render in declared order: text, numbered, bullets, code, table.
    assert!(doc.contains("body one"));
    assert!(doc.contains("1. n1\n2. n2\n"));
    assert!(doc.contains("- b1\n- b2\n"));
    assert!(doc.contains("```bash\necho hi\n```"));
    assert!(doc.contains("| H1 | H2 |\n"));
    assert!(doc.contains("| a | b |\n"));
}

#[test]
fn chapter_selection_keeps_profile_order() {
    let file = load(MINIMAL);
    // Requesting beta first must still render alpha first (declared order).
    let sel = Selection {
        chapters: Some(vec!["beta".into(), "alpha".into()]),
        ..Default::default()
    };
    let assembly = file.assemble(&sel).unwrap();
    let doc = render::render(&file, &assembly);
    let alpha = doc.find("## 1. Alpha chapter").expect("alpha present");
    let beta = doc.find("## 2. Beta chapter").expect("beta present");
    assert!(alpha < beta, "profile order wins: alpha then beta");
}

#[test]
fn chapter_selection_rejects_unknown_ids() {
    let file = load(MINIMAL);
    let sel = Selection {
        chapters: Some(vec!["nope".into()]),
        ..Default::default()
    };
    let err = file.assemble(&sel).unwrap_err();
    assert!(err.contains("unknown chapter `nope`"), "got: {err}");
}

#[test]
fn item_selection_and_drop_work_with_qualified_keys() {
    let file = load(MINIMAL);
    // Both chapters have an item "one": a bare key is ambiguous → error.
    let bare = file.assemble(&Selection {
        items: Some(vec!["one".into()]),
        ..Default::default()
    });
    assert!(bare.is_err(), "bare duplicate item id must be rejected");

    let sel = Selection {
        items: Some(vec!["alpha.one".into()]),
        ..Default::default()
    };
    let assembly = file.assemble(&sel).unwrap();
    assert_eq!(assembly.chapters.len(), 1);
    assert_eq!(assembly.chapters[0].id, "alpha");
    assert_eq!(assembly.chapters[0].items.len(), 1);
    assert_eq!(assembly.chapters[0].items[0].id, "one");
}

#[test]
fn drop_list_removes_points_and_empty_chapters() {
    let file = load(MINIMAL);
    let sel = Selection {
        drop: vec!["beta.one".into()],
        ..Default::default()
    };
    let assembly = file.assemble(&sel).unwrap();
    // beta's only item is dropped → the chapter disappears entirely
    assert!(assembly.chapters.iter().all(|c| c.id != "beta"));

    let sel = Selection {
        drop: vec!["alpha.one".into()],
        ..Default::default()
    };
    let assembly = file.assemble(&sel).unwrap();
    let alpha = assembly.chapters.iter().find(|c| c.id == "alpha").unwrap();
    assert_eq!(alpha.items.len(), 1);
    assert_eq!(alpha.items[0].id, "two");
}

#[test]
fn include_chapters_default_filters_everything_else() {
    let text = MINIMAL.replace(
        "include_chapters = []",
        "include_chapters = [\"beta\"]",
    );
    let file = load(&text);
    let assembly = file.assemble(&Selection::default()).unwrap();
    assert_eq!(assembly.chapters.len(), 1);
    assert_eq!(assembly.chapters[0].id, "beta");
}

#[test]
fn validation_catches_duplicates_and_bad_tables() {
    let dup_chapter = MINIMAL.replace(
        "[[chapter]]\nid = \"beta\"",
        "[[chapter]]\nid = \"alpha\"",
    );
    let file: ProfileFile = toml::from_str(&dup_chapter).expect("parses");
    let err = file.validate().unwrap_err();
    assert!(
        err.contains("duplicate chapter id") || err.contains("duplicate item id"),
        "got: {err}"
    );

    let bad_table = MINIMAL.replace(
        "headers = [\"H1\", \"H2\"]\nrows = [[\"a\", \"b\"]]",
        "headers = [\"H1\", \"H2\"]\nrows = [[\"a\"]]",
    );
    let file: ProfileFile = toml::from_str(&bad_table).expect("parses");
    let err = file.validate().unwrap_err();
    assert!(err.contains("table row 0 has 1 cells"), "got: {err}");
}

#[test]
fn stamp_names_the_profile_source() {
    let stamp = render::stamp("demo");
    assert!(stamp.starts_with("<!-- managed by rust-specs agents;"));
    assert!(stamp.contains("agents/profiles/demo.toml"));
}

#[test]
fn target_resolution_uses_projects_base() {
    let text = MINIMAL.replace("name = \"demo\"", "name = \"demo\"\nrepo = \"some-repo\"");
    let file = load(&text);
    // SAFETY: tests are single-threaded per test binary process here; the
    // env var is set and removed around the one call that reads it.
    unsafe { std::env::set_var("AGENTS_PROJECTS_BASE", "/tmp/agents-test-base") };
    let path = file.target_path().unwrap();
    unsafe { std::env::remove_var("AGENTS_PROJECTS_BASE") };
    assert_eq!(path, std::path::PathBuf::from("/tmp/agents-test-base/some-repo/AGENTS.md"));
}

const WITH_PREAMBLE: &str = r#"
[profile]
name = "demo"
title = "demo title"

[profile.preamble]
text = "intro paragraph"

[[profile.preamble.table]]
headers = ["A", "B"]
rows = [["1", "2"]]

[[chapter]]
id = "one"
title = "One"

[[chapter.item]]
id = "x"
text = "body"
"#;

#[test]
fn preamble_text_and_tables_render_under_the_title() {
    let file = load(WITH_PREAMBLE);
    let assembly = file.assemble(&Selection::default()).unwrap();
    let doc = render::render(&file, &assembly);
    assert!(doc.starts_with("# demo title\n\nintro paragraph\n"), "got: {doc:.80}");
    assert!(doc.contains("| A | B |\n"), "table header missing");
    assert!(doc.contains("| 1 | 2 |\n"), "table row missing");
    let intro = doc.find("intro paragraph").expect("intro");
    let chapter = doc.find("## 1. One").expect("chapter");
    assert!(intro < chapter, "preamble must precede chapters");
}
