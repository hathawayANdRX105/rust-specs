# Rules backlog

Only rules that neither clippy nor canon's RUST-IDIOM can express belong here.

## Already covered elsewhere — do NOT implement

| Rule                                                                                                                                                                                                                                                                                                                                                                                               | Where                                            |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| manual-map / manual-and-then / manual-unwrap-or / manual-map-or / manual-ok-or / manual-flatten / option-filter / vec-push-loop / if-else-chain / while-let-on-iterator / iter-index-loop / bool-comparison / redundant-pattern-matching / derive-able-impl / manual-let-else / needless-return / question-mark / hashmap-entry / collapsible-if / nested-if-let / unwrap-in-fallible(≈ban-unwrap) | clippy deny (PR #435) + canon RUST-IDIOM.ferrite |
| if_then_some_else_none | clippy **restriction** (opt-in, `Option` only); this repo's `bool-if-else-to-combinator` is Warn by default, type-gated, also covers `Ok`/`Err` (as `then_some(..).ok_or(..)`, since `bool::ok_or` drops the `Ok` value) |

## Candidates for this repo (need type info / project policy)

| Rule                                | Why dylint                                                                                                                                                                                                                                                                                           |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ban-unwrap (DONE)                   | project policy: unwrap banned, expect must be English                                                                                                                                                                                                                                                |
| expect-in-non-result (DONE)         | detect `.expect` in non-Result fns where `?` is unavailable; type info needed                                                                                                                                                                                                                        |
| max-nesting-depth (DONE)            | configurable threshold (e.g. 4), HIR block nesting                                                                                                                                                                                                                                                   |
| consecutive-same-shape-loops (DONE) | N loops with same shape → chain; syn can do but weak across fns                                                                                                                                                                                                                                      |
| early-return-needs-no-else (DONE)   | `if c { return .. } else { .. }` → drop else                                                                                                                                                                                                                                                         |
| test_placement (DONE)               | test code must live in `tests/`, not in `src/`; file-path based, no type info needed                                                                                                                                                                                                                 |
| is-check-needs-chain (DONE)         | lone `is_some`/`is_none`/`is_ok`/`is_err` on Option/Result feeding another Option/Result → combinator chain; bool→Option→plain-value chains deliberately NOT reported (sanctioned)                                                                                                                   |
| cjk-in-code (DONE)                  | Chinese in comments, string literals and identifiers outside test code; comments are never exempt, literals are exempt in test scope, i18n/data paths and rsx `data-*` attributes; whole-repo source-text scan (a small lexer + `syn` for the exemptions), rsx copy stays with `dioxus-rsx-cjk-text` |
| bool-if-else-to-combinator (DONE)   | bool `if`/`else` whose branches only wrap a value (`Some`/`None`, `Ok`/`Err`) → `then_some`/`then` (Option) and `then_some(..).ok_or(..)` (Result); eager payload only, lazy payload keeps `then`/`ok_or_else`                                                                                          |

## Process

1. Pick a rule from Candidates.
2. `cargo dylint new lints/<name>`
3. Implement + ui test + README.
4. Run `cargo +nightly-2026-08-20 dylint --path <repo> --pattern 'lints/*' --all --workspace` in ferrite.
