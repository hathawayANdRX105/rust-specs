# dioxus-rsx-list-key

Bans array-index list keys in rsx.

| | |
|---|---|
| Lint | `dioxus_rsx_list_key` |
| Default level | `Warn` |
| Parser | `syn` + `dioxus-rsx` 0.7 |

## What it catches

`key: "{index}"`, `key: "{i}"` or `key: "{idx}"` on an element/component inside
a `for` loop.

## Why

An index key makes Dioxus reuse the wrong node on reorder/insert/delete; row
state jumps to another row. Use a stable business id (`key: "{item.id}"`).
Index keys are only correct for static, never-reordered lists (tabs,
navigation).

## Known problems

Only the literal `{index}` / `{i}` / `{idx}` shapes are caught; a computed key
expression is not.

## CI usage

```sh
cargo +nightly-2026-08-20 dylint --path /home/hathaway/projects/rust-specs \
  --pattern 'lints/dioxus-rsx-list-key' --all --workspace
```
