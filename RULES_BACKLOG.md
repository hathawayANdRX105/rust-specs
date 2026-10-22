# Rules backlog

Only rules that neither clippy nor canon's RUST-IDIOM can express belong here.

## Already covered elsewhere — do NOT implement

| Rule | Where |
|---|---|
| manual-map / manual-and-then / manual-unwrap-or / manual-map-or / manual-ok-or / manual-flatten / option-filter / vec-push-loop / if-else-chain / while-let-on-iterator / iter-index-loop / bool-comparison / redundant-pattern-matching / derive-able-impl / manual-let-else / needless-return / question-mark / hashmap-entry / collapsible-if / nested-if-let / unwrap-in-fallible(≈ban-unwrap) | clippy deny (PR #435) + canon RUST-IDIOM.ferrite |

## Candidates for this repo (need type info / project policy)

| Rule | Why dylint |
|---|---|
| ban-unwrap (DONE) | project policy: unwrap banned, expect must be English |
| expect-in-non-result (DONE) | detect `.expect` in non-Result fns where `?` is unavailable; type info needed |
| max-nesting-depth (DONE) | configurable threshold (e.g. 4), HIR block nesting |
| consecutive-same-shape-loops (DONE) | N loops with same shape → chain; syn can do but weak across fns |
| early-return-needs-no-else (DONE) | `if c { return .. } else { .. }` → drop else |

## Process

1. Pick a rule from Candidates.
2. `cargo dylint new lints/<name>`
3. Implement + ui test + README.
4. Run `cargo +nightly-2026-08-20 dylint --path <repo> --pattern 'lints/*' --all --workspace` in ferrite.
