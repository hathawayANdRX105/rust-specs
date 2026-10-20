# consecutive-same-shape-loops

## What it does

Reports three or more consecutive `for` loops that only push items into the
same `Vec`. Such loops can be merged into one iterator chain.

## Why is this bad?

Several loops with identical shape obscure the fact that they build one
collection. A single `flat_map`/`chain` + `collect` chain expresses the intent
at once.

## Known problems

Only detects `for x in iter { out.push(expr); }` syntactically; it does not
check that pushed expressions share a type. Review each hit before merging.

## Example

```rust
// warning issued
for m in messages {
    out.push(encode(&m));
}
for e in events {
    out.push(encode(&e));
}
for u in users {
    out.push(encode(&u));
}
```

Use instead:

```rust
// no warning
let out = messages
    .iter()
    .chain(&events)
    .chain(&users)
    .map(encode)
    .collect();
```
