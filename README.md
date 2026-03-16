# pyrls

Automated Python release tooling for Git repositories — a single binary that handles version bumps, changelogs, release PRs, GitHub Releases, and PyPI publishing.

<!-- badges -->
<!-- ![CI](https://github.com/OWNER/pyrls/actions/workflows/ci.yml/badge.svg) -->
<!-- ![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg) -->

## Features

- **Conventional Commits** — derives version bumps from commit messages (`fix:` → patch, `feat:` → minor, `feat!:` → major)
- **PEP 440 versions** — full support for standard, pre-release (`a`, `b`, `rc`), post-release, and dev versions
- **Changelog generation** — auto-generates `CHANGELOG.md` in [Keep a Changelog](https://keepachangelog.com/) format
- **Release PRs** — opens and maintains a PR that accumulates changes; release happens when *you* merge it
- **GitHub Releases** — creates git tags and GitHub Releases with changelog notes on PR merge
- **PyPI publishing** — optional integration with `uv publish` or `twine upload`, including OIDC Trusted Publisher support
- **Monorepo support** — independent versioning and release PRs for multiple packages in one repo
- **Single binary** — written in Rust, no runtime dependencies

## Installation

### From GitHub Releases

Download the latest binary for your platform:

```bash
# Linux (x86_64)
curl -L https://github.com/OWNER/pyrls/releases/latest/download/pyrls-linux-x86_64 -o pyrls
chmod +x pyrls
sudo mv pyrls /usr/local/bin/
```

### From source

```bash
cargo install --path .
```

Or build directly:

```bash
git clone https://github.com/OWNER/pyrls.git
cd pyrls
cargo build --release
# Binary at ./target/release/pyrls
```

## Quick Start

```bash
# 1. Initialize config in your Python project
pyrls init

# 2. Make some commits using Conventional Commits format
git commit -m "feat: add user authentication"
git commit -m "fix: handle empty config gracefully"

# 3. Check what pyrls would do
pyrls status

# 4. Create a release PR on GitHub
pyrls release pr
```

## Configuration

All configuration lives in `pyrls.toml` at the repo root. Running `pyrls init` auto-detects your project layout and generates a starting config.

```toml
# ── Release settings ─────────────────────────────────────────────
[release]
branch = "main"                         # branch to watch for new commits
tag_prefix = "v"                        # tag format: v1.2.3
changelog_file = "CHANGELOG.md"         # path to changelog file
pr_title = "chore(release): {version}"  # release PR title template

# ── Versioning ───────────────────────────────────────────────────
[versioning]
strategy = "conventional_commits"       # only supported strategy for now
initial_version = "0.1.0"              # version to use if no tags exist

# ── Version files ────────────────────────────────────────────────
# Where to read and write the version string.
# Each entry needs either `key` (for structured files) or `pattern` (for text files).

[[version_files]]
path = "pyproject.toml"
key = "project.version"                 # dotted key into the TOML structure

[[version_files]]
path = "src/mypackage/__init__.py"
pattern = '__version__ = "{version}"'   # {version} is replaced with the actual version

[[version_files]]
path = "setup.cfg"
key = "metadata.version"

# ── Changelog ────────────────────────────────────────────────────
# Map commit types to changelog sections.
# Set to false to exclude a commit type from the changelog entirely.
[changelog]
sections.feat = "Added"
sections.fix = "Fixed"
sections.refactor = "Changed"
sections.perf = "Changed"
sections.docs = false                   # excluded from changelog

# ── Publishing (opt-in) ─────────────────────────────────────────
[publish]
enabled = false                         # publishing is never on by default
provider = "uv"                         # "uv" or "twine"
repository = "pypi"                     # repository name or custom URL
# repository_url = "https://..."       # optional: explicit index URL
dist_dir = "dist"                       # directory containing built distributions
trusted_publishing = false              # enable OIDC Trusted Publisher (no token needed)
# username_env = "PYPI_USERNAME"        # env var for username (optional)
# password_env = "PYPI_PASSWORD"        # env var for password (optional)
# token_env = "PYPI_TOKEN"             # env var for API token (optional)

# ── GitHub ───────────────────────────────────────────────────────
[github]
# owner = "myorg"                       # auto-detected from git remote
# repo = "myproject"                    # auto-detected from git remote
api_base = "https://api.github.com"     # override for GitHub Enterprise
token_env = "GITHUB_TOKEN"             # env var to read the token from
release_branch_prefix = "pyrls/release" # branch name prefix for release PRs
pending_label = "autorelease: pending"  # label applied to open release PRs
tagged_label = "autorelease: tagged"    # label applied after tagging

# ── Monorepo ─────────────────────────────────────────────────────
[monorepo]
enabled = false                         # set to true for multi-package repos
packages = []                           # list of package directories
release_mode = "unified"                # "unified" (one PR) or "per_package" (one PR each)
```

## CLI Reference

### Global flags

```
--config <PATH>   Path to config file (default: pyrls.toml)
--dry-run         Print what would happen without making changes
--verbose         Enable debug output
--no-color        Disable ANSI colour output
```

### Commands

#### `pyrls init`

Generate a `pyrls.toml` config file by auto-detecting your project layout. Detects `pyproject.toml`, `setup.cfg`, and `__version__` patterns in Python files. Fails if a config file already exists.

```bash
pyrls init
pyrls init --dry-run   # preview the generated config without writing it
```

#### `pyrls status`

Analyze commits since the last release and display a summary: current version, proposed bump, next version, pending changelog entries, and package plan details.

```bash
pyrls status
pyrls status --dry-run
```

#### `pyrls validate`

Parse and validate the config file. Reports the release branch and number of configured version files.

```bash
pyrls validate
pyrls validate --config path/to/pyrls.toml
```

#### `pyrls release pr`

Create or update the release PR on GitHub. The PR includes the proposed changelog entry, version bump, and is labeled `autorelease: pending`. In monorepo mode, creates one PR per package or a unified PR depending on config.

```bash
pyrls release pr
pyrls release pr --dry-run
```

#### `pyrls release tag`

Create a git tag and GitHub Release with the changelog section as release notes. Typically called by CI after the release PR is merged. Labels the merged PR with `autorelease: tagged`.

```bash
pyrls release tag
pyrls release tag --dry-run
```

#### `pyrls release publish`

Publish distributions to PyPI (or a custom index) using the configured provider (`uv` or `twine`). Requires `[publish] enabled = true` in config.

```bash
pyrls release publish
pyrls release publish --dry-run
```

## GitHub Actions

The recommended workflow uses the `pyrls/action` wrapper, which downloads the correct binary for your runner — no Rust or Node runtime needed.

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    branches: [main]

permissions:
  contents: write
  pull-requests: write
  id-token: write  # for OIDC PyPI publishing

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - uses: pyrls/action@v1
        with:
          command: release pr
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  publish:
    runs-on: ubuntu-latest
    if: github.event_name == 'push' && startsWith(github.ref, 'refs/tags/')
    steps:
      - uses: actions/checkout@v4

      - uses: pyrls/action@v1
        with:
          command: release publish
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

## How It Works

pyrls follows the **Release PR model**:

1. **Scan** — on every push to main, pyrls analyzes new commits since the last release tag
2. **Accumulate** — it opens (or updates) a Release PR containing the proposed version bump and changelog entry
3. **Release** — when a maintainer merges the PR, CI calls `pyrls release tag` to create the git tag and GitHub Release
4. **Publish** — optionally, CI calls `pyrls release publish` to push distributions to PyPI

This gives maintainers **human-in-the-loop control** — releases only happen when you merge the PR.

### Conventional Commits

Version bumps are derived from commit messages:

| Commit type | Version bump | Example |
|---|---|---|
| `fix:` | Patch | `1.0.0` → `1.0.1` |
| `feat:` | Minor | `1.0.0` → `1.1.0` |
| `feat!:` or `BREAKING CHANGE:` | Major | `1.0.0` → `2.0.0` |

## Pre-release Versions

pyrls supports PEP 440 pre-release versions:

- Alpha: `1.2.0a1`
- Beta: `1.2.0b1`
- Release candidate: `1.2.0rc1`
- Post-release: `1.2.0.post1`
- Dev: `1.2.0.dev1`

Pre-release and finalization support is planned via `--pre-release` and `--finalize` flags on the release commands (see the [PRD](prd.md) for roadmap details).

## Monorepo Support

Enable monorepo mode to manage multiple Python packages in a single repository with independent versioning.

```toml
# pyrls.toml
[monorepo]
enabled = true
packages = [
  "packages/core",
  "packages/cli",
  "packages/sdk",
]
release_mode = "per_package"  # or "unified"
```

- **`per_package`** — one release PR per changed package
- **`unified`** — one PR covering all changed packages

Each package directory should contain its own `pyproject.toml`. pyrls detects which packages have changed and creates version bumps independently.

When monorepo mode is enabled, the `[[version_files]]` requirement at the top level is relaxed — version files are resolved per package instead.

## License

MIT
