# cjk-in-code

Source text stays English outside test code: comments, string literals and identifiers.

| | |
|---|---|
| Lint | `cjk_in_code` |
| Default level | `Warn` |
| Scanner | raw source text (a small lexer) plus `syn` for the exemptions |
| Scope | every `.rs` file of every crate — not the dioxus-only scope of the rsx rules |

## Why

The project convention is that comments are English, and user-facing copy comes from the i18n
tables. A Chinese literal in normal code is therefore either copy that cannot be translated or a
comment nobody maintains in two languages. Chinese in test code is different — the string is the
fixture — so strings are exempt there and comments are not.

## What it reports

- Chinese in a comment, anywhere: `//`, `///`, `//!` and nested `/* ... */`, including inside
  `#[cfg(test)]` modules and `#[test]` functions.
- Chinese in a string literal, byte literal, raw literal or character literal outside test scope.
- Chinese in an identifier outside test scope.

Literals inside macro invocations are scanned too: `rsx!`, `include_str!`, format strings and any
other macro body are ordinary source text to this lexer.

## Exemptions

Comments have no exemption. Strings and identifiers do:

| Exemption | How it is detected |
| --- | --- |
| test scope | the file is under `tests/`, named `*_test.rs` or `test_*.rs`, or the literal sits inside an item carrying `#[cfg(test)]` / `#[test]` / `#[cfg(all(test, ...))]` — nested items of such an item are exempt too |
| rsx text nodes | `rsx! { span { "保存" } }` is left to `dioxus-rsx-cjk-text`, which already reports it; this rule does not print the same line twice |
| rsx `data-*` attribute values | machine-readable markers (`"data-testid": "..."`), not copy |
| `include_str!` / `include_bytes!` arguments | a file name, not copy |
| i18n and data sources | crate names containing `i18n` or `locale`, and paths with `/locales/`, `/i18n/`, `/assets/`, `/data/` or `/prompts/` |

## Examples

```rust
// BAD
// 保存按钮的渲染分支
const SAVE: &str = "保存";
```

```rust
// GOOD
// Render branch for the save button.
const SAVE: &str = t!("save.label");
```

In test scope the string is allowed and the comment is not:

```rust
#[test]
fn renders_the_save_label() {
    let copy = "保存";
    // 断言文案与 locales/zh.yaml 一致
    assert_eq!(copy, "保存");
}
```

## Known problems

- It cannot tell user-visible copy from log strings: a Chinese `log::warn!` message is reported
  like any other literal, which is why the first run on a large workspace reports existing debt
  (see `FINDINGS.md`).
- Chinese inside the inline-code spans of a doc comment (`` `退避重试` ``) is reported as part of
  the comment. The fix is to write the example in English, not to restructure the doc.
- CJK is detected with the same three ranges as `dioxus-rsx-cjk-text` (`U+3400..U+4DBF`,
  `U+4E00..U+9FFF`, `U+F900..U+FAFF`), so kana, Hangul and fullwidth punctuation pass through.
- The lexer is a hand-rolled scanner, not the compiler's: it knows comments, string/char literals
  (including raw and byte strings), lifetimes and identifiers, and stops at a newline on an
  unterminated literal instead of panicking. Files are compiling by the time the lint runs, so a
  degraded scan is a low risk in practice.
- Files outside the current crate are not scanned (`dylint` runs one crate at a time); a `DIOXUS_LINT_FILES`
  diff list narrows the scan to the changed files, as it does for the dioxus rules.
- `cfg(all(test))` style groupings are recognised, but `cfg(test, foo)` style combinations that
  name a *different* cfg are not.

## Adoption

The gate script loads every lint under `lints/*` (it extracts `declare_late_lint!` names from
`lints/*/src/lib.rs`), so `cjk_in_code` joins automatically. Because it reports pre-existing debt,
start with `Warn` and clean up per crate; promote with `--deny cjk_in_code` only once a crate has no
findings.

## CI usage

```sh
cargo +nightly-2026-08-20 dylint --path /home/hathaway/projects/rust-specs \
  --pattern 'lints/cjk-in-code' --all --workspace
```
