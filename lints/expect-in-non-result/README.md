# expect-in-non-result

## What it does

Reports `.expect("...")` calls on `Result`/`Option` values inside functions
whose return type is **not** `Result`/`Option`.

## Why is this bad?

`.expect("...")` panics when the value is absent. If the enclosing function
cannot propagate the error — `?` is unavailable because the return type is not
`Result`/`Option` — the panic escapes to the caller with no recovery path.
Prefer making the function return `Result`/`Option` (and using `?`), or handle
the error explicitly.

## Example

```rust
// warning issued
fn load(path: &str) -> String {
    std::fs::read_to_string(path).expect("config file must exist")
}
```

Use instead:

```rust
fn load(path: &str) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}
```

## Known problems

- The check uses the **enclosing body owner's** return type. Closures are
  skipped (their `fn_sig` would panic and their body is usually not the
  function whose callers must recover). An `.expect` inside a closure is not
  reported even if the outer function does not return `Result`/`Option`.
- `main`, event handlers and other infallible callbacks will be flagged; add
  `#[allow(expect_in_non_result)]` where returning `Result` is not appropriate.