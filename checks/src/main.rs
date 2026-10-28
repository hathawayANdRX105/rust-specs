//! `checks` — xtask-style CLI validating GitHub PR / issue metadata and bodies
//! against the rust-specs spec rules (see the `specs/github/` docs in this
//! repo for the rule source of truth).
//!
//! Usage:
//!   cargo xtask check github-pr [--title ..] [--body-file ..] [--body ..]
//!                               [--labels ..] [--branch ..] [--number N]
//!   cargo xtask check github-issue [--title ..] [--body-file ..] [--body ..]
//!                                  [--labels ..] [--number N]

mod github_issue;
mod github_pr;
mod util;
mod spec_rules;

use clap::{Args, Parser, Subcommand};
use github_issue::IssueInput;
use github_pr::PrInput;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use util::{Finding, Severity};

#[derive(Parser)]
#[command(
    name = "checks",
    version,
    about = "Validate GitHub PR / issue metadata and bodies"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run checks against a GitHub pull request or issue
    Check {
        #[command(subcommand)]
        target: Target,
    },
    /// Run repository structure rules (dep-edge, path-forbidden, text-pattern, etc.)
    Structure(StructureArgs),
}

#[derive(Subcommand)]
enum Target {
    /// Check a GitHub pull request
    GithubPr(PrArgs),
    /// Check a GitHub issue
    GithubIssue(IssueArgs),
}

#[derive(Args)]
struct PrArgs {
    #[command(flatten)]
    common: CommonArgs,
    /// Branch name (used by the PR-08 branch prefix check)
    #[arg(long)]
    branch: Option<String>,
}

#[derive(Args)]
struct IssueArgs {
    #[command(flatten)]
    common: CommonArgs,
}

#[derive(Args)]
struct CommonArgs {
    /// PR / issue title
    #[arg(long)]
    title: Option<String>,
    /// Path to a markdown body file
    #[arg(long)]
    body_file: Option<PathBuf>,
    /// Body text passed directly
    #[arg(long)]
    body: Option<String>,
    /// Comma-separated label list
    #[arg(long)]
    labels: Option<String>,
    /// PR / issue number to read with `gh`
    #[arg(long)]
    number: Option<u64>,
}
/// Repository structure rule checks (deterministic / advisory classes).
#[derive(Args)]
struct StructureArgs {
    /// Repository root (defaults to current working directory)
    #[arg(long)]
    root: Option<PathBuf>,
    /// Check class: deterministic | advisory | semantic
    #[arg(long, default_value = "deterministic")]
    class: String,
    /// Scope: changed | all
    #[arg(long, default_value = "changed")]
    scope: String,
    /// Base branch for changed scope (merge-base with HEAD)
    #[arg(long)]
    base: Option<String>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Check { target } => match target {
            Target::GithubPr(args) => run_pr(&args),
            Target::GithubIssue(args) => run_issue(&args),
        },
        Command::Structure(args) => run_structure(&args),
    }
}

fn run_pr(args: &PrArgs) -> ExitCode {
    let input = match resolve_pr_input(args) {
        Ok(input) => input,
        Err(e) => return fail_with(e),
    };
    report(&github_pr::check_pr(&input))
}

