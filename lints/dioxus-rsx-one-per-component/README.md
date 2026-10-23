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

Source-text based; files that fail to parse are skipped. Only **content**
`rsx!` calls count:

- a direct statement or tail expression: `rsx! { .. }` / `rsx! { .. };`
- a direct `let` initializer: `let header = rsx! { .. };`
- a direct `return`: `return rsx! { .. };`

Fragments nested in `if`/`match` branches, loops, closures (`map`, `then`),
prop values, or any other expression are **not** counted. Hoisting a
conditional into `let node = if .. { rsx!{} } else { rsx!{} };` therefore does
not trigger this lint.

## Examples

Good — one content rsx, the conditional fragment is not a content view:

```rust
#[component]
fn Card(c: bool) -> Element {
    let node = if c { rsx! { div { "a" } } } else { rsx! { div { "b" } } };
    rsx! { div { {node} } }
}
```

Bad — a direct `let` fragment plus the returned view (two content rsx):

```rust
#[component]
fn Card() -> Element {
    let header = rsx! { h1 { "title" } };
    rsx! { div { {header} } }
}
```

## CI usage

```sh
cargo +nightly-2026-08-20 dylint --path /home/hathaway/projects/rust-specs \
  --pattern 'lints/dioxus-rsx-one-per-component' --all --workspace
```
