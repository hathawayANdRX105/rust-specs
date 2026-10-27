# is-check-needs-chain

## What it does

Reports a lone `is_some` / `is_none` / `is_ok` / `is_err` check on an
`Option`/`Result` receiver that is immediately followed by an expression
which is itself an `Option`/`Result`. The value is already the target
shape; the check re-enters it through `bool` or a hand-written `if`/`else`
instead of a combinator chain.

Two shapes are reported:

```rust
// Shape A: re-entering Option through bool
let a: Option<u32> = o.is_some().then(|| 1);
let b: Option<u32> = o.is_some().then_some(1);

// Shape B: hand-written if/else over the predicate
let c: Option<u32> = if o.is_some() { Some(1) } else { None };
```

Use instead:

```rust
let a: Option<u32> = o.map(|_| 1);
let b: Option<u32> = o.map(|_| 1);
let c: Option<u32> = o.map(|_| 1);
```

`Result` receivers behave the same (`is_ok` / `is_err`).

## Why is this bad?

The receiver is already `Option`/`Result`. Asking `is_*` first throws the
payload away and then rebuilds the same envelope, so the reader tracks two
decisions where one combinator says it all. `is_*` is a predicate: use it
alone (returning `bool`), or chain on the receiver — not in between.

## Deliberate non-hits

- **bool → Option → plain value is sanctioned**: `then_some(..)` /
  `then(..)` followed directly by `unwrap_or(..)` / `unwrap_or_else(..)` /
  `unwrap_or_default()` terminate in a plain value. That is the accepted
  two-way choice for non-`Option` results (e.g.
  `opt.is_none().then_some(default).unwrap_or(v)`), and wrapping or banning
  it would be over-refactoring.
- **Lone predicate**: `o.is_some()` returned or stored as `bool` is correct.
- **Compound conditions**: `o.is_some() && flag` is a boolean expression,
  not a lone check.
- **Plain-value if/else**: `if s.is_empty() { "manual" } else { s }` on a
  `String` is the repo default (§3 binary value choice) — the receiver is
  not `Option`/`Result` and the output is not either.
- `is_some_and(..)` / `is_ok_and(..)` are already chains.

## Known problems

- The suggested combinator is directional: the right rewrite (`map`,
  `map_or`, `filter`, `and_then`) depends on what the branches actually
  do, which the lint does not decide.
- Shape B reports any `if <lone is_*> { .. } else { .. }` whose whole
  expression is `Option`/`Result`, even when a branch carries logic beyond
  `Some`/`None`.
- Shape A recognizes `then`/`then_some` by method name on a
  `bool`-producing check call; a custom inherent `then` on a foreign type
  shadowing `bool::then` would still be reported.

## Example

```rust
// warning issued
fn count(o: Option<u32>) -> Option<u32> {
    if o.is_some() { Some(1) } else { None }
}
```

```rust
// no warning
fn count(o: Option<u32>) -> Option<u32> {
    o.map(|_| 1)
}
```

## Severity

Default `WARN`.
