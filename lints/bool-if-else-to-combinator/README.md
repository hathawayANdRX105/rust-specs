# bool-if-else-to-combinator

## What it does

Reports a `bool` `if`/`else` whose branches only wrap a value, i.e. the
condition already is the `bool` a combinator chain starts from. The branch
pairs below are recognised, in both directions:

```rust
// reported
let a = if ready { Some(1) } else { None }; // ready.then_some(1)
let b = if ready { None } else { Some(1) }; // (!ready).then_some(1)
let c = if ready { Ok(1) } else { Err(code) }; // ready.then_some(1).ok_or(code)
let d = if ready { Err(code) } else { Ok(1) }; // (!ready).then_some(1).ok_or(code)
let e = if ready { Ok(()) } else { Err(code) }; // ready.ok_or(code)
```

`Result` takes the two-step chain because `bool::ok_or` returns `Result<(), E>`:
it carries the error but not the `Ok` value, so the value has to travel through
`Option` first. `ready.ok_or(code)` is therefore not a rewrite of
`if ready { Ok(1) } else { Err(code) }` — it would drop the `1`. A
`Result<(), E>` if/else has no value to carry, so it ends at the error alone,
which is what `bool::ok_or` / `bool::ok_or_else` produce.

## Eager versus lazy payloads

`then_some` and `ok_or` evaluate their argument eagerly, while the `if`/`else`
evaluates a branch only when it is taken. The suggestion keeps the old timing:
a cheap payload (literal, path) uses the eager form, anything else (a call, an
allocation) the lazy one.

```rust
// reported: `ready` and `7` are cheap
let a = if ready { Ok(1) } else { Err(7) }; // ready.then_some(1).ok_or(7)

// reported: both payloads are built, so both stay behind a closure
let b = if ready {
    Ok(String::from("value"))
} else {
    Err(String::from("boom"))
}; // ready.then(|| String::from("value")).ok_or_else(|| String::from("boom"))
```

## Why is this bad?

The `if`/`else` spends five lines restating one chain, and the branches carry
no logic beyond the wrapper — nothing is gained by spelling them out.

## Deliberate non-hits

- **A lone `is_*` condition**: `if o.is_some() { Some(1) } else { None }` is
  `is-check-needs-chain`'s Shape B; the receiver is already an `Option`, so
  `o.map(..)` beats `o.is_some().then_some(..)`.
- **A branch with statements**: `{ let n = 1; Some(n) }` cannot move into a
  combinator argument.
- **`else if` chains**: the outer expression is not a single if/else; each leaf
  is checked on its own.
- **A plain binary value choice**: `if ready { 1 } else { 2 }` is the repo
  default (§3 binary value choice) and is never reported — the branches wrap
  nothing.
- **`if let` / let-chains**: the condition is not an expression that can move
  behind a method call.
- **Locally defined `Some`/`None` variants**: constructors are resolved by
  language item, not by name.

## Known problems

- No suggestion is emitted when a payload needs a coercion at the constructor
  call (unsize, deref): `then_some`/`ok_or` take it as a generic argument,
  where that coercion would not apply. The shape is still reported.
- The value-carrying `Result` rewrite is a two-combinator chain; it is one line
  instead of five, but it is denser than the `if`/`else` it replaces.

## Example

```rust
// warning issued
fn lookup(ready: bool) -> Option<u32> {
    if ready {
        Some(1)
    } else {
        None
    }
}
```

Use instead:

```rust
// no warning
fn lookup(ready: bool) -> Option<u32> {
    ready.then_some(1)
}
```

## Severity

Default `WARN`.