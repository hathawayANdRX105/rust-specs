//! spec_rules — 从 canon 迁移的结构规则实现。
//!
//! 实现 ferrite root `checks.toml` 中的结构类规则，供门禁 CLI 调用。
//! 每条规则返回 `Vec<Finding>`；严重度由调用方按 checks.toml 覆盖链裁决。

use globset::{GlobBuilder, GlobSetBuilder};
use regex::Regex;
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::util::{Finding, Severity};

// ---------------------------------------------------------------------------
// 路径 glob 匹配（对应 canon check-core kinds: path-forbidden / text-pattern /
// name-pattern / duplicate-file 的路径作用域）
// ---------------------------------------------------------------------------

/// 把一组 glob 模式编译成 GlobSet。出错返回 Err。
fn compile_globs(patterns: &[String]) -> Result<globset::GlobSet, String> {
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        let glob = GlobBuilder::new(p)
            .literal_separator(false)
            .build()
            .map_err(|e| format!("glob build failed for {p:?}: {e}"))?;
        builder.add(glob);
    }
    builder
        .build()
        .map_err(|e| format!("globset build failed: {e}"))
}

/// 相对路径是否命中任一 include glob 且未命中任何 exclude glob。
fn path_matches(rel: &str, include: &[String], exclude: &[String]) -> bool {
    let Ok(include_set) = compile_globs(include) else { return false };
    let Ok(exclude_set) = compile_globs(exclude) else { return false };
    if !include_set.is_match(rel) {
        return false;
    }
    exclude_set.is_match(rel) == false
}

// ---------------------------------------------------------------------------
// Finding 辅助
// ---------------------------------------------------------------------------

fn finding(rule: &str, severity: Severity, message: impl Into<String>) -> Finding {
    Finding::new(rule, severity, message.into())
}

// ---------------------------------------------------------------------------
// 1. path-forbidden：命中的跟踪路径存在即违规（全仓不变量）
// ---------------------------------------------------------------------------

/// path-forbidden 参数（与 checks.toml `[kinds.path-forbidden.*]` 对齐）。
#[derive(Debug, Deserialize)]
pub struct PathForbiddenParams {
    pub paths: Vec<String>,
    pub message: Option<String>,
}

/// 单条 path-forbidden 规则实例（id 由实例名提供，如 `path-forbidden.pages-own-ui`）。
pub struct PathForbiddenRule {
    pub id: String,
    pub glob_set: globset::GlobSet,
    pub message: String,
    pub default_severity: Severity,
}

impl PathForbiddenRule {
    pub fn new(id: String, params: &PathForbiddenParams, default_severity: Severity) -> Result<Self, String> {
        let glob_set = compile_globs(&params.paths)?;
        Ok(Self {
            id,
            glob_set,
            message: params.message.clone().unwrap_or_else(|| "path-forbidden violation".into()),
            default_severity,
        })
    }

