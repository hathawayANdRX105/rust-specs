# rust-specs

Custom Rust syntax rules and spec checks built on [dylint]. Each lint lives in its own directory
under `lints/`, with `src/lib.rs` (implementation), `README.md` (spec + good/bad
examples), and `ui/` (snapshot tests).

Rules here cover what clippy cannot express, e.g. checks that need type info,
project-specific style rules, or readability/nesting policies.

## Rules

| Lint | What it detects | Status |
|---|---|---|
| `ban-unwrap` | any bare `.unwrap()` on Result/Option; use `expect("reason")` or `?`; expect messages must be English | ✅ implemented, ui-tested |
| `max-nesting-depth` | `if`/`match`/`loop`/`for`/`while` nested deeper than 4 visual levels | ✅ implemented, ui-tested |
| `early-return-needs-no-else` | `if c { return/break/continue } else { .. }` → drop the `else` | ✅ implemented, ui-tested |
| `consecutive-same-shape-loops` | ≥3 same-shape push loops in one function → chain | ✅ implemented, ui-tested |
| `expect-in-non-result` | `.expect()` on Result/Option in a function that does not return Result/Option (`?` unavailable) | ✅ implemented, ui-tested |

## Usage

Run all lints against a package in the ferrite repo:

```sh
cargo +nightly-2026-08-20 dylint \
  --path /home/hathaway/projects/rust-specs \
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
