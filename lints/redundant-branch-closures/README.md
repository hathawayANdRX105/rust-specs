# redundant-branch-closures

Bans a redundant pair of no-arg closures used as the two branches of a `let`
initializer.

The rule catches two shapes:

```rust
let node = pick_with(cond, || rsx! { span { "a" } }, || rsx! { span { "b" } });
```

```rust
let node = if cond { || rsx! { span { "a" } } } else { || rsx! { span { "b" } } };
```

Both branches already produce the selected value, so the closure layer is an
extra shell. Pass the values directly.

## Good

Which form to use depends on what the branches build.

rsx branches — `pick` is the only single-expression form, because
`dioxus-rsx-no-if-else` bans `if` / `else` as an rsx child:

```rust
let node = pick(cond, rsx! { span { "a" } }, rsx! { span { "b" } });
```

Plain values — a plain `if` / `else`. Do not use `pick` here; see
[`pick-outside-rsx`](../pick-outside-rsx/README.md):

```rust
let n = if cond { 1 } else { 2 };
```

## Why

The extra closure layer hides the actual branch values: the reader has to open
two closures to see what is selected. Dropping the layer keeps the choice
visible, and for plain values it also keeps the untaken branch unevaluated.

## Known problems

Source-text based (`syn`), so there is no type information: files that fail to
parse are skipped. Only `let` initializers are inspected, and a `pick_with` is
reported when it has exactly three arguments whose two branch arguments are
no-arg closures (`||` or `move ||`). `pick_with` used as a nested expression or
returned directly is not reported.

## Severity

Default `WARN`.
