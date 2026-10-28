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

A `let` whose branches are plain values is **not** reported by this rule.
There the `if` / `else` expression is the preferred form, and `pick` is
reserved for rsx branches — see
[`pick-outside-rsx`](../pick-outside-rsx/README.md):

```rust
let class = if selected { "ui-item-selected" } else { "ui-item" };
let variant = if active { ButtonVariant::Secondary } else { ButtonVariant::Ghost };
```

A `let` bound directly to `pick(cond, rsx! { .. }, rsx! { .. })` is **not**
reported either: this lint inspects `if` expressions only, and a `pick` /
`pick_with` call is a function call, never an `Expr::If`.

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
