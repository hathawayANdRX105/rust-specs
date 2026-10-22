# dioxus-rsx-one-per-component

A `#[component]` function body must contain at most one content `rsx!`.

| | |
|---|---|
| Lint | `dioxus_rsx_one_per_component` |
| Default level | `Warn` |
| Parser | `syn` |

## Why

One component, one view: a single rsx keeps the render tree scannable and lets
the component be rendered/tested as a unit. A second inline rsx hides the
structure; extract a child component or pass the fragment as a slot prop.

## Known problems

The slot-wiring exception (`rsx!` in an `Option<Element>` prop value) is
counted too; allow the lint with a written reason when intentional.

## CI usage

```sh
cargo +nightly-2026-08-20 dylint --path /home/hathaway/projects/rust-specs \
  --pattern 'lints/dioxus-rsx-one-per-component' --all --workspace
```
