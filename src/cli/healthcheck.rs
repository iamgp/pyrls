use std::{env, path::Path, process::Command};

use anyhow::Result;
use console::style;

use crate::{
    analysis::read_current_version,
    config::Config,
    git::{run_git, GitRepository},
    github,
    version::Version,
};

use super::{Cli, HealthcheckArgs};

#[derive(Debug)]
enum CheckResult {
    Pass(String),
    Warn(String),
    Fail(String),
}

struct HealthcheckReport {
    categories: Vec<(String, Vec<CheckResult>)>,
}

impl HealthcheckReport {
    fn new() -> Self {
        Self {
            categories: Vec::new(),
        }
    }

    fn add_category(&mut self, name: &str, checks: Vec<CheckResult>) {
        self.categories.push((name.to_string(), checks));
    }

    fn print(&self) {
        println!();
        println!("{}", style("pyrls healthcheck").bold());

        for (category, checks) in &self.categories {
            println!();
            println!(" {}", style(category).cyan().bold());
            for check in checks {
                match check {
                    CheckResult::Pass(msg) => {
                        println!(" {} {}", style("✓").green(), msg);
                    }
                    CheckResult::Warn(msg) => {
                        println!(" {} {}", style("⚠").yellow(), msg);
                    }
                    CheckResult::Fail(msg) => {
                        println!(" {} {}", style("✗").red(), msg);
                    }
                }
            }
        }

        let (warnings, errors) = self.counts();
        println!();
        println!(" {} warning(s), {} error(s).", warnings, errors);
    }

    fn counts(&self) -> (usize, usize) {
        let mut warnings = 0;
        let mut errors = 0;
        for (_, checks) in &self.categories {
            for check in checks {
                match check {
                    CheckResult::Warn(_) => warnings += 1,
                    CheckResult::Fail(_) => errors += 1,
                    CheckResult::Pass(_) => {}
                }
            }
        }
        (warnings, errors)
    }

    fn exit_code(&self) -> i32 {
        let (warnings, errors) = self.counts();
        if errors > 0 {
            1
        } else if warnings > 0 {
            2
        } else {
            0
        }
    }
}

const CATEGORIES: &[&str] = &["config", "git", "github", "build", "pypi"];

pub fn run(cli: &Cli, args: &HealthcheckArgs) -> Result<()> {
    if let Some(only) = &args.only {
        let lower = only.to_lowercase();
        if !CATEGORIES.contains(&lower.as_str()) {
            anyhow::bail!(
                "unknown category `{only}`. Valid categories: {}",
                CATEGORIES.join(", ")
            );
        }
    }

    let filter = args.only.as_deref().map(|s| s.to_lowercase());
    let should_run = |cat: &str| filter.as_deref().is_none() || filter.as_deref() == Some(cat);

    let config = Config::load(&cli.config).ok();
    let repo = GitRepository::discover(".").ok();

    let mut report = HealthcheckReport::new();

    if should_run("config") {
        report.add_category("Config", check_config(cli, &config));
    }

    if should_run("git") {
        report.add_category("Git", check_git(&config, &repo));
    }

    if should_run("github") {
        report.add_category("GitHub", check_github(&config, &repo));
    }

    if should_run("build") {
        report.add_category("Build", check_build(&config));
    }

    if should_run("pypi") {
        report.add_category("PyPI", check_pypi(&config, &repo));
    }

    report.print();

    let code = report.exit_code();
    if code != 0 {
        std::process::exit(code);
    }

    Ok(())
}

