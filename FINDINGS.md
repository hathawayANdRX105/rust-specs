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