    /// 检查一组跟踪文件的相对路径，命中即返回 finding。
    pub fn check_files(&self, tracked_files: &[PathBuf]) -> Vec<Finding> {
        let mut findings = Vec::new();
        for f in tracked_files {
            let rel = f.to_string_lossy().replace('\\', "/");
            if self.glob_set.is_match(&rel) {
                findings.push(finding(
                    &self.id,
                    self.default_severity,
                    format!("{}: {}", self.message, rel),
                ));
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// 2. text-pattern：逐行匹配拒绝正则，patterns_allow 同一行豁免
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct TextPatternParams {
    pub paths_include: Vec<String>,
    pub paths_exclude: Vec<String>,
    pub patterns_deny: Vec<String>,
    pub patterns_allow: Vec<String>,
    pub message: Option<String>,
}

pub struct TextPatternRule {
    pub id: String,
    pub deny_regexes: Vec<Regex>,
    pub allow_regexes: Vec<Regex>,
    pub paths_include: Vec<String>,
    pub paths_exclude: Vec<String>,
    pub message: String,
    pub default_severity: Severity,
}

impl TextPatternRule {
    pub fn new(id: String, params: &TextPatternParams, default_severity: Severity) -> Result<Self, String> {
        let mut deny_regexes = Vec::new();
        for p in &params.patterns_deny {
            deny_regexes.push(Regex::new(p).map_err(|e| format!("bad deny regex {p:?}: {e}"))?);
        }
        let mut allow_regexes = Vec::new();
        for p in &params.patterns_allow {
            allow_regexes.push(Regex::new(p).map_err(|e| format!("bad allow regex {p:?}: {e}"))?);
        }
        Ok(Self {
            id,
            deny_regexes,
            allow_regexes,
            paths_include: params.paths_include.clone(),
            paths_exclude: params.paths_exclude.clone(),
            message: params.message.clone().unwrap_or_else(|| "text-pattern violation".into()),
            default_severity,
        })
    }

    /// 检查单个文件：逐行匹配 deny 正则，allow 命中同一行则豁免。
    pub fn check_file(&self, path: &Path, content: &str) -> Vec<Finding> {
        let rel = path.to_string_lossy().replace('\\', "/");
        if !path_matches(&rel, &self.paths_include, &self.paths_exclude) {
            return Vec::new();
        }
        let mut findings = Vec::new();
        for (lineno, line) in content.lines().enumerate() {
            let denied = self.deny_regexes.iter().any(|r| r.is_match(line));
            if !denied {
                continue;
            }
            let exempted = self.allow_regexes.iter().any(|r| r.is_match(line));
            if exempted {
                continue;
            }
            findings.push(finding(
                &self.id,
                self.default_severity,
                format!("{}: {}:{}: {}", self.message, rel, lineno + 1, line.trim()),
            ));
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// 3. name-pattern：跟踪文件文件名必须匹配 pattern
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct NamePatternParams {
    pub paths_include: Vec<String>,
    pub pattern: String,
    pub message: Option<String>,
}

pub struct NamePatternRule {
    pub id: String,
    pub pattern_re: Regex,
    pub paths_include: Vec<String>,
    pub message: String,
    pub default_severity: Severity,
}

impl NamePatternRule {
    pub fn new(id: String, params: &NamePatternParams, default_severity: Severity) -> Result<Self, String> {
        // 验证 pattern 能编译为正则（checks.toml 的 name-pattern 用 regex 语法）
        let pattern_re = Regex::new(&params.pattern)
            .map_err(|e| format!("bad name-pattern regex {:?}: {e}", params.pattern))?;
        Ok(Self {
            id,
            pattern_re,
            paths_include: params.paths_include.clone(),
            message: params.message.clone().unwrap_or_else(|| "name-pattern violation".into()),
            default_severity,
        })
    }

    pub fn check_files(&self, tracked_files: &[PathBuf]) -> Vec<Finding> {
        let mut findings = Vec::new();
        for f in tracked_files {
            let rel = f.to_string_lossy().replace('\\', "/");
            if !path_matches(&rel, &self.paths_include, &[]) {
                continue;
            }
            let filename = f.file_name().and_then(|n| n.to_str()).unwrap_or("");
            // 全匹配：pattern 已自带 ^...$ 锚点；is_match 做未锚定搜索，配合锚点等价全匹配
            if !self.pattern_re.is_match(filename) {
                findings.push(finding(
                    &self.id,
                    self.default_severity,
                    format!("{}: {} (expected {})", self.message, rel, self.pattern_re.as_str()),
                ));
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// 4. duplicate-file：Jaccard 相似度 ≥ 阈值 → advisory
// ---------------------------------------------------------------------------

const DEFAULT_STOPWORDS: &[&str] = &[
    "self", "Self", "super", "crate", "use", "mod", "pub", "fn", "struct",
    "enum", "trait", "impl", "let", "mut", "const", "static", "if", "else",
    "match", "while", "for", "in", "return", "break", "continue", "loop",
    "where", "as", "&", "*", "::", "<", ">", ",", ";", ":", "{", "}", "[",
    "]", "(", ")", "=", "+", "-", "!", "?", ".", "\"", "'",
];

#[derive(Debug, Deserialize)]
pub struct DuplicateFileParams {
    pub paths_include: Vec<String>,
    pub paths_exclude: Vec<String>,
    pub min_tokens: Option<usize>,
    pub min_similarity: Option<f64>,
    pub limit: Option<usize>,
    pub message: Option<String>,
}

pub struct DuplicateFileRule {
    pub id: String,
    pub paths_include: Vec<String>,
    pub paths_exclude: Vec<String>,
    pub min_tokens: usize,
    pub min_similarity: f64,
    pub limit: usize,
    pub message: String,
    pub default_severity: Severity,
}

impl DuplicateFileRule {
    pub fn new(id: String, params: &DuplicateFileParams, default_severity: Severity) -> Result<Self, String> {
        Ok(Self {
            id,
            paths_include: params.paths_include.clone(),
            paths_exclude: params.paths_exclude.clone(),
            min_tokens: params.min_tokens.unwrap_or(80),
            min_similarity: params.min_similarity.unwrap_or(0.85),
            limit: params.limit.unwrap_or(5),
            message: params.message.clone().unwrap_or_else(|| "duplicate-file advisory".into()),
            default_severity,
        })
    }

    /// 将文件内容分词为 token 集合（去掉停用词后）。
    fn tokenize(content: &str) -> BTreeSet<String> {
        let mut set = BTreeSet::new();
        for token in content.split(|c: char| !c.is_alphanumeric() && c != '_') {
            if token.is_empty() {
                continue;
            }
            if DEFAULT_STOPWORDS.contains(&token) {
                continue;
            }
            set.insert(token.to_string());
        }
        set
    }

    fn jaccard(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
        if a.is_empty() && b.is_empty() {
            return 0.0;
        }
        let inter = a.intersection(b).count();
        let union = a.union(b).count();
        inter as f64 / union as f64
    }

    /// 对变更文件（changed）与全仓既有文件（existing）做两两相似度比较，
    /// 返回超过阈值的 advisory findings（限 `limit` 条）。
    pub fn check(
        &self,
        changed: &[(PathBuf, String)], // (path, content)
        existing: &[(PathBuf, String)],
    ) -> Vec<Finding> {
        let mut scored: Vec<(f64, PathBuf, PathBuf)> = Vec::new();

        for (cpath, ccontent) in changed {
            let crel = cpath.to_string_lossy().replace('\\', "/");
            if !path_matches(&crel, &self.paths_include, &self.paths_exclude) {
                continue;
            }
            let ctokens = Self::tokenize(ccontent);
            if ctokens.len() < self.min_tokens {
                continue;
            }
            for (epath, econtent) in existing {
                let erel = epath.to_string_lossy().replace('\\', "/");
                if !path_matches(&erel, &self.paths_include, &self.paths_exclude) {
                    continue;
                }
                if cpath == epath {
                    continue;
                }
                let etokens = Self::tokenize(econtent);
                if etokens.len() < self.min_tokens {
                    continue;
                }
                let sim = Self::jaccard(&ctokens, &etokens);
                if sim >= self.min_similarity {
                    scored.push((sim, cpath.clone(), epath.clone()));
                }
            }
        }

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(self.limit);

        let mut findings = Vec::new();
        for (sim, cpath, epath) in scored {
            findings.push(finding(
                &self.id,
                self.default_severity,
                format!(
                    "{}: {} 与 {} 相似度 {:.2} ≥ {}，考虑复用（或确认独立实现）",
                    self.message,
                    cpath.to_string_lossy(),
                    epath.to_string_lossy(),
                    sim,
                    self.min_similarity
                ),
            ));
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// 依赖图构建（对应 canon check-deps metadata.rs：cargo metadata --no-deps
// 最小建模，解析包名→相对目录与 normal path 依赖边）
// ---------------------------------------------------------------------------

/// cargo metadata 中一个包的简化信息。
#[derive(Debug, Deserialize)]
struct MetaPackage {
    name: String,
    manifest_path: String,
    dependencies: Vec<MetaDep>,
}

/// cargo metadata 中一个依赖声明（untagged：normal edge 为字符串名，dev/build 为对象）。
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum MetaDep {
    Str(String),
    Obj(MetaDepObj),
}

/// 依赖对象（normal edge kind 为 null）。
#[derive(Debug, Deserialize)]
struct MetaDepObj {
    name: String,
    #[serde(default)]
    kind: Option<String>,
}

/// cargo metadata 根对象（只取需要的字段）。
#[derive(Debug, Deserialize)]
struct MetaRoot {
    packages: Vec<MetaPackage>,
}

/// 依赖图：包名 → 相对目录，以及包名 → 被依赖包名集合。
#[derive(Debug, Default)]
pub struct DepGraph {
    pub dir_of: HashMap<String, String>,
    pub edges: HashMap<String, HashSet<String>>,
}

/// 运行 `cargo metadata --no-deps --format-version 1` 并构建依赖图。
pub fn load_dep_graph(root: &Path) -> Option<DepGraph> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let root_meta: MetaRoot = serde_json::from_slice(&output.stdout).ok()?;
    let mut graph = DepGraph::default();
    for pkg in &root_meta.packages {
        let dir = rel_dir(root, &pkg.manifest_path)?;
        graph.dir_of.insert(pkg.name.clone(), dir);
        graph.edges.entry(pkg.name.clone()).or_default();
    }
    for pkg in &root_meta.packages {
        for dep in &pkg.dependencies {
            let dep_name = match dep {
                MetaDep::Str(name) => name,
                MetaDep::Obj(obj) if obj.kind.is_none() => &obj.name,
                _ => continue, // dev/build deps are not normal dependency edges
            };
            if let Some(dst_dir) = graph.dir_of.get(dep_name) {
                let _dst_dir = dst_dir;
                graph
                    .edges
                    .entry(pkg.name.clone())
                    .or_default()
                    .insert(dep_name.clone());
            }
        }
    }
    Some(graph)
}

/// Cargo.toml 绝对路径 → 相对仓库根的目录串；仓库外返回 None。
fn rel_dir(root: &Path, manifest_path: &str) -> Option<String> {
    let mpath = Path::new(manifest_path);
    if !mpath.is_absolute() {
        return None;
    }
    let stripped = mpath.strip_prefix(root).ok()?;
    let parent = stripped.parent()?;
    Some(parent.to_string_lossy().replace('\\', "/"))
}

// ---------------------------------------------------------------------------
// 5. dep-edge：cargo metadata 路径依赖边的 glob deny 清单（三层依赖方向）
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct DepEdgeParams {
    pub edges_deny: Vec<DepEdgeDeny>,
}

#[derive(Debug, Deserialize)]
pub struct DepEdgeDeny {
    pub from: String,
    pub to: String,
    pub message: Option<String>,
}

pub struct DepEdgeRule {
    pub id: String,
    pub compiled: Vec<(globset::GlobSet, globset::GlobSet, String)>,
    pub default_severity: Severity,
}

impl DepEdgeRule {
    pub fn new(id: String, params: &DepEdgeParams, default_severity: Severity) -> Result<Self, String> {
        let mut compiled = Vec::new();
        for e in &params.edges_deny {
            let from_gs = compile_globs(&[e.from.clone()])?;
            let to_gs = compile_globs(&[e.to.clone()])?;
            compiled.push((from_gs, to_gs, e.message.clone().unwrap_or_default()));
        }
        Ok(Self {
            id,
            compiled,
            default_severity,
        })
    }

    /// 检查依赖图：对每条 normal path 依赖边 (src -> dst)，判断其包目录是否
    /// 命中任一 (from -> to) deny 边。
    pub fn check(&self, graph: &DepGraph) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (src_name, dsts) in &graph.edges {
            let src_dir = match graph.dir_of.get(src_name) {
                Some(d) => d,
                None => continue,
            };
            for dst_name in dsts {
                let dst_dir = match graph.dir_of.get(dst_name) {
                    Some(d) => d,
                    None => continue,
                };
                for (from_gs, to_gs, msg) in &self.compiled {
                    if from_gs.is_match(src_dir) && to_gs.is_match(dst_dir) {
                        findings.push(finding(
                            &self.id,
                            self.default_severity,
                            format!(
                                "dep-edge violation: {} ({}) depends on {} ({}){}",
                                src_name,
                                src_dir,
                                dst_name,
                                dst_dir,
                                if msg.is_empty() {
                                    String::new()
                                } else {
                                    format!(" — {msg}")
                                }
                            ),
                        ));
                        break; // 一条边只报一次
                    }
                }
            }
        }
        findings
    }
}

// ---------------------------------------------------------------------------
// 6. cycles：依赖图无环（硬伤，Fail）
// ---------------------------------------------------------------------------

pub struct CyclesRule {
    pub id: String,
    pub default_severity: Severity,
}

impl CyclesRule {
    pub fn new(id: String, default_severity: Severity) -> Self {
        Self { id, default_severity }
    }

    /// DFS 检测有向图环，返回环中涉及的包名链。
    pub fn check(&self, graph: &DepGraph) -> Vec<Finding> {
        let mut findings = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut rec_stack: HashSet<String> = HashSet::new();
        let mut path: Vec<String> = Vec::new();

        for start in graph.edges.keys() {
            if visited.contains(start) {
                continue;
            }
            if let Some(cycle) = Self::dfs(start, graph, &mut visited, &mut rec_stack, &mut path) {
                findings.push(finding(
                    &self.id,
                    self.default_severity,
                    format!("dependency cycle: {}", cycle.join(" → ")),
                ));
            }
        }
        findings
    }

    fn dfs(
        node: &str,
        graph: &DepGraph,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(neighbors) = graph.edges.get(node) {
            for nb in neighbors {
                if rec_stack.contains(nb) {
                    let start_idx = path.iter().position(|p| p == nb).unwrap_or(0);
                    let mut cycle: Vec<String> = path[start_idx..].to_vec();
                    cycle.push(nb.to_string());
                    return Some(cycle);
                }
                if !visited.contains(nb) {
                    if let Some(cycle) = Self::dfs(nb, graph, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
        None
    }
}

// ---------------------------------------------------------------------------
// 7. css-var-mirror：`.dark` 段必须是 `:root` 块的同 token 镜像
// ---------------------------------------------------------------------------

/// css-var-mirror 参数（与 checks.toml `[kinds.css-var-mirror.*]` 对齐）。
#[derive(Debug, Deserialize)]
pub struct CssVarMirrorParams {
    pub paths_include: Vec<String>,
    pub message: Option<String>,
}

/// 检查 theme.css 的 `.dark` 块是否镜像 `:root` 块的 CSS 变量 token 集合。
pub struct CssVarMirrorRule {
    pub id: String,
    pub paths_include: Vec<String>,
    pub message: String,
    pub default_severity: Severity,
}

impl CssVarMirrorRule {
    pub fn new(id: String, params: &CssVarMirrorParams, default_severity: Severity) -> Result<Self, String> {
        if params.paths_include.is_empty() {
            return Err("css-var-mirror requires at least one paths_include glob".into());
        }
        Ok(Self {
            id,
            paths_include: params.paths_include.clone(),
            message: params.message.clone().unwrap_or_else(|| "css-var-mirror violation".into()),
            default_severity,
        })
    }

    /// 检查单个 CSS 文件：`:root` 与 `.dark` 块的 token 集合必须一致。
    pub fn check_file(&self, path: &Path, content: &str) -> Vec<Finding> {
        let rel = path.to_string_lossy().replace('\\', "/");
        if !path_matches(&rel, &self.paths_include, &[]) {
            return Vec::new();
        }

        let (root_tokens, dark_tokens) = extract_theme_blocks(content);
        let mut findings = Vec::new();

        let missing: Vec<&str> = root_tokens.difference(&dark_tokens).map(|s| s.as_str()).collect();
        for token in missing {
            findings.push(finding(
                &self.id,
                self.default_severity,
                format!("{}: `.dark` 段缺少 :root 的 token `--{token}`", self.message),
            ));
        }

        let extra: Vec<&str> = dark_tokens.difference(&root_tokens).map(|s| s.as_str()).collect();
        for token in extra {
            findings.push(finding(
                &self.id,
                self.default_severity,
                format!("{}: `.dark` 段包含 :root 没有的 token `--{token}`", self.message),
            ));
        }

        findings
    }
}

/// 去掉 CSS 注释（`/* ... */`），返回清理后的文本。
fn strip_css_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let bytes = css.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i += 2; // skip closing '*/' (or past end)
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

/// 提取 `:root` 与 `.dark` 规则块中的 `--token` 集合。
fn extract_theme_blocks(css: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let css = strip_css_comments(css);
    let token_re = Regex::new(r"--([a-zA-Z0-9_-]+)\s*:").expect("token regex");
    let mut root_tokens = BTreeSet::new();
    let mut dark_tokens = BTreeSet::new();

    // 逐字符扫描顶层规则块：选择器文本到 '{'，块内容到匹配的 '}'。
    let bytes = css.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // 跳过字符串字面量（"..." 与 '...'），避免选择器/值里的花括号干扰。
        match bytes[i] {
            b'"' | b'\'' => {
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                i += 1;
                continue;
            }
            b'{' => {
                // 找到块起点，回溯到上一个分号/右括号/行首得到选择器。
                let selector_start = css[..i].rfind(|c: char| c == ';' || c == '}' || c == '{')
                    .map(|p| p + 1)
                    .unwrap_or(0);
                let selector = css[selector_start..i].trim();
                let mut depth = 1usize;
                let block_start = i + 1;
                i += 1;
                while i < bytes.len() && depth > 0 {
                    match bytes[i] {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        _ => {}
                    }
                    i += 1;
                }
                let block = &css[block_start..i.saturating_sub(1)];
                let tokens: BTreeSet<String> = token_re
                    .captures_iter(block)
                    .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
                    .collect();
                if selector.contains(":root") {
                    root_tokens.extend(tokens);
                } else if selector.contains(".dark") {
                    dark_tokens.extend(tokens);
                }
                continue;
            }
            _ => i += 1,
        }
    }

    (root_tokens, dark_tokens)
}
