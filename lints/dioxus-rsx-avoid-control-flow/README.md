# dioxus-rsx-avoid-control-flow

Avoids `match`, `while`, `loop` and `{ let ...; ... }` blocks inside `rsx!`
bodies. `if` / `if-else` is a separate hard rule:
[`dioxus-rsx-no-if-else`](../dioxus-rsx-no-if-else/README.md) (FAIL via `-D`).

| | |
|---|---|
| Lint | `dioxus_rsx_avoid_control_flow` |
| Default level | `Warn` |
| Scope | `ui_*` and `web` crates (by crate name); fixtures marked `dioxus-lint-fixture` |
| Parser | `syn` to find `rsx!`, `dioxus-rsx` 0.7 to parse the macro body |
| Changed-files mode | `DIOXUS_LINT_FILES` (newline-separated repo-relative paths; unset/empty = all) |

## What it catches

| Construct | Level |
|---|---|
| `match` inside an rsx child | WARN |
| `while` inside an rsx child | WARN |
| `loop` inside an rsx child | WARN |
| `{ let ...; ... }` raw expression block | WARN |
| `for` loop | allowed |
| `if` / `if-else` | not this lint; see `dioxus-rsx-no-if-else` (FAIL) |

Only the **top level** of a raw expression is judged. Rust inside closures
(`.map(|x| match ...)`) and attribute expressions (`onclick: move |_| match ...`)
is not inspected, so ordinary Rust logic does not false-positive.

## Why

rsx describes a view. Branching and statement blocks embedded in it hide
rendering logic from the component boundary and make the view hard to scan.
Move the logic above the `rsx!` call, or extract a child component / slot.

## Bad

```rust
rsx! {
    div {
        { match state { _ => "idle" } }
    }
}
```

## Good

```rust
let label = match state { _ => "idle" };
rsx! { div { {label} } }
```

## CI usage

```sh
cargo +nightly-2026-08-20 dylint \
  --path /home/hathaway/projects/rust-specs \
  --pattern 'lints/dioxus-rsx-avoid-control-flow' \
  --all \
  --workspace
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
- `for` is allowed by design; nested control flow inside a `for` body is still
  reported.
