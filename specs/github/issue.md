# GitHub Issue 规范

> 正本：原 canon `github_issues.yaml`（已迁移到本目录）
> 适用：创建 issue / sub-issue、关闭 issue 前自查。
> 规则 ID：IS-00 ~ IS-16（由 rust-specs 的 `cargo xtask check github-issue` 逐条检查；规则正本源自 canon 的 `github_issues.yaml`）。

## 检查清单

### 1. 标题（IS-05 FAIL / IS-00 FAIL / IS-16 FAIL）

- [ ] 标题必须**中文**：非 ASCII 拼音/英文标题即 FAIL，但技术标识符（如 `dylint`、`PR-01`、`Cargo.toml`、`gh pr create`）可混入。
- [ ] 标题禁止中文全角括号：`（` `）` `「` `」` `【` `】` `『` `』` `《` `》` `〈` `〉`。
- [ ] 标题禁用前缀：`父`、`issue:`、`sub:`、`parent:`（父子关系用 label 表达，不写进标题）。

### 2. 正文小节（IS-01 FAIL / IS-06 FAIL / IS-07 FAIL）

- [ ] 以下小节必须存在（建议按模板顺序，heading 一律英文）：

  `Goal` → `Background` → `Done when` → `Suspected areas` → `Out of scope` → `How to observe success`

- [ ] heading 必须英文，禁止 `## 目标` 这类中文标题。
- [ ] 正文必须含中文。

### 3. Done when（IS-04 FAIL / IS-15 FAIL）

- [ ] `Done when` 段必须含至少 1 个 `- [ ]` 勾选框。
- [ ] 关闭 issue 时所有 `Done when` 勾选框必须全勾。

### 4. 禁用内容（IS-16 FAIL）

- [ ] 正文禁止关键词：`TODO`、`TBD`、`FIXME`、`XXX`。
- [ ] 禁止在正文写 `## Labels` 段（label 用 `gh label` 操作，不写进正文）。
- [ ] 正文禁止出现字面量 `\n`（反斜杠 + n）和 U+FFFD（`�`，乱码替换符）。

### 5. 交叉引用与占位（IS-09 FAIL / IS-10 FAIL / IS-11 FAIL）

- [ ] 禁止 `Parent:` / `Related:` 这类交叉引用。
- [ ] sub-issue 正文禁止出现 PR 占位符。
- [ ] parent issue 正文禁止出现 `Done when` 段。

### 6. type label（IS-14 WARN）

- [ ] 建议挂至少 1 个 type label，取值限：`bug`、`enhancement`、`feature`、`documentation`、`chore`、`refactor`、`tests`、`epic`。
- [ ] 标题/正文命中关键词但缺对应 label 时 WARN 建议补挂：

  | 关键词 | 建议 label |
  |---|---|
  | bug / bug 报告 | `bug` |
  | 重构 / 重构( | `refactor` |
  | 测试 / 测试代码 | `tests` |
  | 文档 / 文档清洁 | `documentation` |
  | feature / 新功能 | `enhancement` |
  | 清理 / cleanup | `chore` |
  | epic | `epic` |

### 7. sub-issue（IS-15）

- [ ] sub-issue 创建后必须挂到 `--parent` 指定的父 issue 下（`sub_issue_must_link_parent`）。

## 好例子

> **标题**：`PR 标题全角括号检查没有生效`
> **type label**：`bug`

~~~markdown
## Goal

让 `gh pr create` 在 PR 标题包含全角括号时给出 FAIL 拦截，并把括号清单写进 `specs/github/pr.md`，避免 agent 创建 PR 时踩坑。

## Background

目前标题语言检查（PR-01）已经接入 rust-specs，但 `forbidden_brackets_in_title` 清单没有对应文档；全角括号只能靠人工 review 发现，容易漏到 main。

## Done when

- [ ] 标题含 `（` 时 `gh pr create` 被拦截并输出 PR-01 finding
- [ ] `specs/github/pr.md` 已写明全角括号清单
- [ ] 用规范标题创建 PR 的回归验证通过

## Suspected areas

- `crates/gh_wrap` 的标题检查逻辑
- `github_pull_requests.yaml` 的 `forbidden_brackets_in_title`（规则已迁入 rust-specs）

## Out of scope

- 不修改正文语言检查（PR-04）
- 不做 review 评论格式检查（见 #103）

## How to observe success

1. 用含 `（` 的标题创建 PR，观察被拦截
2. 用规范标题创建 PR，观察通过
3. `cargo xtask check github-issue` 本地验证通过
~~~

**为什么好：**

- 标题中文，无全角括号，无禁用前缀 → IS-05 / IS-00 / IS-16 通过。
- 六个必填小节齐全、顺序正确，heading 全英文 → IS-01 / IS-06 通过。
- 正文含中文 → IS-07 通过。
- `Done when` 含 3 个 `- [ ]` 勾选框 → IS-04 通过。
- 无 TODO/TBD/FIXME/XXX、无 `Labels` 段、无 `\n`/U+FFFD → IS-16 通过。
- type label `bug` 与「bug 报告」场景对齐 → IS-14。

## 坏例子

> **标题**：`Fix title check`（✗ 非中文 → IS-05 FAIL）
> **type label**：未挂任何 label（✗ IS-14 WARN）

~~~markdown
## Goal

Add a title check.

## Background

The validator does not check titles.

## Done when

完成修复。

## TODO

- [ ] 实现检查

## Labels

bug、enhancement
~~~

**为什么坏：**

| 问题 | 规则 |
|---|---|
| 标题英文 `Fix title check` | IS-05 FAIL（标题必须中文） |
| 缺 `Suspected areas`、`Out of scope`、`How to observe success` | IS-01 FAIL（缺必填 heading） |
| 正文全英文无中文 | IS-07 FAIL |
| `Done when` 无 `- [ ]` 勾选框 | IS-04 FAIL |
| 出现 `TODO` 关键词 | IS-16 FAIL |
| 正文写 `## Labels` 段 | IS-16 FAIL（labels_section_forbidden） |
| 正文若含字面量 `\n` 或 U+FFFD（`�`）也会被拦 | IS-16 FAIL（乱码检查） |