fn check_config(cli: &Cli, config: &Option<Config>) -> Vec<CheckResult> {
    let mut checks = Vec::new();

    match config {
        Some(cfg) => {
            checks.push(CheckResult::Pass(format!(
                "{} is valid",
                cli.config.display()
            )));

            let pyproject_exists = Path::new("pyproject.toml").exists();
            if pyproject_exists {
                checks.push(CheckResult::Pass("pyproject.toml found".to_string()));
            } else {
                checks.push(CheckResult::Warn("pyproject.toml not found".to_string()));
            }

            match read_current_version(Path::new("."), &cfg.version_files) {
                Ok(Some(version_str)) => match version_str.parse::<Version>() {
                    Ok(v) => {
                        checks.push(CheckResult::Pass(format!(
                            "project.version = {} (PEP 440 valid)",
                            v
                        )));
                    }
                    Err(_) => {
                        checks.push(CheckResult::Fail(format!(
                            "project.version = {} (not PEP 440 valid)",
                            version_str
                        )));
                    }
                },
                Ok(None) => {
                    checks.push(CheckResult::Warn(
                        "project.version not found in version files".to_string(),
                    ));
                }
                Err(e) => {
                    checks.push(CheckResult::Fail(format!(
                        "failed to read project.version: {}",
                        e
                    )));
                }
            }
        }
        None => {
            checks.push(CheckResult::Fail(format!(
                "{} could not be loaded",
                cli.config.display()
            )));
        }
    }

    checks
}

fn check_git(config: &Option<Config>, repo: &Option<GitRepository>) -> Vec<CheckResult> {
    let mut checks = Vec::new();

    let repo = match repo {
        Some(r) => r,
        None => {
            checks.push(CheckResult::Fail(
                "Not inside a git repository".to_string(),
            ));
            return checks;
        }
    };

    match run_git(repo.path(), ["status", "--porcelain"]) {
        Ok(output) => {
            if output.trim().is_empty() {
                checks.push(CheckResult::Pass("Working tree is clean".to_string()));
            } else {
                checks.push(CheckResult::Warn(
                    "Working tree has uncommitted changes".to_string(),
                ));
            }
        }
        Err(_) => {
            checks.push(CheckResult::Warn(
                "Could not check working tree status".to_string(),
            ));
        }
    }

    match repo.current_branch() {
        Ok(branch) => {
            let expected = config
                .as_ref()
                .map(|c| c.release.branch.as_str())
                .unwrap_or("main");
            if branch == expected {
                checks.push(CheckResult::Pass(format!("On branch {}", branch)));
            } else {
                checks.push(CheckResult::Warn(format!(
                    "On branch {} (expected {})",
                    branch, expected
                )));
            }
        }
        Err(_) => {
            checks.push(CheckResult::Warn("Could not determine current branch".to_string()));
        }
    }

    match run_git(repo.path(), ["ls-remote", "--exit-code", "origin"]) {
        Ok(_) => {
            checks.push(CheckResult::Pass("Remote origin is reachable".to_string()));
        }
        Err(_) => {
            checks.push(CheckResult::Fail(
                "Remote origin is not reachable".to_string(),
            ));
        }
    }

    match repo.commits_since_latest_tag() {
        Ok(commits) => {
            if commits.is_empty() {
                checks.push(CheckResult::Warn(
                    "No commits since last tag (nothing to release)".to_string(),
                ));
            } else {
                checks.push(CheckResult::Pass(format!(
                    "{} commit(s) since last tag",
                    commits.len()
                )));
            }
        }
        Err(_) => {
            checks.push(CheckResult::Warn(
                "Could not determine commits since last tag".to_string(),
            ));
        }
    }

    checks
}

