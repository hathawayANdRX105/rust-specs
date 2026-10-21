# early-return-needs-no-else

## What it does

Reports `if cond { return/break/continue ... } else { ... }` where the `if`
branch ends with an early exit. The `else` is dead in that path and should be
removed: move the `else` body after the `if`.

## Why is this bad?

An `else` after an early exit implies both branches are alternatives, but the
`if` path never reaches the `else`. Dropping it makes control flow linear.

## Known problems

Only the last statement of the `if` block is examined; a deeper-nested early
exit is not detected. An `else if` chain is not reported: the `else` of such a
chain is another `if`, so there is no `else` block to drop.

## Example

```rust
// warning issued
if limit > MAX {
    return Err(InvalidLimit);
} else {
    Ok(limit)
}
```

Use instead:

```rust
// no warning
if limit > MAX {
    return Err(InvalidLimit);
}
Ok(limit)
```
