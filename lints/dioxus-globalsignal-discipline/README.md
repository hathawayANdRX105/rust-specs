# dioxus-globalsignal-discipline

Enforces the `GlobalSignal` discipline.

| | |
|---|---|
| Lint | `dioxus_globalsignal_discipline` |
| Default level | `Warn` |
| Scope | any crate that declares or uses `GlobalSignal` |

## What it catches

1. **Missing reason comment**: `static X: GlobalSignal<T>` without a `//`
   comment in the eight lines above it.
2. **Non-static use**: `GlobalSignal::new(...)` in a function body, or a local
   `let x: GlobalSignal<T>` type annotation. Reads of an existing static are
   not flagged.

## Why

A `GlobalSignal` is a process-wide writable singleton. Every one widens the
implicit dependency graph and the re-render surface. It is legitimate only for
genuinely global state (locale, toast queue, theme); state that one component
tree needs belongs in `use_signal` or a page store. The reason comment forces
the author to state why global is necessary.

## Good

```rust
/// Global by design: every page needs the current locale.
static LOCALE: GlobalSignal<Locale> = Signal::global(|| Locale::Zh);
```

## Bad

```rust
static ENABLED: GlobalSignal<bool> = Signal::global(|| true);

fn component() {
    let local: GlobalSignal<bool> = GlobalSignal::new(true);
}
```

## Known problems

- The comment check only requires *a* comment, not a good one; review judges
  quality.
- Only `GlobalSignal::new` and local type annotations are flagged; a
  `GlobalSignal` obtained through a type alias or a helper function is not.

## CI usage

```sh
cargo +nightly-2026-08-20 dylint \
  --path /home/hathaway/projects/rust-specs \
  --pattern 'lints/dioxus-globalsignal-discipline' \
  --all \
  --workspace
```
