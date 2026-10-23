# rust-specs

Custom Rust syntax rules and spec checks built on [dylint]. Each lint lives in its own directory
under `lints/`, with `src/lib.rs` (implementation), `README.md` (spec + good/bad
examples), and `ui/` (snapshot tests).

Rules here cover what clippy cannot express, e.g. checks that need type info,
project-specific style rules, or readability/nesting policies.

## Specs

本仓库同时维护一份规范库（`specs/`）：把 GitHub issue / PR 的格式检查翻译成 agent 可直接照做的检查清单与好坏例子。dylint 管语法，xtask 管特殊检查（如 GitHub issue/PR 格式）。入口见 [specs/README.md](specs/README.md)。
## Checks

非语法类特殊检查放在 `checks/` crate，通过 `cargo xtask` alias 运行：

```bash
cargo xtask check github-pr --title "<title>" --body-file <path> [--labels a,b] [--branch <name>]
cargo xtask check github-issue --title "<title>" --body-file <path> [--labels a,b]
```

- `github-pr`：按 [specs/github/pr.md](specs/github/pr.md) 检查 PR 标题/正文（标题英文、conventional 前缀、必需小节、勾选框、type label、分支前缀）。
- `github-issue`：按 [specs/github/issue.md](specs/github/issue.md) 检查 issue 标题/正文（标题中文、必需小节、Done when 勾选、禁用内容、type label）。
- 不传 title/body 时自动用 `gh pr view` / `gh issue view` 读当前 PR/issue 补全。
- 退出码：0 = 无 FAIL；1 = 有 FAIL；2 = gh 调用失败或参数错误。

## Rules

| Lint | What it detects | Status |
|---|---|---|
| `ban-unwrap` | any bare `.unwrap()` on Result/Option; use `expect("reason")` or `?`; expect messages must be English | ✅ implemented, ui-tested |
| `max-nesting-depth` | `if`/`match`/`loop`/`for`/`while` nested deeper than 4 visual levels | ✅ implemented, ui-tested |
| `early-return-needs-no-else` | `if c { return/break/continue } else { .. }` → drop the `else` | ✅ implemented, ui-tested |
| `consecutive-same-shape-loops` | ≥3 same-shape push loops in one function → chain | ✅ implemented, ui-tested |
| `expect-in-non-result` | `.expect()` on Result/Option in a function that does not return Result/Option (`?` unavailable) | ✅ implemented, ui-tested |
| `chainable-single-statement` | single-expression `if let` / `for` bodies that could be chained into `is_some_and` / `for_each` | 🚧 detection-only (stats pending) |

## Dioxus rules

The dioxus rules apply to `ui_*` / `web` crates (plus fixtures marked
`dioxus-lint-fixture`). The rsx-structure rules parse rsx with `dioxus-rsx` 0.7.

| Lint | What it detects | Default |
|---|---|---|
| `dioxus-rsx-no-if-else` | `if` / `if-else` as an rsx child — **FAIL via `-D dioxus_rsx_no_if_else`** | WARN |
| `dioxus-let-if-else` | `let x = if ... { rsx!{} } else { rsx!{} };` hoisting — use `pick_with` / `pick` | WARN |
| `dioxus-rsx-avoid-control-flow` | `match` / `while` / `loop` / `{ let ... }` inside rsx (`for` allowed) | WARN |
| `dioxus-rsx-nesting` | rsx element nesting deeper than one level | WARN |
| `dioxus-rsx-one-per-component` | a `#[component]` body with more than one content `rsx!` | WARN |
| `dioxus-rsx-cjk-text` | Chinese text inside rsx element bodies | WARN |
| `dioxus-rsx-list-key` | `key: "{index}"` / `{i}` / `{idx}` array-index keys | WARN |
| `dioxus-effect-self-loop` | `use_effect` writing a `Signal` it also reads | WARN |
| `dioxus-globalsignal-discipline` | `GlobalSignal` static without a reason comment; non-static use | WARN |
| `dioxus-props-contract` | `handle_*` props, `render_*` / `show_*: bool`, more than 7 fields | WARN |
| `dioxus-layer-direction` | `components/layouts/utils` importing `views`; `ui_*` importing server crates | WARN |
| `dioxus-ownership-imports` | `ui-components` importing `ui_api` / `app_contract` | WARN |

Changed-files mode: set `DIOXUS_LINT_FILES` to a newline-separated list of
repo-relative paths (e.g. `git diff --name-only`) to scan only those files;
unset or empty scans everything. Generated icon modules are always excluded:
any path containing `/icons/` (mainly
`crates/ui-components/src/icons/**`) is skipped, even when it appears in the
diff list. `bin/kit-demo` is outside the `ui_*` / `web` crate scope and is not
scanned.

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
