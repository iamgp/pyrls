use anyhow::{Context, Result};

use crate::{
    analysis,
    changelog::PendingChangelog,
    channels,
    cli::{Cli, PreReleaseArgs, PreReleaseKind, ReleaseCommand, ReleaseSubcommand},
    config::Config,
    git::GitRepository,
    github, progress, publish,
    version::{BumpLevel, Version},
};

fn apply_suffix_bump(version: &Version, kind: &PreReleaseKind) -> Result<Version> {
    match kind {
        PreReleaseKind::Alpha => version.bump_pre("a"),
        PreReleaseKind::Beta => version.bump_pre("b"),
        PreReleaseKind::Rc => version.bump_pre("rc"),
        PreReleaseKind::Post => Ok(version.bump_post()),
        PreReleaseKind::Dev => Ok(version.bump_dev()),
    }
}

fn apply_pre_release_override(
    analysis: &mut analysis::ReleaseAnalysis,
    args: &PreReleaseArgs,
) -> Result<()> {
    if args.finalize {
        let finalized = analysis.current_version.finalize();
        analysis.next_version = Some(finalized);
        for package in &mut analysis.package_plan.packages {
            if package.selected {
                package.next_version = Some(package.current_version.finalize());
            }
        }
    } else if let Some(kind) = &args.pre_release {
        let base = match &analysis.next_version {
            Some(v) => v.clone(),
            None => analysis.current_version.bump_patch(),
        };
        analysis.next_version = Some(apply_suffix_bump(&base, kind)?);
        for package in &mut analysis.package_plan.packages {
            if package.selected {
                let pkg_base = match &package.next_version {
                    Some(v) => v.clone(),
                    None => package.current_version.bump_patch(),
                };
                package.next_version = Some(apply_suffix_bump(&pkg_base, kind)?);
            }
        }
    }
    Ok(())
}

fn apply_channel_override(
    repo: &GitRepository,
    config: &Config,
    analysis: &mut analysis::ReleaseAnalysis,
    args: &PreReleaseArgs,
) -> Result<()> {
    let branch = repo
        .current_branch()
        .unwrap_or_else(|_| "unknown".to_string());
    channels::apply_channel_to_analysis(repo, config, analysis, &branch, args.channel.as_deref())?;
    Ok(())
}

/// When `release tag` runs after a release PR has been merged, the version
/// files already contain the bumped version (e.g. 0.2.0) and the latest tag
/// is still the old one (e.g. v0.1.0).  A naive re-analysis would scan the
/// commits since v0.1.0 — which now include the merge commit — and bump
/// *again* to 0.3.0.
///
/// This function detects that situation: the current version in the version
/// files is already newer than the latest tag, so we should tag the current
/// version rather than computing a new bump.
fn adjust_for_merged_release_pr(
    repo: &GitRepository,
    config: &Config,
    analysis: &mut analysis::ReleaseAnalysis,
) -> Result<()> {
    let tag_prefix = &config.release.tag_prefix;
    let latest_tag_version = repo
        .latest_tag()?
        .and_then(|tag| tag.strip_prefix(tag_prefix).map(|s| s.to_string()))
        .and_then(|s| s.parse::<Version>().ok());

    let Some(tag_version) = latest_tag_version else {
        return Ok(());
    };

    // If current version (from files) is already ahead of the latest tag,
    // the release PR has been merged — tag the current version as-is.
    if analysis.current_version > tag_version {
        let version = analysis.current_version.clone();
        analysis.next_version = Some(version.clone());
        analysis.bump = BumpLevel::None;
        analysis.changelog = PendingChangelog::from_commits(
            config,
            &analysis
                .commits
                .iter()
                .filter_map(|c| {
                    crate::conventional_commits::ConventionalCommit::parse_message(&c.message).ok()
                })
                .collect::<Vec<_>>(),
        );
        for package in &mut analysis.package_plan.packages {
            if package.selected {
                package.next_version = Some(version.clone());
                package.bump = BumpLevel::None;
            }
        }
    }

    Ok(())
}