fn run_issue(args: &IssueArgs) -> ExitCode {
    let input = match resolve_issue_input(args) {
        Ok(input) => input,
        Err(e) => return fail_with(e),
    };
    report(&github_issue::check_issue(&input))
}
/// 运行仓库结构规则检查（deterministic / advisory / semantic 类）。
fn run_structure(args: &StructureArgs) -> ExitCode {
    let root = args.root.as_deref().unwrap_or_else(|| Path::new("."));
    let root_path = Path::new(root);

    // 收集跟踪文件（git ls-files）。
    let tracked_files: Vec<String> = match std::process::Command::new("git")
        .args(["ls-files"])
        .current_dir(root_path)
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect(),
        _ => {
            eprintln!("failed to run git ls-files; is {} a git repository?", root.display());
            return ExitCode::from(2);
        }
    };

    let mut findings: Vec<Finding> = Vec::new();

    match args.class.as_str() {
        "deterministic" => {
            // ---- dep-edge: 三层依赖方向 (checks.toml [kinds.dep-edge.domain]) ----
            if let Some(graph) = spec_rules::load_dep_graph(root_path) {
                let dep_edge_params = spec_rules::DepEdgeParams {
                    edges_deny: vec![
                        spec_rules::DepEdgeDeny { from: "crates/ui-*".into(), to: "crates/api-*".into(), message: None },
                        spec_rules::DepEdgeDeny { from: "crates/ui-*".into(), to: "crates/gateway-*".into(), message: None },
                        // The page layer may depend on harness-protocol (wire vocabulary) and
                        // nothing else from the harness domain: every other member is a host
                        // capability or the daemon kernel. After the coding-prefix retirement
                        // (ferrite doc 16) the old `-> crates/coding-harness-*` glob matched
                        // nothing, so the rule is spelled against the real members.
                        spec_rules::DepEdgeDeny { from: "crates/ui-*".into(), to: "crates/harness-agent-loop".into(), message: None },
                        spec_rules::DepEdgeDeny { from: "crates/ui-*".into(), to: "crates/harness-compaction".into(), message: None },
                        spec_rules::DepEdgeDeny { from: "crates/ui-*".into(), to: "crates/harness-config".into(), message: None },
                        spec_rules::DepEdgeDeny { from: "crates/ui-*".into(), to: "crates/harness-daemon".into(), message: None },
                        spec_rules::DepEdgeDeny { from: "crates/ui-*".into(), to: "crates/harness-llm".into(), message: None },
                        spec_rules::DepEdgeDeny { from: "crates/ui-*".into(), to: "crates/harness-mcp".into(), message: None },
                        spec_rules::DepEdgeDeny { from: "crates/ui-*".into(), to: "crates/harness-session".into(), message: None },
                        spec_rules::DepEdgeDeny { from: "crates/ui-*".into(), to: "crates/harness-tools".into(), message: None },
                        spec_rules::DepEdgeDeny { from: "crates/ui-*".into(), to: "crates/app-db/**".into(), message: None },
                        spec_rules::DepEdgeDeny { from: "crates/api-*".into(), to: "crates/ui-*".into(), message: None },
                        spec_rules::DepEdgeDeny { from: "crates/**".into(), to: "bin/**".into(), message: None },
                        spec_rules::DepEdgeDeny { from: "crates/**".into(), to: "platforms/**".into(), message: None },
                        spec_rules::DepEdgeDeny { from: "bin/**".into(), to: "platforms/**".into(), message: None },
                        spec_rules::DepEdgeDeny { from: "bin/**".into(), to: "bin/**".into(), message: None },
                        spec_rules::DepEdgeDeny { from: "platforms/**".into(), to: "platforms/**".into(), message: None },
                    ],
                };
                if let Ok(rule) =
                    spec_rules::DepEdgeRule::new("dep-edge.domain".into(), &dep_edge_params, Severity::Fail)
                {
                    findings.extend(rule.check(&graph));
                }

                // ---- dep-cycles: 依赖无环 ----
                let cycle_rule =
                    spec_rules::CyclesRule::new("dep-cycles.domain".into(), Severity::Fail);
                findings.extend(cycle_rule.check(&graph));
            }

            // ---- path-forbidden: pages-own-ui ----
            let path_forbidden_params = spec_rules::PathForbiddenParams {
                paths: vec![
                    "crates/ui-pages/src/*/ui.rs".into(),
                    "crates/ui-pages/src/*/ui/**".into(),
                ],
                message: Some(
                    "page 域不应自带 ui 子模块；共享组件放 crates/ui-components".into(),
                ),
            };
            if let Ok(rule) = spec_rules::PathForbiddenRule::new(
                "path-forbidden.pages-own-ui".into(),
                &path_forbidden_params,
                Severity::Fail,
            ) {
                let tracked: Vec<PathBuf> = tracked_files
                    .iter()
                    .map(|p| PathBuf::from(p.as_str()))
                    .collect();
                findings.extend(rule.check_files(&tracked));
            }

            // ---- name-pattern: rust-file-snake ----
            let name_pattern_params = spec_rules::NamePatternParams {
                paths_include: vec!["crates/**/*.rs".into(), "platforms/**/*.rs".into(), "bin/**/*.rs".into()],
                pattern: "^[a-z0-9_]+\\.rs$".into(),
                message: Some("Rust 源文件名必须 snake_case".into()),
            };
            if let Ok(rule) = spec_rules::NamePatternRule::new(
                "name-pattern.rust-file-snake".into(),
                &name_pattern_params,
                Severity::Fail,
            ) {
                let tracked: Vec<PathBuf> = tracked_files
                    .iter()
                    .map(|p| PathBuf::from(p.as_str()))
                    .collect();
                findings.extend(rule.check_files(&tracked));
            }

            // ---- text-pattern: hardcoded-secret ----
            let text_pattern_params = spec_rules::TextPatternParams {
                paths_include: vec![
                    "**/*.rs".into(),
                    "**/*.ts".into(),
                    "**/*.js".into(),
                    "**/*.py".into(),
                    "**/*.go".into(),
                ],
                paths_exclude: vec![
                    "**/tests/**".into(),
                    "tests/**".into(),
                    "**/test_*.rs".into(),
                    "*.md".into(),
                    "target/".into(),
                    ".wt/".into(),
                    "node_modules/".into(),
                ],
                patterns_deny: vec![
                    "(?i)(password|passwd|secret|api_key|apikey|token|auth_token|access_token|private_key|connection_string)\\s*[:=]\\s*[\"'][^\"'\\s]{8,}[\"]'".into()
                ],
                patterns_allow: vec![
                    r"(?i)(your-|xxx|todo|placeholder|example|test-key|mock|fake|dummy|change-me|replace-me|insert-|changeme|replaceme|fixture|seed_|devpass)".into()
                ],
                message: Some(
                    "疑似硬编码凭据（占位符/示例已滤；真假判定待语义层）".into(),
                ),
            };
            if let Ok(rule) = spec_rules::TextPatternRule::new(
                "text-pattern.hardcoded-secret".into(),
                &text_pattern_params,
                Severity::Warn,
            ) {
                for f in &tracked_files {
                    let path = Path::new(f);
                    if let Ok(content) = std::fs::read_to_string(path) {
                        findings.extend(rule.check_file(path, &content));
                    }
                }
            }

            // ---- css-var-mirror: theme.css 的 .dark 段镜像 :root token 集合 ----
            let css_var_mirror_params = spec_rules::CssVarMirrorParams {
                paths_include: vec!["crates/ui-*/**/theme.css".into()],
                message: Some("theme.css 的 .dark 段必须是语义 :root 块的同 token 镜像".into()),
            };
            if let Ok(rule) = spec_rules::CssVarMirrorRule::new(
                "css-var-mirror.theme-dark".into(),
                &css_var_mirror_params,
                Severity::Fail,
            ) {
                for f in &tracked_files {
                    let path = Path::new(f);
                    if let Ok(content) = std::fs::read_to_string(path) {
                        findings.extend(rule.check_file(path, &content));
                    }
                }
            }
        }
        "advisory" => {
            // ---- duplicate-file: 疑似复刻既有实现 ----
            let dup_params = spec_rules::DuplicateFileParams {
                paths_include: vec!["crates/**/*.rs".into(), "platforms/**/*.rs".into(), "bin/**/*.rs".into()],
                paths_exclude: vec!["**/tests/**".into(), "crates/ui-components/src/icons/**".into()],
                min_tokens: Some(80),
                min_similarity: Some(0.8),
                limit: Some(3),
                message: Some("疑似复刻既有实现（考虑复用；若确为独立实现可忽略）".into()),
            };
            if let Ok(rule) =
                spec_rules::DuplicateFileRule::new("duplicate-file.dup-advisory".into(), &dup_params, Severity::Warn)
            {
                // 既有文件 = 全仓跟踪文件；变更文件 = 本次改动（简化：用全仓文件自比，找近亲）
                let existing: Vec<(PathBuf, String)> = tracked_files
                    .iter()
                    .filter_map(|p| {
                        let path = Path::new(p);
                        std::fs::read_to_string(path).ok().map(|c| (path.to_path_buf(), c))
                    })
                    .collect();
                findings.extend(rule.check(&existing, &existing));
            }
            // ---- git 信号（hotspot/coupling/defects）：advisory，只给审查背景 ----
            // scope_exclude = @generated（checks.toml [rules."GIT.*"] 口径）：
            // 生成物 / manifest / demo 场不参与热点与耦合评分。
            let git_exclude: Vec<String> = vec![
                "Cargo.lock".into(),
                "Cargo.toml".into(),
                "README.md".into(),
                "**/tailwind.out.css".into(),
                "bin/kit-demo/**".into(),
                "corpus-dump.tsv".into(),
            ];
            let git_params = spec_rules::GitSignalParams {
                min_churn: 8,
                min_cochanges: 8,
                min_fixes: 6,
                limit: 10,
            };
            let hotspot = spec_rules::GitHotspotRule::new(
                "git-hotspot.mono".into(),
                &git_params,
                git_exclude.clone(),
                Severity::Info,
            );
            findings.extend(hotspot.check(root_path));
            let coupling = spec_rules::GitCouplingRule::new(
                "git-coupling.mono".into(),
                &git_params,
                git_exclude.clone(),
                Severity::Info,
            );
            findings.extend(coupling.check(root_path));
            let defects = spec_rules::GitDefectsRule::new(
                "git-defects.mono".into(),
                &git_params,
                git_exclude,
                Severity::Info,
            );
            findings.extend(defects.check(root_path));
        }
        "semantic" => {
            // ---- semantic-question.api-security：jev 端点语义判决 ----
            let api_key = std::env::var("JEV_API_KEY")
                .or_else(|_| std::env::var("TYPESAFE_API_KEY"));
            let api_key = match api_key {
                Ok(k) => k,
                Err(_) => {
                    eprintln!("note: semantic class requires JEV_API_KEY or TYPESAFE_API_KEY env");
                    return report(&findings);
                }
            };
            let questions = serde_json::json!({
                "input_validation_missing": {
                    "type": "noul",
                    "instructions": "Does this file contain public API endpoints that handle user input without validation or sanitization?",
                    "fail": 0.85,
                    "warn": 0.6,
                    "message": "公共 API 端点可能缺少输入校验"
                },
                "output_sanitization_missing": {
                    "type": "noul",
                    "instructions": "Does this file contain public API endpoints that output data without proper sanitization?",
                    "fail": 0.85,
                    "warn": 0.6,
                    "message": "公共 API 端点可能缺少输出净化"
                },
                "error_handling_exposure": {
                    "type": "choice",
                    "instructions": "Classify this file's error handling in public API endpoints.",
                    "fail": 0.9,
                    "warn": 0.7,
                    "fail_labels": ["risky"],
                    "message": "公共 API 错误处理可能泄露内部信息"
                },
                "authentication_missing": {
                    "type": "noul",
                    "instructions": "Does this file contain API endpoints that should require authentication but don't validate any auth tokens?",
                    "fail": 0.9,
                    "warn": 0.7,
                    "message": "公共 API 端点可能缺少认证"
                },
            });
            let rule = spec_rules::SemanticQuestionRule::new(
                "semantic-question.api-security".into(),
                vec!["crates/api-*/**/*.rs".into(), "crates/gateway-*/**/*.rs".into()],
                vec!["**/tests/**".into(), "**/test_*.rs".into()],
                "http://127.0.0.1:8001".into(),
                "jev-latest".into(),
                questions,
            );
            let tracked: Vec<PathBuf> = tracked_files
                .iter()
                .map(|p| PathBuf::from(p.as_str()))
                .collect();
            findings.extend(rule.check_files(&tracked, &api_key));
        }
        other => {
            eprintln!("unknown class: {other} (use deterministic | advisory | semantic)");
            return ExitCode::from(2);
        }
    }

    report(&findings)
}

