# Real-world findings (ferrite)

Scans produced by rust-lints rules against the ferrite workspace.

## max-nesting-depth (2026-10-21)

Scan: `cargo +nightly-2026-08-20 dylint --path ... --pattern 'lints/*' --all -- -p coding-harness-session`

| Location | Shape |
|---|---|
| crates/harness-compaction/src/controller.rs:389 | >4 nested control flow |
| crates/harness-compaction/src/reassembly.rs:336 | >4 nested control flow |
| crates/harness-compaction/src/reassembly.rs:353 | >4 nested control flow |
| crates/harness-compaction/src/reassembly.rs:355 | >4 nested control flow |
| crates/harness-agent-loop/src/hygiene/pattern.rs:21 | >4 nested control flow |
| crates/harness-agent-loop/src/hygiene/pattern.rs:25 | >4 nested control flow |
| crates/harness-agent-loop/src/hygiene/pattern.rs:26 | >4 nested control flow |
| crates/harness-agent-loop/src/hygiene/pattern.rs:29 | >4 nested control flow |
| crates/harness-agent-loop/src/runtime/compaction_bridge.rs:78 | >4 nested control flow |
| crates/harness-agent-loop/src/runtime/mod.rs:626 | >4 nested control flow |

Candidate for a future refactor PR: flatten with combinators / helper extraction.

## early-return-needs-no-else (2026-10-21)

Scan: same command. No hits in coding-harness-session closure; pattern absent.

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
