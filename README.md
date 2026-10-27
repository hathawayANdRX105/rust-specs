# rust-specs

Custom Rust syntax rules and spec checks built on [dylint]. Each lint lives in its own directory
under `lints/`, with `src/lib.rs` (implementation), `README.md` (spec + good/bad
examples), and `ui/` (snapshot tests).

Rules here cover what clippy cannot express, e.g. checks that need type info,
project-specific style rules, or readability/nesting policies.

## Layout

| Path       | Contents                                                                |
| ---------- | ----------------------------------------------------------------------- |
| `lints/`   | dylint rules (Rust syntax and semantics that need type information)     |
| `checks/`  | non-syntax checks (GitHub PR / issue format), run through `cargo xtask` |
| `agents/`  | per-project `AGENTS.md` assembly from TOML profiles                     |
| `specs/`   | human-readable rule specs, checklists, and good/bad examples            |
| `support/` | helper crates used by the dioxus lints                                  |

## Specs

This repository also maintains a spec library (`specs/`): GitHub issue / PR format checks
translated into checklists an agent can follow directly, with good and bad examples. dylint
owns syntax; xtask owns special checks (for example GitHub issue / PR format). Entry point:
[specs/README.md](specs/README.md).

## Checks

Non-syntax checks live in the `checks/` crate and run through the `cargo xtask` alias:

```bash
cargo xtask check github-pr --title "<title>" --body-file <path> [--labels a,b] [--branch <name>]
cargo xtask check github-issue --title "<title>" --body-file <path> [--labels a,b]
```

- `github-pr`: checks a PR title/body against [specs/github/pr.md](specs/github/pr.md) (English title, conventional prefix, required sections, checkboxes, type label, branch prefix).
- `github-issue`: checks an issue title/body against [specs/github/issue.md](specs/github/issue.md) (Chinese title, required sections, Done when checkboxes, forbidden content, type label).
- Without `--title` / `--body`, the current PR or issue is read through `gh pr view` / `gh issue view`.
- Exit codes: 0 = no FAIL; 1 = at least one FAIL; 2 = `gh` call failed or bad arguments.

## Agents

Per-project `AGENTS.md` documents are assembled from TOML profiles by the `agents/` crate.
Profiles live in `agents/profiles/<name>.toml` and declare ordered chapters made of small,
individually addressable points — any chapter or point can be included, dropped, or reordered
at render time. See [agents/README.md](agents/README.md).

```bash
cargo agents list
cargo agents build ferrite                          # render to stdout
cargo agents build ferrite --chapter code,testing   # free assembly
cargo agents push ferrite                           # write the target file (stamped)
cargo agents check ferrite                          # drift check
```

## Rules

| Lint                           | What it detects                                                                                                                                             | Status                                                    |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| `ban-unwrap`                   | any bare `.unwrap()` on Result/Option; use `expect("reason")` or `?`; expect messages must be English                                                       | ✅ implemented, ui-tested                                 |
| `max-nesting-depth`            | `if`/`match`/`loop`/`for`/`while` nested deeper than 4 visual levels                                                                                        | ✅ implemented, ui-tested                                 |
| `early-return-needs-no-else`   | `if c { return/break/continue } else { .. }` → drop the `else`                                                                                              | ✅ implemented, ui-tested                                 |
| `consecutive-same-shape-loops` | ≥3 same-shape push loops in one function → chain                                                                                                            | ✅ implemented, ui-tested                                 |
| `expect-in-non-result`         | `.expect()` on Result/Option in a function that does not return Result/Option (`?` unavailable)                                                             | ✅ implemented, ui-tested                                 |
| `chainable-single-statement`   | single-expression `if let` / `for` bodies that could be chained into `is_some_and` / `for_each`                                                             | ✅ implemented, ui-tested; adopted (detection-only, WARN) |
| `let-if-else`                  | `let x = if ... { rsx!{} } else { rsx!{} };` hoisting — use `pick` / `pick_with`                                                                            | ✅ implemented, ui-tested                                 |
| `redundant-branch-closures`    | both branches of a `let`-bound `pick_with` / `if-else` are no-arg closures — drop the closure layer (`pick` for rsx branches, `if`/`else` for plain values) | ✅ implemented, ui-tested                                 |
| `pick-outside-rsx`             | `ui_util::pick` / `pick_with` outside `rsx!` with no rsx branch — use a plain `if`/`else` expression                                                        | ✅ implemented, ui-tested                                 |
| `test_placement`               | `#[test]`, `#[cfg(test)]`, or `mod ..._tests;` inside a crate's `src/` tree — test code must live in `tests/`                                               | ✅ implemented, ui-tested                                 |

## Dioxus rules

The dioxus rules apply to `ui_*` / `web` crates (plus fixtures marked
`dioxus-lint-fixture`). The rsx-structure rules parse rsx with `dioxus-rsx` 0.7.

| Lint                             | What it detects                                                              | Default |
| -------------------------------- | ---------------------------------------------------------------------------- | ------- |
| `dioxus-rsx-no-if-else`          | `if` / `if-else` as an rsx child — **FAIL via `-D dioxus_rsx_no_if_else`**   | WARN    |
| `dioxus-rsx-avoid-control-flow`  | `match` / `while` / `loop` / `{ let ... }` inside rsx (`for` allowed)        | WARN    |
| `dioxus-rsx-nesting`             | rsx element nesting deeper than one level                                    | WARN    |
| `dioxus-rsx-one-per-component`   | a `#[component]` body with more than one content `rsx!`                      | WARN    |
| `dioxus-rsx-cjk-text`            | Chinese text inside rsx element bodies                                       | WARN    |
| `dioxus-rsx-list-key`            | `key: "{index}"` / `{i}` / `{idx}` array-index keys                          | WARN    |
| `dioxus-effect-self-loop`        | `use_effect` writing a `Signal` it also reads                                | WARN    |
| `dioxus-globalsignal-discipline` | `GlobalSignal` static without a reason comment; non-static use               | WARN    |
| `dioxus-props-contract`          | `handle_*` props, `render_*` / `show_*: bool`, more than 7 fields            | WARN    |
| `dioxus-layer-direction`         | `components/layouts/utils` importing `views`; `ui_*` importing server crates | WARN    |
| `dioxus-ownership-imports`       | `ui-components` importing `ui_api` / `app_contract`                          | WARN    |

Changed-files mode: set `DIOXUS_LINT_FILES` to a newline-separated list of
repo-relative paths (e.g. `git diff --name-only`) to scan only those files;
unset or empty scans everything. Generated icon modules are always excluded:
any path containing `/icons/` (mainly
`crates/ui-components/src/icons/**`) is skipped, even when it appears in the
diff list. `bin/kit-demo` is outside the `ui_*` / `web` crate scope and is not
scanned.

## Usage

Run every lint against the whole workspace (from the repository being checked):

```sh
cargo +nightly-2026-08-20 dylint \
  --path /home/hathaway/projects/rust-specs \
  --pattern 'lints/*' \
  --all --workspace
```

Run the lints against a single crate:

```sh
cargo +nightly-2026-08-20 dylint --no-deps \
  --path /home/hathaway/projects/rust-specs \
  --pattern 'lints/*' -p <crate>
```

Pin the toolchain in `rust-toolchain.toml` (currently `nightly-2026-08-20`).

## Adding a lint

1. `cargo dylint new lints/<name>`
2. Implement `LateLintPass` in `src/lib.rs`
3. Add trigger cases to `ui/main.rs`, run `cargo test` to bless `ui/main.stderr`
4. Update this README table

[dylint]: https://github.com/trailofbits/dylint
