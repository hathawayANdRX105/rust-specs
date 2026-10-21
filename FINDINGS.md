# Real-world findings (ferrite)

Scans produced by rust-lints rules against the ferrite workspace.

## max-nesting-depth (2026-10-21, after the depth fixes)

Whole-workspace scan, with the rule counting *visual* nesting:

**89 hits.** The number came down from 274 as the rule was corrected:

| Version | Hits | What was wrong |
|---|---|---|
| first | 274 | counted the `for`/`while` HIR desugaring and the `else if` chain |
| for/else-if fixed | 96 | a `match` scrutinee and an `if` condition were counted as nested |
| scrutinee fixed | 58 | a `while` was never counted (it is the `Loop` node itself) |
| final | 89 | — (`while` now counts, so the number went up again) |

Top files:

| File | Hits |
|---|---|
| crates/harness-tools/src/semsearch/rank.rs | 11 |
| crates/ui-coding/src/state/store.rs | 10 |
| crates/gateway-metering/src/estimate.rs | 7 |
| crates/ui-coding/src/state/subscriptions.rs | 5 |
| crates/harness-tools/src/semsearch.rs | 5 |
| crates/gateway-forward/src/pipeline.rs | 5 |

Spot check of `semsearch/rank.rs:750` confirms real nesting:
`while` -> `if` -> `for` -> `if` -> `if` = 5 levels.

Candidate for a future refactor PR: flatten with combinators / helper
extraction, starting with `semsearch/rank.rs` and `ui-coding/src/state/store.rs`.

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
