# pick-outside-rsx

## What it does

Reports `ui_util::pick(..)` and `ui_util::pick_with(..)` calls that sit
outside an `rsx!` body.

`pick` exists for one reason: `dioxus-rsx-no-if-else` bans `if` / `else` as
an `rsx!` child, so a branch between two elements has to be a single
expression. Outside `rsx!` there is no such restriction, and the plain Rust
form reads better.

```rust
// reported
let class = pick(active, "row-active", "row");

// preferred
let class = if active { "row-active" } else { "row" };
```

## Why is this bad?

A helper call hides a two-branch decision behind a name the reader has to
look up, and it makes the two branches eager: both are evaluated before one
is discarded. The plain `if` / `else` says what it does at the call site and
keeps the unused branch unevaluated.

`pick` also widens the crate's dependency surface for a construct that plain
Rust already expresses, which is why the rule keeps it for `rsx!` only.

## Known problems

Source-text based (`syn`), so there is no type information: files that fail
to parse are skipped.

The check is positional — a call is inside `rsx!` when it appears within an
`rsx!` macro invocation. A `pick` in an attribute expression of an `rsx!`
element counts as inside, since that expression is ordinary Rust and the
`dioxus-rsx-no-if-else` lint does not inspect attribute expressions either.

A call whose branches build `rsx!` elements is never reported, wherever it
is written. `dioxus-rsx-no-if-else` bans `if` / `else` as an `rsx!` child, so
a choice between two elements has to be a single expression, and hoisting the
result into a `let` is sometimes forced — a `#[cfg]` attribute is not
accepted in an rsx child position. Only a choice between plain values has a
better form.

Calls to other functions named `pick` are not reported: only the path
segments `ui_util::pick`, `ui_util::pick::pick` and a bare `pick` that was
imported from `ui_util::pick` are recognized.

## Example

```rust
// warning issued
let state = pick(open, "open", "closed");

// no warning — inside rsx
rsx! {
    div {
        { pick(open, rsx! { span { "open" } }, rsx! {}) }
    }
}
```
