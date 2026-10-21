# Real-world findings (ferrite)

Scans produced by rust-lints rules against the ferrite workspace.

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
cargo +nightly-2026-08-20 dylint --path /home/hathaway/projects/rust-lints \
  --pattern 'lints/*' --all --workspace
```

Result: **0 hits** for `ban-unwrap` (bare unwrap + non-English expect) across all
53 packages. The last 4 hits (all in `ui-feature-component`) went to PR #448.

Note: the scan needs `--workspace`; `cargo dylint` rejects multiple `-p` flags.
`ui-components` must compile on the lint toolchain (nightly-2026-08-20) or the
whole scan aborts — PR #448 fixed a raw identifier inside an rsx format string
that broke exactly that.
