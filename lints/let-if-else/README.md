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

Keeping the branch hoisted in a `let` adds an indirection before the returned
rsx. Write the branch inside the `rsx!` body directly — plain `if` / `if-else`
as an rsx child is not restricted by any rule in this library — or extract a
child component / slot.

## Good

For `Element` branches, write the branch inline in the `rsx!` body:

```rust
rsx! {
    div {
        if show { span { "visible" } }
    }
}
```

A `let` whose branches are plain values is **not** reported by this rule.
There the `if` / `else` expression is the preferred form.

```rust
let class = if selected { "ui-item-selected" } else { "ui-item" };
let variant = if active { ButtonVariant::Secondary } else { ButtonVariant::Ghost };
```

A `let` bound to anything that is not an `if` expression is **not** reported:
this lint inspects `if` expressions only — a function call is never an
`Expr::If`.

## Why

A `let node = if ..` block adds a second view-shaped value and pushes the reader
through a branch before reaching the returned rsx. Write the branch inside the
`rsx!` body directly, or extract a child component / slot.

## Division of labour

- `let-if-else` — an `if` / `if-else` with `rsx!` branches hoisted into a `let`.

`for` inside rsx is allowed. `match` / `while` / `loop` / `{ let ... }`
inside rsx are handled by `dioxus-rsx-avoid-control-flow` at WARN. Plain
`if` / `if-else` written directly inside `rsx!` is not restricted by any rule
in this library.

## Known problems

Source-text based; files that fail to parse are skipped. Only `let`
initializers are inspected. Ordinary `let x = if ..` values without an `rsx!`
branch are not reported. Generated `/icons/` paths are skipped, and
`DIOXUS_LINT_FILES` still narrows the scan in changed-files mode.

## Severity

Default `WARN`. After the existing hoists are cleaned up, merge can elevate it
with `-D let_if_else`.
