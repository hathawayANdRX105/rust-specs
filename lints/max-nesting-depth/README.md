# max-nesting-depth

## What it does

Reports control-flow expressions (`if` / `match` / `loop`) nested deeper than
4 levels inside a function body.

## Why is this bad?

Beyond 4 levels the reader must track many open blocks at once. Extract helper
functions or rewrite with Option/Result combinators (`map`, `filter`,
`and_then`) to flatten the body.

## Known problems

Depth counts **visual** nesting: a `for`/`while` counts as one level (its HIR
desugaring — `Match` + `Loop` + the `next()` `Match` — is not counted), and an
`else if` chain counts as one level (the chain is flat, not nested). The walk
visits every HIR node once, so a node reachable through two paths (a
desugared `while let` body, for instance) is measured once.

Compiler-generated constructs do not count as levels either. `<expr>.await`
and `?` desugar into a `Match` (`MatchSource::AwaitDesugar` /
`TryDesugar`) whose arms hold the generator's state machine; counting those
would add a level the source does not show, so every `await` inside a nested
expression would inflate the depth.

`while let` bodies are followed into the desugared `if let`; a `while cond`
body is followed past the desugared condition check.

Reported depths have not been cross-checked against every HIR shape; treat a
hit as a prompt to look, not as a verdict.

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
