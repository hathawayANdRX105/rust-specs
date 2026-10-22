# Real-world findings (ferrite)

Scans produced by rust-specs rules against the ferrite workspace.

## max-nesting-depth (2026-10-21, after the depth fixes)

Whole-workspace scan, with the rule counting *visual* nesting:

**39 hits.** The number came down from 274 as the rule was corrected:

| Version | Hits | What was wrong |
|---|---|---|
| first | 274 | counted the `for`/`while` HIR desugaring and the `else if` chain |
| for/else-if fixed | 96 | a `match` scrutinee and an `if` condition were counted as nested |
| scrutinee fixed | 58 | a `while` was never counted (it is the `Loop` node itself) |
| while counts again | 89 | — (`while` now counts, so the number went up again) |
| visited set | 78 | the same HIR node was reached through two paths and counted twice |
| while-let body | 71 | the `while let` body was skipped, and the loop counted twice around it |
| closure skip | 64 | a closure body was walked twice: from `check_fn` and from its enclosing expression |
| await desugar | 39 | `<expr>.await` desugars into a `Match` + generator `Loop`, both compiler-generated |

Each drop came from removing a construct that is not nesting the source
shows. The last one was the largest: every `await` inside a nested
expression added a level, and async code is nested code.

Top files:

| File | Hits |
|---|---|
| crates/ui-coding/src/state/store.rs | 7 |
| crates/harness-tools/src/semsearch.rs | 5 |
| crates/gateway-forward/src/pipeline.rs | 4 |
| crates/harness-llm/src/openai.rs | 3 |
| crates/gateway-protocol-bridge/src/gemini.rs | 3 |
| crates/gateway-metering/src/scanner.rs | 3 |
| crates/gateway-metering/src/estimate.rs | 3 |

Spot check of `semsearch/rank.rs:750` confirms real nesting:
`while` -> `if` -> `for` -> `if` -> `if` = 5 levels.

Done so far: `semsearch/rank.rs` (11 hits) — PR #450, split into
`keyword_hit_at` + `skip_qualifiers`; a differential test ran the old and
the new implementation over 20k generated inputs plus 32 hand-written edge
cases and both agreed everywhere.

Next candidates: `ui-coding/src/state/store.rs` (7),
`harness-tools/src/semsearch.rs` (5),
`gateway-forward/src/pipeline.rs` (4),
`harness-llm/src/openai.rs` (3).

## early-return-needs-no-else (2026-10-21)

Whole-workspace scan: **1 hit** —
`crates/gateway-metering/src/estimate.rs:80`. The rule originally also fired
on an `else if` chain (a false positive); fixed in commit 29cbd44, and the
remaining hit is a genuine `if c == '"' { break; } else { .. }`.

## ferrite CI: overview_wire_contract flake (2026-10-21)

- Symptom: PR #441 shell-ci test fails repeatedly at
  tests/overview_wire_contract.rs dashboard_flat_envelope_fields_and_unit_deltas
  (channels/users/groups delta assertions, left=2 right=1).
- Root cause: test relies on SERIAL isolation within one process, but the CI
  postgres instance is shared across concurrent workflow runs; another job's
  writes land inside the delta window. Not related to PR #441's diff
  (unwrap->expect in 17 unrelated files; lint+app-shell pass).
- Status: environmental; needs either CI serialization of this test or a
  per-run database.
- Retried 8+ times over ~2h after cancelling a stuck run (39484760989); every
  retry fails at this same test while lint+app-shell pass. PR 440 passed in the
  same window, confirming the flake is traffic-dependent, not diff-dependent.

## ban-unwrap (2026-10-21, final)

Full-workspace scan on ferrite main:

```text
cargo +nightly-2026-08-20 dylint --path /home/hathaway/projects/rust-specs \
  --pattern 'lints/*' --all --workspace
```

Result: **0 hits** for `ban-unwrap` (bare unwrap + non-English expect) across all
53 packages. The last 4 hits (all in `ui-feature-component`) went to PR #448.

Note: the scan needs `--workspace`; `cargo dylint` rejects multiple `-p` flags.
`ui-components` must compile on the lint toolchain (nightly-2026-08-20) or the
whole scan aborts — PR #448 fixed a raw identifier inside an rsx format string
that broke exactly that.

## max-nesting-depth (2026-10-22, while double-count fixed)

A rule defect was found and fixed: `visit_while_body` only special-cased
`while let`; a normal `while cond` desugars to `Loop { stmts: [], expr:
If(cond, body, break) }`, and the desugared `If` was counted as a user level,
making every `while` contribute 2 levels instead of 1 (contradicting the
README). 8 of the 39 hits from the 2026-10-21 scan were false positives
(`semsearch.rs` 5, `memory/recall.rs` 2, `web/parse.rs` 1); the remaining 31
were real and were refactored in ferrite PR #457.

Fix: `visit_while_body` now walks the `then` branch of `block.expr` when it is
an `If` (same path for `while let` and `while cond`), and a regression case
(`while` + 3 `if`s = visual depth 4, no report) was added to the ui test.

Re-scan on ferrite main: **0 hits** for `max-nesting-depth`.

## expect-in-non-result (2026-10-22, first scan)

New rule: `.expect()` on `Result`/`Option` inside a function whose return type
is not `Result`/`Option` (i.e. `?` is unavailable). Closures are skipped.

Whole-workspace scan on ferrite main:

**74 hits.** Top files:

| File | Hits |
|---|---|
| crates/coding-harness-subagent/src/runtime.rs | 10 |
| crates/coding-harness-subagent/src/acp_provider.rs | 9 |
| crates/gateway-metering/src/ledger.rs | 6 |
| crates/harness-daemon/src/runtime/state.rs | 5 |
| crates/harness-tools/src/semsearch.rs | 4 |
| crates/harness-daemon/src/runtime/dispatch.rs | 3 |
| crates/gateway-proxy/src/manager.rs | 3 |
| crates/coding-harness-subagent/src/acp.rs | 3 |

`ban-unwrap`, `early-return-needs-no-else`, `consecutive-same-shape-loops`,
`max-nesting-depth`: **0 hits** in the same scan.

These are real hits (not yet refactored); each one is a candidate to make the
function return `Result`/`Option` and use `?`, or to add
`#[allow(expect_in_non_result)]` where returning `Result` is not appropriate.
