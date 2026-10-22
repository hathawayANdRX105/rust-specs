# dioxus-rsx-no-if-else

Bans `if` / `if-else` inside `rsx!` bodies.

| | |
|---|---|
| Lint | `dioxus_rsx_no_if_else` |
| Default level | `Warn` (CI/merge uses `-D dioxus_rsx_no_if_else` to make it **FAIL**) |
| Scope | `ui_*` and `web` crates (by crate name); fixtures marked `dioxus-lint-fixture` |
| Parser | `syn` to find `rsx!`, `dioxus-rsx` 0.7 to parse the macro body |
| Changed-files mode | `DIOXUS_LINT_FILES` (newline-separated repo-relative paths; unset/empty = all) |

## What it catches

| Construct | Level |
|---|---|
| `if` / `if-else` / `else if` as an rsx child | **FAIL** (`-D`) |
| `for` loop | allowed |
| `match` / `while` / `loop` / `{ let ... }` | not this lint; see `dioxus-rsx-avoid-control-flow` (WARN) |

Only rsx **child nodes** are judged. Rust inside closures
(`.map(|x| if ...)`) and attribute expressions (`onclick: move |_| if ...`)
is not inspected, so ordinary Rust logic does not false-positive.

## Why

rsx describes a view. An `if` embedded in it hides a rendering branch from the
component boundary, makes the view hard to scan, and works against the
"one rsx per component" discipline.

## Bad

```rust
rsx! {
    div {
        if show { span { "visible" } }
    }
}
```

## Good

```rust
let body = if show { rsx! { span { "visible" } } } else { rsx! {} };
rsx! { div { {body} } }
```

## CI usage

```sh
cargo +nightly-2026-08-20 dylint \
  --path /home/hathaway/projects/rust-specs \
  --pattern 'lints/dioxus-rsx-no-if-else' \
  --all \
  --workspace \
  -- -D dioxus_rsx_no_if_else
```

To scan only the files touched by the current change:

```sh
export DIOXUS_LINT_FILES="$(git diff --name-only)"
```

## Known problems

- Files that fail to parse with `syn` are skipped (the lint never invents
  findings from unparseable input).
- Source-text based: it sees `#[cfg]`-gated code as written; it does not know
  type information.
- `for` is allowed by design; an `if` nested inside a `for` body is still
  reported.
