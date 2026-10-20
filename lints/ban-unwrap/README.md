# ban-unwrap

## What it does

Reports any bare `.unwrap()` call on a `Result`/`Option` receiver, and any
`expect("...")` message that contains CJK characters.

## Why is this bad?

`unwrap` panics without explaining why the failure is impossible. The project
requires `expect("english reason")` instead: the message documents the invariant
that makes the unwrap safe. In fallible functions, `?` propagates the error.

## Known problems

The lint does not distinguish tests/examples. Add `#[allow(ban_unwrap)]` where a
bare unwrap is acceptable, or configure paths to exclude them.

## Example

```rust
// warning issued
let x = v.unwrap();
let y = v.expect("值必须存在");
```

Use instead:

```rust
// no warning
let x = v.expect("value must be present");
let y = v?; // in a fallible function
```
