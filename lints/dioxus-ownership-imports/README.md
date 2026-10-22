# dioxus-ownership-imports

A shared component crate must not import business types.

| | |
|---|---|
| Lint | `dioxus_ownership_imports` |
| Default level | `Warn` |
| Scope | crate `ui_components` |

## What it catches

`use ui_api::...` or `use app_contract::...` inside `ui-components`.

## Why

`ui-components` takes props and emits events. Importing a business crate
couples every consumer to that business model, so the component can no longer
be reused across pages/apps and the dependency direction page -> component is
reversed.

## Bad / good

```rust
// bad
use ui_api::dto::Channel;

// good: define plain props; the page maps its DTO to them
struct ChannelRowProps { name: String, active: bool }
```

## Known problems

Only direct `use` items are checked; a fully-qualified path or a re-export is
not caught. The crate scope is hard-coded to `ui_components`.

## CI usage

```sh
cargo +nightly-2026-08-20 dylint --path /home/hathaway/projects/rust-specs \
  --pattern 'lints/dioxus-ownership-imports' --all --workspace
```