/// Resolves the effective PR input, fetching missing data from `gh`.
fn resolve_pr_input(args: &PrArgs) -> Result<PrInput, String> {
    let mut title = args.common.title.clone();
    let mut body = read_local_body(&args.common)?;
    let mut labels = util::parse_labels(args.common.labels.as_deref());
    let mut branch = args.branch.clone();

    // When the title or the body is missing, read the PR from `gh` and fill
    // in every missing field (labels and branch are picked up from there too).
    if title.is_none() || body.is_none() {
        let (gh_title, gh_body, gh_labels, gh_branch) = fetch_pr(args.common.number)?;
        if title.is_none() {
            title = Some(gh_title);
        }
        if body.is_none() {
            body = Some(gh_body);
        }
        if labels.is_none() {
            labels = Some(gh_labels);
        }
        if branch.is_none() {
            branch = Some(gh_branch);
        }
    }

    Ok(PrInput {
        title: title.unwrap_or_default(),
        body: body.unwrap_or_default(),
        labels,
        branch,
    })
}

/// Resolves the effective issue input, fetching missing data from `gh`.
fn resolve_issue_input(args: &IssueArgs) -> Result<IssueInput, String> {
    let mut title = args.common.title.clone();
    let mut body = read_local_body(&args.common)?;
    let mut labels = util::parse_labels(args.common.labels.as_deref());

    if title.is_none() || body.is_none() {
        let (gh_title, gh_body, gh_labels) = fetch_issue(args.common.number)?;
        if title.is_none() {
            title = Some(gh_title);
        }
        if body.is_none() {
            body = Some(gh_body);
        }
        if labels.is_none() {
            labels = Some(gh_labels);
        }
    }

    Ok(IssueInput {
        title: title.unwrap_or_default(),
        body: body.unwrap_or_default(),
        labels,
    })
}

