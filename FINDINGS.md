# Real-world findings (ferrite)

Scans produced by rust-lints rules against the ferrite workspace.

## max-nesting-depth (2026-10-21, after the depth fix)

Whole-workspace scan, after counting *visual* nesting (a `for`/`while` is one
level, an `else if` chain is one level):

**96 hits** (the first version of the rule reported 274 because it counted
the `for`/`while` HIR desugaring and the `else if` chain).

| File | Hits |
|---|---|
| crates/api-mcp/src/mcp_dispatch.rs | 19 |
| crates/ui-coding/src/state/store.rs | 8 |
| crates/ui-coding/src/state/subscriptions.rs | 7 |
| crates/gateway-forward/src/pipeline.rs | 6 |
| crates/harness-tools/src/semsearch/rank.rs | 4 |
| crates/api-mcp/src/mcp_service.rs | 4 |

The rest are spread over ~40 files with 1-3 hits each. Spot checks of the
survivors show real nesting (match arms inside match arms inside a `match`
result), so the number is actionable.

Candidate for a future refactor PR: flatten with combinators / helper
extraction, starting with `mcp_dispatch.rs`.

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
