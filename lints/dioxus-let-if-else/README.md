# dioxus-let-if-else

Bans hoisting an `if` / `if-else` whose branches build `rsx!` into a `let`
binding:

```rust
let node = if show {
    rsx! { span { "visible" } }
} else {
    rsx! {}
};
rsx! { div { {node} } }
```

This satisfies `dioxus-rsx-no-if-else` on a technicality but keeps the view
split into a branch block. Use the project's single-expression helpers from
`ui_util::pick` instead.

## Good

For `Element` branches (lazy — only the selected closure runs):

```rust
rsx! {
    div {
        { pick_with(show, || rsx! { span { "visible" } }, || rsx! {}) }
    }
}
```

For cheap `Copy` values (strings, variants, numbers):

```rust
let class = pick(selected, "ui-item-selected", "ui-item");
let variant = pick(active, ButtonVariant::Secondary, ButtonVariant::Ghost);
```

## Why

`pick` / `pick_with` keep the choice at the call site, in one expression. A
`let node = if ..` block adds a second view-shaped value and pushes the reader
through a branch before reaching the returned rsx.

## Division of labour

- `dioxus-rsx-no-if-else` — an `if` written **inside** the `rsx!` body.
- `dioxus-let-if-else` — the same branch hoisted **out** of rsx into a `let`.

Both are active; neither reports the other's pattern. `for` inside rsx is
allowed. `match` / `while` / `loop` / `{ let ... }` inside rsx are handled by
`dioxus-rsx-avoid-control-flow` at WARN.

## Known problems

Source-text based; files that fail to parse are skipped. Only `let`
initializers are inspected. Ordinary `let x = if ..` values without an `rsx!`
branch are not reported. The lint scans `ui_*` / `web` crates (and fixtures
marked `dioxus-lint-fixture`), skips generated `/icons/` paths, and honours
`DIOXUS_LINT_FILES` for changed-files mode.

## Severity

Default `WARN`. After the existing hoists are cleaned up, merge can elevate it
with `-D dioxus_let_if_else`.
