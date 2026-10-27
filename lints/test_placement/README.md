# test_placement

Test code must live in `tests/`, not alongside source code in `src/`.

## What it does

Fires on `#[test]`, `#[cfg(test)]`, and `mod ..._tests;` declarations that appear
inside a crate's `src/` tree. A directory is considered a crate root when it
contains a `Cargo.toml`; the file must be located under that root's `src/`
subdirectory to be flagged.

## Why

Ferrite's rule is that all tests — unit, integration, and contract — live under
`tests/`. Keeping test code in `src/` mixes concerns and makes the public API
surface harder to audit.

## Example

```rust
// BAD: in crates/foo/src/lib.rs
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
```

```rust
// GOOD: in crates/foo/tests/unit.rs
#[test]
fn it_works() {}
```

## Default severity

`Warn`. Promote to `Deny` at merge/pre-push gate via `--deny test_placement`.
