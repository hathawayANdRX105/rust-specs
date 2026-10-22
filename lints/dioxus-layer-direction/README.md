# dioxus-layer-direction

Enforces the web crate layering direction.

| | |
|---|---|
| Lint | `dioxus_layer_direction` |
| Default level | `Warn` |
| Scope | every crate; the file path decides the intra-crate rule |

## What it catches

- `use crate::views::...` / `use super::views::...` in `components/`,
  `layouts/` or `utils/`;
- a `ui_*` crate importing `api_*`, `gateway_*` or `app_db`.

## Why

Layering is one-way: views -> components/layouts/utils. A reverse dependency
forces a low-level component to know page-level state, and the UI can no longer
be split or reused. A UI crate importing a server crate drags the backend into
the wasm build.

## Bad / good

```rust
// crates/ui-pages/src/components/card.rs
use crate::views::dashboard::DashboardState; // bad: pass it through props
```

## Known problems

Syntactic on `use` items: a re-exported path or a fully-qualified call
(`crate::views::foo()`) is not caught.

## CI usage

```sh
cargo +nightly-2026-08-20 dylint --path /home/hathaway/projects/rust-specs \
  --pattern 'lints/dioxus-layer-direction' --all --workspace
```
