# chainable-single-statement

## What it does

Reports `if let` and `for` bodies that contain exactly one expression and no
control-flow escape, i.e. candidates for chaining into `Option::is_some_and` /
`Result::is_ok_and` / `Result::is_err_and` / `Iterator::for_each`.

**`let _ = expr;` bodies are excluded by design**: the `let _` is the explicit
"deliberately ignore this result" signal, and folding it into
`is_some_and(|x| ...is_ok())` hides that intent inside a closure.

This rule is **detection-only** for now: it reports candidates so they can be
counted and reviewed. A type-aware rewrite suggestion is a follow-up (see the
`chainable-single-statement` evaluation note in ferrite `todo/spec/`).

## Why is this bad?

A single-expression body often reads better as one chain:

```rust
// reported
if let Some(x) = value {
    x.checked_add(1).is_some();
}

// preferred
value.is_some_and(|x| x.checked_add(1).is_some());
```

```rust
// reported
for chunk in content {
    handlers.on_message_chunk(&session_id, &chunk.text);
}

// preferred
content
    .into_iter()
    .for_each(|chunk| handlers.on_message_chunk(&session_id, &chunk.text));
```

## Known problems

- **No type check yet**: the suggestion must match the body type (`bool` →
  `is_some_and`; `Result` → `.is_ok()`), which the detection-only version does
  not emit.
- **Escapes are skipped**: the body must not contain `?`, `return`, `break`,
  `continue` or `.await`; inside a closure those would change meaning.
- **Nested control flow is skipped**: a body that is itself an `if`/`match`/
  loop reads worse as a closure.
- **`else` is required for `if let`**: an `if let ... else` is not reported
  (the rewrite would need `map_or`/`map_or_else`).
- **`let _ = ...` is not reported** (deliberate discard, see above).
- Macro expansions: `println!`/`vec!`/`format!` expand to a block; the rule
  looks through a single-statement block so the macro call counts as the body.

## Example

```rust
// warning issued
if let Some(x) = value {
    x.checked_add(1).is_some();
}

// warning issued
for x in values {
    use_value(x);
}
```