fn check_github(config: &Option<Config>, repo: &Option<GitRepository>) -> Vec<CheckResult> {
    let mut checks = Vec::new();

    let token_env = config
        .as_ref()
        .map(|c| c.github.token_env.as_str())
        .unwrap_or("GITHUB_TOKEN");

    let token = env::var(token_env);
    match &token {
        Ok(_) => {
            checks.push(CheckResult::Pass(format!("{} is set", token_env)));
        }
        Err(_) => {
            checks.push(CheckResult::Fail(format!("{} is not set", token_env)));
            return checks;
        }
    }

    let token = token.unwrap();
    let config = match config {
        Some(c) => c,
        None => return checks,
    };
    let repo = match repo {
        Some(r) => r,
        None => return checks,
    };

    match github::detect_repo(repo, &config.github) {
        Ok(repo_ref) => {
            let url = format!(
                "{}/repos/{}/{}",
                config.github.api_base.trim_end_matches('/'),
                repo_ref.owner,
                repo_ref.name
            );
            match ureq::get(&url)
                .set("Authorization", &format!("Bearer {}", token))
                .set("User-Agent", "pyrls")
                .call()
            {
                Ok(_) => {
                    checks.push(CheckResult::Pass(format!(
                        "GitHub repo {}/{} is reachable",
                        repo_ref.owner, repo_ref.name
                    )));
                }
                Err(_) => {
                    checks.push(CheckResult::Fail(format!(
                        "GitHub repo {}/{} is not reachable",
                        repo_ref.owner, repo_ref.name
                    )));
                }
            }
        }
        Err(_) => {
            checks.push(CheckResult::Warn(
                "Could not detect GitHub repository".to_string(),
            ));
        }
    }

    checks
}

fn check_build(config: &Option<Config>) -> Vec<CheckResult> {
    let mut checks = Vec::new();

    let config = match config {
        Some(c) => c,
        None => {
            checks.push(CheckResult::Warn(
                "Config not loaded, skipping build checks".to_string(),
            ));
            return checks;
        }
    };

    if config.publish.enabled {
        let provider = config.publish.provider.trim();
        let installed = match provider {
            "uv" => Command::new("uv").arg("--version").output().is_ok_and(|o| o.status.success()),
            "twine" => Command::new("twine")
                .arg("--version")
                .output()
                .is_ok_and(|o| o.status.success()),
            _ => false,
        };

        if installed {
            checks.push(CheckResult::Pass(format!("{} is installed", provider)));
        } else {
            checks.push(CheckResult::Fail(format!("{} is not installed", provider)));
        }
    }

    if Path::new("pyproject.toml").exists() {
        checks.push(CheckResult::Pass(
            "pyproject.toml exists (build should succeed)".to_string(),
        ));
    } else {
        checks.push(CheckResult::Warn(
            "pyproject.toml not found (build may fail)".to_string(),
        ));
    }

    checks
}

fn check_pypi(config: &Option<Config>, repo: &Option<GitRepository>) -> Vec<CheckResult> {
    let mut checks = Vec::new();

    let config = match config {
        Some(c) => c,
        None => return checks,
    };

    if !config.publish.enabled {
        checks.push(CheckResult::Pass(
            "Publishing is disabled, skipping PyPI checks".to_string(),
        ));
        return checks;
    }

    if let Some(repo) = repo {
        match (read_current_version(Path::new("."), &config.version_files), repo.latest_tag()) {
            (Ok(Some(version_str)), Ok(Some(tag))) => {
                let expected_tag = format!("{}{}", config.release.tag_prefix, version_str);
                if tag == expected_tag {
                    checks.push(CheckResult::Fail(format!(
                        "Version {} is already tagged ({})",
                        version_str, tag
                    )));
                } else {
                    checks.push(CheckResult::Pass(format!(
                        "Version {} is not yet tagged",
                        version_str
                    )));
                }
            }
            (Ok(Some(version_str)), Ok(None)) => {
                checks.push(CheckResult::Pass(format!(
                    "Version {} is not yet tagged (no tags exist)",
                    version_str
                )));
            }
            _ => {
                checks.push(CheckResult::Warn(
                    "Could not check version tag status".to_string(),
                ));
            }
        }
    }

    let has_credentials = config.publish.oidc
        || config.publish.trusted_publishing
        || config.publish.token_env.is_some()
        || config.publish.username_env.is_some();

    if has_credentials {
        checks.push(CheckResult::Pass(
            "PyPI credentials or OIDC configured".to_string(),
        ));
    } else {
        checks.push(CheckResult::Warn(
            "No PyPI credentials or OIDC configured".to_string(),
        ));
    }

    checks
}
