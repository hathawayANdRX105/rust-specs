# agents

Assembles per-project `AGENTS.md` documents from TOML profiles. This is the
replacement for the canon `scripts/agents` generator: rust-specs owns the
profiles, the profiles are plain TOML, and the output is plain markdown.

A profile declares **chapters** (top-level sections) made of **items**
(small, individually addressable points). Rendering is free assembly:

```sh
cargo agents list                                     # profiles + their points
cargo agents build ferrite                            # render everything
cargo agents build ferrite --chapter code,testing     # two chapters only
cargo agents build ferrite --drop code.lsp            # everything minus one point
cargo agents build ferrite --item code.lsp,code.rust-specs   # only these points
cargo agents build ferrite --out /tmp/AGENTS.md       # render to a file
cargo agents push ferrite                             # write the target file (stamped)
cargo agents push ferrite --force                     # overwrite a hand-edited target
cargo agents check ferrite                            # drift check (exit 1 = stale)
```

Item keys are `chapter.item`. A bare `item` id works when that id is unique
across the profile; ambiguous bare keys are rejected.

## Profile shape (`agents/profiles/<name>.toml`)

```toml
[profile]
name = "ferrite"        # must match the file name
title = "ferrite conventions"
repo = "ferrite"        # repo root: absolute, ~/…, or relative to AGENTS_PROJECTS_BASE (default ~/projects)
file = "AGENTS.md"      # target file relative to the repo root
include_chapters = []   # empty = every chapter, in declared order
exclude_items = []      # points to always drop ("chapter.item" keys)

[profile.preamble]
text = 'intro paragraph rendered under the title'

[[profile.preamble.table]]
headers = ["Purpose", "Command"]
rows = [["Tests", "`just test-fast`"]]

[[chapter]]
id = "code"
title = "Write Rust code"
summary = 'optional sentence under the chapter heading'

[[chapter.item]]
id = "chain-first"
title = "Prefer chained expressions"      # optional; renders as ###
text = 'a markdown paragraph'
bullets = ["a `- ` bullet", "another one"]
numbered = ["a `1. ` step", "the next step"]
code.lang = "bash"
code.text = 'just verify'
[[chapter.item.table]]
headers = ["Question", "Operation"]
rows = [["Who breaks if I change it", "`findReferences`"]]
```

Every field of an item except `id` is optional; blocks render in the order
text → numbered → bullets → code → tables.

## Commands

| Command | Effect |
|---|---|
| `cargo agents list` | list profiles, chapters, points, and targets |
| `cargo agents build <profile>` | print the generated document |
| `cargo agents push <profile>` | write `<repo>/<file>` with a managed-by stamp; refuses to overwrite hand-edited files without `--force` |
| `cargo agents check <profile>` | exit 0 when the target matches the generated document, exit 1 on drift |

`AGENTS_PROFILE_DIR` overrides the profile directory (used by tests).

## Adding a project

1. Copy `profiles/ferrite.toml` to `profiles/<project>.toml` and set
   `[profile] name / title / repo / file`.
2. Keep or trim the six chapters; every item is optional and selectable.
3. `cargo agents build <project>` to review, `cargo agents check <project>`
   to verify, `cargo agents push <project>` to land.
