# dioxus-rsx-no-rust-control-flow

Bans Rust control flow inside `rsx!` bodies.

| | |
|---|---|
| Lint | `dioxus_rsx_no_rust_control_flow` |
| Default level | `Warn` |
| Scope | `ui_*` and `web` crates (by crate name); fixtures marked `dioxus-lint-fixture` |
| Parser | `syn` to find `rsx!`, `dioxus-rsx` 0.7 to parse the macro body |

## What it catches

| Construct | Level in CI |
|---|---|
| `if` / `if-else` as an rsx child | **FAIL** (`-D`) |
| `match` inside an rsx child | WARN |
| `while` inside an rsx child | WARN |
| `loop` inside an rsx child | WARN |
| `{ let ...; ... }` raw expression block | WARN |
| `for` loop | allowed |

Only the **top level** of a raw expression is judged. Rust inside closures
(`.map(|x| if ...)`) and attribute expressions (`onclick: move |_| if ...`) is
not inspected, so ordinary Rust logic does not false-positive.

## Why

rsx describes a view. Control flow embedded in it hides rendering branches
from the component boundary, makes the view hard to scan, and works against the
"one rsx per component" discipline. Move the branch above the `rsx!` call, or
extract a child component / slot.

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

The lint defaults to `Warn`. To make it blocking (the intended FAIL level for
`if` / `if-else`), pass `-D`:

```sh
cargo +nightly-2026-08-20 dylint \
  --path /home/hathaway/projects/rust-specs \
  --pattern 'lints/dioxus-rsx-no-rust-control-flow' \
  --all \
  --workspace \
  -- -D dioxus_rsx_no_rust_control_flow
```

## Known problems

- Files that fail to parse with `syn` are skipped (the lint never invents
  findings from unparseable input).
- The rule is source-text based: it sees `#[cfg]`-gated code as written, which
  is what the repo wants for style rules, but it does not know type information.
- `for` is allowed by design; nested `if`/`match` inside a `for` body is still
  reported.
