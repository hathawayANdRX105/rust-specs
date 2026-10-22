//! `checks` — xtask-style CLI validating GitHub PR / issue metadata and bodies
//! against the rust-specs spec rules (see the ferrite `.githooks/spec/github/`
//! YAML files for the rule source of truth).
//!
//! Usage:
//!   cargo xtask check github-pr [--title ..] [--body-file ..] [--body ..]
//!                               [--labels ..] [--branch ..] [--number N]
//!   cargo xtask check github-issue [--title ..] [--body-file ..] [--body ..]
//!                                  [--labels ..] [--number N]

mod github_issue;
mod github_pr;
mod util;

use clap::{Args, Parser, Subcommand};
use github_issue::IssueInput;
use github_pr::PrInput;
use std::path::PathBuf;
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

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Check { target } => match target {
            Target::GithubPr(args) => run_pr(&args),
            Target::GithubIssue(args) => run_issue(&args),
        },
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
        println!("PASS: {} checks, 0 failures", findings.len());
        ExitCode::SUCCESS
    } else {
        println!("FAIL: {} checks, {} failures", findings.len(), failures);
        ExitCode::from(1)
    }
}

/// Prints an error to stderr and returns exit code 2 (gh failure / bad args).
fn fail_with(message: String) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(2)
}
