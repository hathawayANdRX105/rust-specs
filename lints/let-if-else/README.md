# let-if-else

Bans hoisting an `if` / `if-else` whose branches build `rsx!` into a `let`
binding. The rule applies to every crate:

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

For `Element` branches, use `pick` directly:

```rust
rsx! {
    div {
        { pick(show, rsx! { span { "visible" } }, rsx! {}) }
    }
}
```

Use `pick_with` only when an unselected branch must not run (it has side
effects or collects/iterates):

```rust
{ pick_with(show, || load_page(), || load_empty()) }
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
- `let-if-else` — the same branch hoisted **out** of rsx into a `let`.

Both are active; neither reports the other's pattern. `for` inside rsx is
allowed. `match` / `while` / `loop` / `{ let ... }` inside rsx are handled by
`dioxus-rsx-avoid-control-flow` at WARN.

## Known problems

Source-text based; files that fail to parse are skipped. Only `let`
initializers are inspected. Ordinary `let x = if ..` values without an `rsx!`
branch are not reported. Generated `/icons/` paths are skipped, and
`DIOXUS_LINT_FILES` still narrows the scan in changed-files mode.

## Severity

Default `WARN`. After the existing hoists are cleaned up, merge can elevate it
with `-D let_if_else`.