/// Reads the body from `--body-file` (priority) or `--body`.
fn read_local_body(common: &CommonArgs) -> Result<Option<String>, String> {
    if let Some(path) = &common.body_file {
        return std::fs::read_to_string(path)
            .map(Some)
            .map_err(|e| format!("failed to read body file {}: {e}", path.display()));
    }
    Ok(common.body.clone())
}

fn fetch_pr(number: Option<u64>) -> Result<(String, String, Vec<String>, String), String> {
    let mut cmd = vec!["pr".to_string(), "view".to_string()];
    if let Some(n) = number {
        cmd.push(n.to_string());
    }
    cmd.push("--json".to_string());
    cmd.push("title,body,labels,headRefName".to_string());

    #[derive(serde::Deserialize)]
    struct GhPr {
        title: String,
        body: String,
        labels: Vec<GhLabel>,
        #[serde(rename = "headRefName")]
        head_ref_name: String,
    }
    #[derive(serde::Deserialize)]
    struct GhLabel {
        name: String,
    }

    let stdout = run_gh(&cmd)?;
    let parsed: GhPr = serde_json::from_str(&stdout)
        .map_err(|e| format!("failed to parse `gh pr view` output: {e}"))?;
    Ok((
        parsed.title,
        parsed.body,
        parsed.labels.into_iter().map(|label| label.name).collect(),
        parsed.head_ref_name,
    ))
}

