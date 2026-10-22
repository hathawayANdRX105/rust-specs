# Specs

本目录是 rust-specs 的规范库（`specs/`）：把 ferrite 仓库 `.githooks/spec/` 下的 canon 检查规则翻译成 agent 可以直接照做的检查清单与好坏例子。

仓库定位：

- `lints/`（dylint）管语法类规则；
- xtask / 特殊检查管 GitHub issue、PR 格式等流程约束；
- `specs/` 是这些检查的规范文档，供 agent 创建 GitHub issue / PR 时逐条自查。

## 目录

### GitHub

| 文档 | 用途 |
|---|---|
| [github/pr.md](github/pr.md) | PR 规范：标题语言与括号、正文小节、勾选框数量、type label、分支前缀、Fixes/Related 关联的检查清单与好坏例子。 |
| [github/issue.md](github/issue.md) | Issue 规范：标题语言与括号、正文小节、Done when、禁用内容、type label 建议的检查清单与好坏例子。 |

## 规范来源

| 文档 | 正本 |
|---|---|
| `specs/github/pr.md` | ferrite `.githooks/spec/github/github_pull_requests.yaml`、`github_pr_gates.yaml`、`.github/PULL_REQUEST_TEMPLATE.md` |
| `specs/github/issue.md` | ferrite `.githooks/spec/github/github_issues.yaml` |
