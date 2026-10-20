# max-nesting-depth

## What it does

Reports control-flow expressions (`if` / `match` / `loop`) nested deeper than
4 levels inside a function body.

## Why is this bad?

Beyond 4 levels the reader must track many open blocks at once. Extract helper
functions or rewrite with Option/Result combinators (`map`, `filter`,
`and_then`) to flatten the body.

## Known problems

Depth is syntactic; `for` / `while` desugar to `loop` + `match`, so they count
as two levels.

## Example

```rust
// warning issued (depth 5: for -> if-let -> for -> if-let -> if)
for x in items {
    if let Some(inner) = x {
        for y in inner {
            if let Some(v) = y {
                if *v > 0 { sum += *v; }
            }
        }
    }
}
```

Use instead:

```rust
// no warning: chain combinators
let sum = items
    .iter()
    .flatten()
    .flatten()
    .filter(|&&v| v > 0)
    .sum();
```