fn fetch_issue(number: Option<u64>) -> Result<(String, String, Vec<String>), String> {
    let mut cmd = vec!["issue".to_string(), "view".to_string()];
    if let Some(n) = number {
        cmd.push(n.to_string());
    }
    cmd.push("--json".to_string());
    cmd.push("title,body,labels".to_string());

    #[derive(serde::Deserialize)]
    struct GhIssue {
        title: String,
        body: String,
        labels: Vec<GhLabel>,
    }
    #[derive(serde::Deserialize)]
    struct GhLabel {
        name: String,
    }

    let stdout = run_gh(&cmd)?;
    let parsed: GhIssue = serde_json::from_str(&stdout)
        .map_err(|e| format!("failed to parse `gh issue view` output: {e}"))?;
    Ok((
        parsed.title,
        parsed.body,
        parsed.labels.into_iter().map(|label| label.name).collect(),
    ))
}

/// Runs `gh` with the given arguments and returns stdout on success.
fn run_gh(args: &[String]) -> Result<String, String> {
    let output = std::process::Command::new("gh")
        .args(args)
        .output()
        .map_err(|e| format!("failed to run `gh`: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "`gh {}` failed with status {}: {}",
            args.join(" "),
            output.status,
            stderr.trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| format!("`gh` output is not UTF-8: {e}"))
}

/// Prints findings and the summary line, returning the process exit code.
fn report(findings: &[Finding]) -> ExitCode {
    let failures = findings
        .iter()
        .filter(|finding| finding.severity == Severity::Fail)
        .count();

    for finding in findings {
        println!("{finding}");
    }

    if failures == 0 {
        println!("PASS: {} findings, 0 failures", findings.len());
        ExitCode::SUCCESS
    } else {
        println!("FAIL: {} findings, {} failures", findings.len(), failures);
        ExitCode::from(1)
    }
}

/// Prints an error to stderr and returns exit code 2 (gh failure / bad args).
fn fail_with(message: String) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(2)
}
