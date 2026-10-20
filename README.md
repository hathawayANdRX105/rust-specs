# rust-lints

Custom Rust lint rules built on [dylint]. Each lint lives in its own directory
under `lints/`, with `src/lib.rs` (implementation), `README.md` (spec + good/bad
examples), and `ui/` (snapshot tests).

Rules here cover what clippy cannot express, e.g. checks that need type info,
project-specific style rules, or readability/nesting policies.

## Rules

| Lint | What it detects | Status |
|---|---|---|
| `ban-unwrap` | any bare `.unwrap()` on Result/Option; use `expect("reason")` or `?` | ✅ implemented, ui-tested |

## Usage

Run all lints against a package in the ferrite repo:

```sh
cargo +nightly-2026-08-20 dylint \
  --path /home/hathaway/projects/rust-lints \
  --pattern 'lints/*' \
  --all \
  -- -p <crate>
```

Pin the toolchain in `rust-toolchain.toml` (currently `nightly-2026-08-20`).

## Adding a lint

1. `cargo dylint new lints/<name>`
2. Implement `LateLintPass` in `src/lib.rs`
3. Add trigger cases to `ui/main.rs`, run `cargo test` to bless `ui/main.stderr`
4. Update this README table

[dylint]: https://github.com/trailofbits/dylint
