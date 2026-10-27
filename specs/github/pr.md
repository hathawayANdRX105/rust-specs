# GitHub PR 规范

> 正本：原 canon `github_pull_requests.yaml`、`github_pr_gates.yaml`（已迁移到本目录）与 ferrite `.github/PULL_REQUEST_TEMPLATE.md`
> 适用：创建 PR、更新 PR、合并 PR 前自查。
> 规则 ID：PR-01 ~ PR-16（由 rust-specs 的 `cargo xtask check github-pr` 逐条检查；规则正本源自 canon 的 `github_pull_requests.yaml`），GT-* 合并门禁（`gh pr merge` 时拦截）。
> 自查命令：正文写好后先跑 `cd /home/hathaway/projects/rust-specs && cargo xtask check github-pr --title "<标题>" --body-file <正文>`（可选 `--labels`、`--branch`），再 `gh pr create`。

## 检查清单

### 1. 分支名（PR-08，FAIL）

- [ ] 分支名必须以白名单前缀开头：`feat/`、`fix/`、`chore/`、`refactor/`、`epic/`、`release/`，或恰为 `main` / `master`。
- 反例：`dev/...`、`feature/...`、`test/...`、`patch/...`、`docs/...` 均不在白名单，会被 FAIL 拦截。

### 2. 标题（PR-01 FAIL / PR-02 WARN）

- [ ] 标题必须**全英文**：含 CJK 字符即 FAIL（技术标识符不影响判断）。
- [ ] 推荐 conventional commit 前缀（不匹配只是 WARN，不硬拦）：`feat`、`fix`、`chore`、`docs`、`style`、`refactor`、`test`、`ci`、`build`、`perf`、`revert`。
- [ ] 标题禁止出现中文全角括号：`（` `）` `「` `」` `【` `】` `『` `』` `《` `》` `〈` `〉`。即使标题是英文，含这些字符也 FAIL。

### 3. 正文小节（PR-03 FAIL / PR-04 FAIL）

- [ ] 以下小节必须存在（建议按模板顺序，heading 一律英文）：

  `What` → `Why` → `Construction plan` → `Delivery record` → `How to test` → `Checklist`

- [ ] `What` 小节的正文必须含中文（PR-04）；heading 本身必须英文，禁止写 `## 背景` 这类中文标题。
- [ ] 模板中的 `## Issue`（`Fixes #N` / `Related #N`）为可选段，放在 `Why` 之后、`Construction plan` 之前。
- [ ] 只复制适用的额外区块（Screenshots / Risk / Changelog / Notes for reviewers），不要发布空的额外标题。

### 4. 勾选框（PR-07，FAIL）

- [ ] `Construction plan` 段至少 2 个 `- [ ]` 勾选框。
- [ ] `Checklist` 段至少 2 个 `- [ ]` 勾选框。
- 合并前 PR 内所有勾选框必须全部勾上（GT-CHK）。

### 5. type label（PR-06）

- [ ] PR 必须挂至少 1 个 type label，取值限：`bug`、`enhancement`、`feature`、`documentation`、`chore`、`refactor`、`tests`、`epic`。
- [ ] 标题/正文命中关键词但缺对应 label 时，检查会 WARN 建议补挂：

  | 关键词 | 建议 label |
  |---|---|
  | 修复 | `bug` |
  | 新功能 / 功能 / 功能添加 | `enhancement` |
  | 清理 | `chore` |
  | 优化 / 代码质量 | `refactor` |

### 6. Issue 关联（PR-05 INFO / PR-10）

- [ ] `Fixes #N` / `Closes #N` / `Resolves #N` **可选**：没有 issue 直接开 PR 是合法默认形态（INFO 提示，不拦截）。
- [ ] 一个 PR 只写一个 `Fixes`；多个 `Fixes` 恒为 WARN。
- [ ] `Related #N` / `Part of #N` 是纯文本引用，不产生 GitHub Development 关联（INFO）；`N` 必须是 **issue 号**。
- [ ] `Fixes #N` 的目标若是 epic（父 issue）会 WARN：epic 侧不显示 closing 关联，应改 Fixes sub-issue。

### 7. CI 状态（WARN）

- [ ] PR 的 CI（`shell-ci`）状态是 WARN 级检查：红了要修，不能静默合并。

