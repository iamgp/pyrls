use anyhow::{Context, Result};
use console::style;

use crate::{
    analysis::{self, PackageReleaseAnalysis, ReleaseAnalysis},
    cli::{Cli, StatusArgs},
    config::Config,
    conventional_commits::ConventionalCommit,
    git::GitRepository,
    progress,
};

pub fn run(cli: &Cli, args: &StatusArgs) -> Result<()> {
    let repo = GitRepository::discover(".").context("failed to inspect git repository")?;
    let config = Config::load(&cli.config)?;

    let analysis = if cli.dry_run {
        match &args.since {
            Some(tag) => analysis::analyze_since(&repo, &config, tag)?,
            None => analysis::analyze(&repo, &config)?,
        }
    } else {
        let sp = progress::spinner("Analyzing commits…");
        let result = match &args.since {
            Some(tag) => analysis::analyze_since(&repo, &config, tag),
            None => analysis::analyze(&repo, &config),
        };
        sp.finish_and_clear();
        result?
    };

    if args.json {
        print_json(&repo, &analysis)?;
    } else if args.short {
        print_short(&analysis);
    } else if cli.dry_run && !args.json && !args.short {
        print_legacy(cli, &repo, &analysis)?;
    } else {
        print_dashboard(&repo, &config, &analysis, args)?;
    }

    Ok(())
}

fn print_legacy(cli: &Cli, repo: &GitRepository, analysis: &ReleaseAnalysis) -> Result<()> {
    println!("Repository: {}", repo.path().display());
    println!("Branch: {}", repo.current_branch()?);
    println!("Config: {}", cli.config.display());
    println!(
        "Last tag: {}",
        repo.latest_tag()?.unwrap_or_else(|| "none".to_string())
    );
    println!("Current version: {}", analysis.current_version);
    println!("Commit count: {}", analysis.commits.len());
    println!("Proposed bump: {}", analysis.bump.as_str());
    println!(
        "Next version: {}",
        analysis
            .next_version
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "unchanged".to_string())
    );
    println!("Release mode: {}", analysis.package_plan.release_mode);
    println!(
        "Package discovery: {}",
        analysis.package_plan.discovery_source
    );

    let selected_packages = analysis.package_plan.selected_packages();
    println!(
        "Selected package set: {} package(s)",
        selected_packages.len()
    );
    for package in &analysis.package_plan.packages {
        println!(
            "  - {} [{}] current={} next={} bump={} reason={}",
            package.name,
            package.root,
            package.current_version,
            package
                .next_version
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "unchanged".to_string()),
            package.bump.as_str(),
            package.selection_reason
        );
        if !package.changed_paths.is_empty() {
            println!("    changed files: {}", package.changed_paths.join(", "));
        }
    }

    if analysis.changelog.is_empty() {
        println!("Pending changelog: none");
    } else {
        println!("Pending changelog:");
        for (section, entries) in &analysis.changelog.sections {
            println!("  {section}:");
            for entry in entries {
                println!("    - {entry}");
            }
        }
    }

    if cli.dry_run {
        println!("Dry run: no files changed");
    }

    Ok(())
}

