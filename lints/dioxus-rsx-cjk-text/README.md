# dioxus-rsx-cjk-text

Bans Chinese text inside rsx element bodies.

| | |
|---|---|
| Lint | `dioxus_rsx_cjk_text` |
| Default level | `Warn` |
| Parser | `syn` + `dioxus-rsx` 0.7 |

## Why

User-visible copy comes from the i18n table / `LBL_` constants. A literal in
rsx cannot be translated and shows up untranslated in the English locale.
`data-testid` values, numbers, symbols and units stay literal; comments are not
text nodes.

## Known problems

Only `BodyNode::Text` is inspected; attribute values are not text nodes.

## CI usage

```sh
cargo +nightly-2026-08-20 dylint --path /home/hathaway/projects/rust-specs \
  --pattern 'lints/dioxus-rsx-cjk-text' --all --workspace
```