### 8. 合并门禁（GT-*）

- [ ] 合并必须带 `--body` 说明原因（GT-BODY）。
- [ ] 只允许 squash merge（GT-METHOD）：`gh pr merge <N> --squash --delete-branch --body "..."`。
- [ ] squash 标题必须是 conventional commit（CM-01/02）。
- [ ] PR 内 checkbox 必须全勾（GT-CHK）。

## 好例子

> **标题**：`feat(gate): reject PR titles containing full-width brackets`
> **分支**：`feat/github-pr-title-gate`
> **type label**：`enhancement`
> **关联**：Fixes #123 · Related #100

~~~markdown
## What

合并后，`gh pr create` 会在 PR 标题包含中文全角括号（如 `（`、`）`、`「」`）时直接拦截并输出 PR-01 finding；同时本 PR 新增 `specs/github/pr.md`，把标题检查规则翻译成 agent 可照做的清单。

## Why

之前全角括号只能靠人工 review 发现，容易漏到 main；标题语言检查（PR-01）已经存在，但括号清单没有写进文档，agent 创建 PR 时不知道要避开。

## Issue

Fixes #123

Related #100

## Construction plan

- [ ] 确认 `github_pull_requests.yaml` 的 `forbidden_brackets_in_title` 清单覆盖全部全角括号
- [ ] 实现标题括号检查并接入 `gh pr create` 门禁
- [ ] 新增 `specs/github/pr.md` 并登记到 `specs/README.md`

## Delivery record

- Delivered: 标题括号检查已实现；`specs/github/pr.md` 已新增
- Verification: `cargo xtask check github-pr` 本地通过；坏例被拦截
- Follow-up: none

## How to test

1. 用含 `（` 的标题创建 PR，确认被拦截并给出 PR-01 finding
2. 用本 PR 的标题格式创建 PR，确认通过

```text
cd /home/hathaway/projects/rust-specs && cargo xtask check github-pr --title "feat(gate): reject PR titles containing full-width brackets" --body-file /tmp/pr-body.md
```

## Checklist

- [ ] 已使用 `Fixes #N` 关联主 Issue
- [ ] 已挂英文 type label（`enhancement`）
- [ ] 已执行上述验证命令并确认结果
- [ ] diff 只服务于关联的主 Issue
~~~

**为什么好：**

- 标题英文 + conventional 前缀 `feat:`，无全角括号 → PR-01 / PR-02 通过。
- 分支 `feat/` 在白名单 → PR-08 通过。
- 六个必填小节齐全、顺序正确，heading 全英文 → PR-03 通过。
- `What` 正文含中文 → PR-04 通过。
- Construction plan 3 个、Checklist 4 个勾选框（均 ≥ 2）→ PR-07 通过。
- 挂了 type label `enhancement` → PR-06 通过。
- 单个 `Fixes #123`（主 issue），`Related #100` 用 issue 号 → PR-05 / PR-10 通过。
- 合并前把全部勾选框勾上即可满足 GT-CHK。

> 模板还有 `## Review record` 收尾段（closeout hard rule）：审查结束后逐条登记发现与处置，并勾选「全部 BLOCK/WARN 已处置」等三项。

## 坏例子

> **标题**：`feat: 添加标题检查`（✗ 含中文 → PR-01 FAIL）
> **分支**：`dev/title-check`（✗ 不在白名单 → PR-08 FAIL）
> **type label**：未挂任何 label（✗ PR-06 FAIL）

~~~markdown
## What

Add title check.

## Why

We need it.

## Construction plan

- [ ] 实现检查

## Checklist

- [ ] 已测试
~~~

**为什么坏：**

| 问题 | 规则 |
|---|---|
| 标题含中文 `添加标题检查` | PR-01 FAIL（PR 标题必须英文） |
| 分支 `dev/` 不在白名单 | PR-08 FAIL |
| 缺 `Delivery record`、`How to test` 两个必填小节 | PR-03 FAIL |
| `What` 正文无中文 | PR-04 FAIL |
| Construction plan 只有 1 个勾选框 | PR-07 FAIL |
| Checklist 只有 1 个勾选框 | PR-07 FAIL |
| 未挂 type label | PR-06 FAIL |