fn analyze_for_publish(repo: &GitRepository, config: &Config) -> Result<analysis::ReleaseAnalysis> {
    let analysis = analysis::analyze(repo, config)?;
    if !(config.monorepo.enabled && analysis.package_plan.release_mode != "unified") {
        return Ok(analysis);
    }

    if !analysis.package_plan.selected_packages().is_empty() {
        return Ok(analysis);
    }

    let Some(previous_tag) = repo.previous_tag_before_head()? else {
        return Ok(analysis);
    };

    analysis::analyze_since(repo, config, &previous_tag)
}

pub fn run(cli: &Cli, command: &ReleaseCommand) -> Result<()> {
    if command.snapshot {
        return super::snapshot::run(cli);
    }

    let repo = GitRepository::discover(".").context("failed to inspect git repository")?;
    let config = Config::load(&cli.config_path())?;

    match &command.command {
        ReleaseSubcommand::Pr(args) => {
            let mut analysis = if cli.dry_run {
                analysis::analyze(&repo, &config)?
            } else {
                let sp = progress::spinner("Analyzing commits…");
                let result = analysis::analyze(&repo, &config);
                sp.finish_and_clear();
                result?
            };
            apply_channel_override(&repo, &config, &mut analysis, args)?;
            apply_pre_release_override(&mut analysis, args)?;
            if cli.dry_run {
                github::print_release_pr_dry_run(&repo, &config, &analysis)?;
            } else if config.monorepo.enabled {
                let sp = progress::spinner("Creating monorepo release PR(s)…");
                let result = github::execute_monorepo_release_pr(&repo, &config, &analysis);
                sp.finish_and_clear();
                result?;
            } else {
                let sp = progress::spinner("Creating release PR…");
                let result = github::execute_release_pr(&repo, &config, &analysis);
                sp.finish_and_clear();
                result?;
            }
        }
        ReleaseSubcommand::Tag(args) => {
            let mut analysis = if cli.dry_run {
                analysis::analyze(&repo, &config)?
            } else {
                let sp = progress::spinner("Analyzing commits…");
                let result = analysis::analyze(&repo, &config);
                sp.finish_and_clear();
                result?
            };
            adjust_for_merged_release_pr(&repo, &config, &mut analysis)?;
            apply_channel_override(&repo, &config, &mut analysis, args)?;
            apply_pre_release_override(&mut analysis, args)?;
            if cli.dry_run {
                github::print_release_tag_dry_run(&repo, &config, &analysis)?;
            } else if config.monorepo.enabled {
                let sp = progress::spinner("Tagging monorepo packages…");
                let result = github::execute_monorepo_release_tag(&repo, &config, &analysis);
                sp.finish_and_clear();
                result?;
            } else {
                let sp = progress::spinner("Creating tag and GitHub release…");
                let result = github::execute_release_tag(&repo, &config, &analysis);
                sp.finish_and_clear();
                result?;
            }
        }
        ReleaseSubcommand::Publish(args) => {
            let analysis = if cli.dry_run {
                analysis::analyze(&repo, &config)?
            } else {
                let sp = progress::spinner("Analyzing commits…");
                let result = analyze_for_publish(&repo, &config);
                sp.finish_and_clear();
                result?
            };
            if cli.dry_run {
                publish::print_dry_run(repo.path(), &config, args.skip_published)?;
            } else if config.monorepo.enabled && analysis.package_plan.release_mode != "unified" {
                let sp = progress::spinner("Publishing monorepo packages…");
                let result =
                    publish::execute_monorepo(repo.path(), &config, &analysis, args.skip_published);
                sp.finish_and_clear();
                result?;
            } else {
                let sp = progress::spinner("Publishing…");
                let result = publish::execute(repo.path(), &config, args.skip_published);
                sp.finish_and_clear();
                result?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, process::Command};

    use tempfile::tempdir;

    use super::analyze_for_publish;
    use crate::config::Config;
    use crate::git::GitRepository;

    #[test]
    fn analyze_for_publish_uses_previous_tag_for_release_set_tag_commits() {
        let repo_dir = tempdir().expect("tempdir");
        let repo_path = repo_dir.path();

        run(repo_path, &["git", "init", "-b", "main"]);
        run(repo_path, &["git", "config", "user.name", "Relx Test"]);
        run(
            repo_path,
            &["git", "config", "user.email", "relx@example.com"],
        );

        fs::create_dir_all(repo_path.join("packages/delta/src")).expect("create package dirs");
        fs::write(
            repo_path.join("pyproject.toml"),
            r#"[project]
name = "phlo"
version = "0.7.3"

[tool.uv.workspace]
members = ["packages/delta"]
"#,
        )
        .expect("write root pyproject");
        fs::write(repo_path.join("src_placeholder.txt"), "root initial\n")
            .expect("write root placeholder");
        fs::write(
            repo_path.join("packages/delta/pyproject.toml"),
            r#"[project]
name = "phlo-delta"
version = "0.2.3"
"#,
        )
        .expect("write package pyproject");
        fs::write(
            repo_path.join("packages/delta/src/mod.py"),
            "print('initial')\n",
        )
        .expect("write package source");
        fs::write(
            repo_path.join("relx.toml"),
            r#"[project]
ecosystem = "python"

[release]
branch = "main"
tag_prefix = "v"

[versioning]
strategy = "conventional_commits"
initial_version = "0.7.0"

[[version_files]]
path = "pyproject.toml"
key = "project.version"

[monorepo]
enabled = true
release_mode = "release_set"
packages = [".", "packages/delta"]

[workspace]
cascade_bumps = false
"#,
        )
        .expect("write config");
        run(repo_path, &["git", "add", "."]);
        run(
            repo_path,
            &["git", "commit", "-m", "chore: initial release state"],
        );
        run(
            repo_path,
            &["git", "-c", "tag.gpgSign=false", "tag", "v0.7.3"],
        );

        fs::write(repo_path.join("src_placeholder.txt"), "root changed\n").expect("update root");
        fs::write(
            repo_path.join("packages/delta/src/mod.py"),
            "print('changed')\n",
        )
        .expect("update package");
        run(
            repo_path,
            &[
                "git",
                "add",
                "src_placeholder.txt",
                "packages/delta/src/mod.py",
            ],
        );
        run(
            repo_path,
            &["git", "commit", "-m", "fix: centralize host resolution"],
        );

        fs::write(
            repo_path.join("pyproject.toml"),
            r#"[project]
name = "phlo"
version = "0.7.4"

[tool.uv.workspace]
members = ["packages/delta"]
"#,
        )
        .expect("bump root version");
        fs::write(
            repo_path.join("packages/delta/pyproject.toml"),
            r#"[project]
name = "phlo-delta"
version = "0.2.4"
"#,
        )
        .expect("bump package version");
        run(
            repo_path,
            &[
                "git",
                "add",
                "pyproject.toml",
                "packages/delta/pyproject.toml",
            ],
        );
        run(
            repo_path,
            &[
                "git",
                "commit",
                "-m",
                "chore(release): phlo 0.7.4 + 1 packages",
            ],
        );
        run(
            repo_path,
            &[
                "git",
                "-c",
                "tag.gpgSign=false",
                "tag",
                "v2pkgs-phlo-phlo-delta-deadbeef",
            ],
        );

        let repo = GitRepository::discover(repo_path).expect("repo");
        let config = Config::load(&repo_path.join("relx.toml")).expect("config");
        let analysis = analyze_for_publish(&repo, &config).expect("analysis");
        let selected = analysis.package_plan.selected_packages();

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].name, "phlo");
        assert_eq!(selected[1].name, "phlo-delta");
    }

    fn run(repo_path: &std::path::Path, args: &[&str]) {
        let status = Command::new(args[0])
            .args(&args[1..])
            .current_dir(repo_path)
            .status()
            .expect("command should run");
        assert!(status.success(), "command failed: {args:?}");
    }
}
