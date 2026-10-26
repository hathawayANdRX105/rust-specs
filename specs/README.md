# Specs

本目录是 rust-specs 的规范库（`specs/`）：把原 canon 检查规则翻译成 agent 可以直接照做的检查清单与好坏例子。canon 已退役，规则正本统一在 rust-specs。

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
| `specs/github/pr.md` | 原 canon `github_pull_requests.yaml`、`github_pr_gates.yaml`（已迁移到本目录）与 ferrite `.github/PULL_REQUEST_TEMPLATE.md` |
| `specs/github/issue.md` | 原 canon `github_issues.yaml`（已迁移到本目录） |

## CI 集成状态（2026-10-22）

- **PR 检查已接入**：ferrite 壳仓 CI（ferrite-ci `.github/workflows/ci.yml` 的 `rust-specs-check` job）在 PR 派发时跑 `cargo xtask check github-pr`，回写 `rust-specs` commit status 到 PR 头提交，`gh pr checks` 可见。已在真实 PR（PR 31）上验证：`rust-specs: pass`。
- **Issue 检查未接入**：GitHub issue 没有 commit status 可写，且 ferrite 硬约束禁止新建 issue。如将来需要，可用 `issues: [opened]` workflow 在 issue 创建后跑 `cargo xtask check github-issue` 并把结果评论到 issue（无法像 PR 一样在创建时拦截）。
- **canon 拦截已删除**：canon 二进制与 gh shim 已从 ferrite 环境移除，`gh pr create`/`gh issue create` 不再被 canon 拦截；rust-specs 检查是唯一 PR/Issue 格式门禁。
