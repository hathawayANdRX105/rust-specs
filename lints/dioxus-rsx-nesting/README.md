# dioxus-rsx-nesting

Checks rsx element nesting depth.

| | |
|---|---|
| Lint | `dioxus_rsx_nesting` |
| Default level | `Warn` |
| Threshold | root = depth 1, one level of nesting allowed (depth 3+ reported) |
| Parser | `syn` + `dioxus-rsx` 0.7 |

## Why

Deeply nested rsx means one component renders a whole view. Extract the
subtree into a child component; one numbered section per component is the
minimum boundary. `for`/`if` bodies continue at the same depth, and a
component's children are a separate rsx (they do not add depth).

## Known problems

Source-text based; files that fail to parse are skipped.

## CI usage

```sh
cargo +nightly-2026-08-20 dylint --path /home/hathaway/projects/rust-specs \
  --pattern 'lints/dioxus-rsx-nesting' --all --workspace
```