fn print_json(repo: &GitRepository, analysis: &ReleaseAnalysis) -> Result<()> {
    let packages: Vec<serde_json::Value> = analysis
        .package_plan
        .packages
        .iter()
        .map(|pkg| {
            serde_json::json!({
                "name": pkg.name,
                "root": pkg.root,
                "current_version": pkg.current_version.to_string(),
                "next_version": pkg.next_version.as_ref().map(|v| v.to_string()),
                "bump": pkg.bump.as_str(),
                "selected": pkg.selected,
                "commit_count": pkg.commits.len(),
                "commits": pkg.commits.iter().map(|c| {
                    serde_json::json!({
                        "id": c.id,
                        "message": c.message,
                    })
                }).collect::<Vec<_>>(),
            })
        })
        .collect();

    let output = serde_json::json!({
        "branch": repo.current_branch().unwrap_or_else(|_| "unknown".to_string()),
        "last_tag": repo.latest_tag().unwrap_or(None),
        "current_version": analysis.current_version.to_string(),
        "next_version": analysis.next_version.as_ref().map(|v| v.to_string()),
        "bump": analysis.bump.as_str(),
        "commit_count": analysis.commits.len(),
        "release_mode": analysis.package_plan.release_mode,
        "packages": packages,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn print_short(analysis: &ReleaseAnalysis) {
    for pkg in &analysis.package_plan.packages {
        let version_info = match &pkg.next_version {
            Some(next) => format!(
                "{} → {} ({})",
                pkg.current_version,
                next,
                pkg.bump.as_str()
            ),
            None => format!("{} → no change", pkg.current_version),
        };

        let commit_info = if pkg.commits.is_empty() {
            String::new()
        } else {
            format!("   {} commits", pkg.commits.len())
        };

        println!(
            " {:<12} {}{}",
            style(&pkg.name).bold(),
            version_info,
            commit_info
        );
    }
}

fn print_dashboard(
    repo: &GitRepository,
    config: &Config,
    analysis: &ReleaseAnalysis,
    args: &StatusArgs,
) -> Result<()> {
    println!();
    println!("{}", style("pyrls status").bold());
    println!();

    let branch = repo.current_branch().unwrap_or_else(|_| "unknown".to_string());
    let last_tag = repo.latest_tag().unwrap_or(None);

    for pkg in &analysis.package_plan.packages {
        print_package_section(pkg, &branch, &last_tag, config, args);
        println!();
    }

    // Release PR status placeholder
    println!(
        " {} {}",
        style("Release PR:").dim(),
        style("not yet implemented").dim().italic()
    );
    // PyPI placeholder
    println!(
        " {} {}",
        style("PyPI:").dim(),
        style("not yet implemented").dim().italic()
    );
    println!();

    Ok(())
}

fn print_package_section(
    pkg: &PackageReleaseAnalysis,
    branch: &str,
    last_tag: &Option<String>,
    _config: &Config,
    _args: &StatusArgs,
) {
    println!(" {} {}", style("Package").cyan().bold(), style(&pkg.name).bold());

    match &pkg.next_version {
        Some(next) => {
            println!(
                " {} {} → {} ({})",
                style("Version").cyan().bold(),
                pkg.current_version,
                style(next).green(),
                style(pkg.bump.as_str()).yellow()
            );
        }
        None => {
            println!(
                " {} {} (no change)",
                style("Version").cyan().bold(),
                pkg.current_version
            );
        }
    }

    println!(" {} {}", style("Branch").cyan().bold(), branch);

    match last_tag {
        Some(tag) => println!(" {} {}", style("Last release").cyan().bold(), tag),
        None => println!(" {} {}", style("Last release").cyan().bold(), style("none").dim()),
    }

    if pkg.commits.is_empty() {
        println!();
        println!(" {}", style("No unreleased commits").dim());
        return;
    }

    println!();
    println!(
        " {} ({})",
        style("Unreleased commits").cyan().bold(),
        pkg.commits.len()
    );

    // Parse conventional commits for type info
    let rows: Vec<(String, String, String)> = pkg
        .commits
        .iter()
        .map(|commit| {
            let commit_type = ConventionalCommit::parse_message(&commit.message)
                .map(|cc| cc.commit_type)
                .unwrap_or_else(|_| "other".to_string());
            let first_line = commit.message.lines().next().unwrap_or(&commit.message);
            let description = ConventionalCommit::parse_message(&commit.message)
                .map(|cc| cc.description)
                .unwrap_or_else(|_| first_line.to_string());
            let short_hash = if commit.id.len() >= 7 {
                &commit.id[..7]
            } else {
                &commit.id
            };
            (commit_type, description, short_hash.to_string())
        })
        .collect();

    // Calculate column widths
    let type_width = rows
        .iter()
        .map(|(t, _, _)| t.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let msg_width = rows
        .iter()
        .map(|(_, m, _)| m.len())
        .max()
        .unwrap_or(7)
        .max(7)
        .min(50);
    let hash_width = 7;

    // Table header
    println!(
        " ┌{:─<tw$}┬{:─<mw$}┬{:─<hw$}┐",
        "", "", "",
        tw = type_width + 2,
        mw = msg_width + 2,
        hw = hash_width + 2
    );
    println!(
        " │ {:<tw$} │ {:<mw$} │ {:<hw$} │",
        "Type", "Message", "Hash",
        tw = type_width,
        mw = msg_width,
        hw = hash_width
    );
    println!(
        " ├{:─<tw$}┼{:─<mw$}┼{:─<hw$}┤",
        "", "", "",
        tw = type_width + 2,
        mw = msg_width + 2,
        hw = hash_width + 2
    );

    for (commit_type, description, hash) in &rows {
        let truncated_msg = if description.len() > msg_width {
            format!("{}…", &description[..msg_width - 1])
        } else {
            description.clone()
        };
        println!(
            " │ {:<tw$} │ {:<mw$} │ {:<hw$} │",
            commit_type, truncated_msg, hash,
            tw = type_width,
            mw = msg_width,
            hw = hash_width
        );
    }

    println!(
        " └{:─<tw$}┴{:─<mw$}┴{:─<hw$}┘",
        "", "", "",
        tw = type_width + 2,
        mw = msg_width + 2,
        hw = hash_width + 2
    );
}
