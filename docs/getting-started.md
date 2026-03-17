# Getting Started

## Install

Build from source:

```bash
git clone https://github.com/iamgp/pyrls.git
cd pyrls
cargo build --release
./target/release/pyrls --version
```

Or install directly into Cargo's binary path:

```bash
cargo install --path .
```

## Repository requirements

`pyrls` expects:

- a Git repository
- a Python project with a `pyproject.toml`, `setup.cfg`, or `__version__` pattern
- commit messages that follow Conventional Commits
- a GitHub remote for PR and release automation

Optional features add more requirements:

- `uv` or `twine` for publishing
- `GITHUB_TOKEN` for GitHub API access
- PyPI credentials or OIDC trusted publishing for package uploads

## Initialize configuration

Run:

```bash
pyrls init
```

This creates a starter `pyrls.toml` by detecting:

- the default release branch
- GitHub owner and repository name
- version-bearing files such as `pyproject.toml`, `setup.cfg`, and `__init__.py`

Preview without writing:

```bash
pyrls init --dry-run
```

## Basic workflow

Make changes and commit using Conventional Commits:

```bash
git commit -m "feat: add async support"
git commit -m "fix: handle empty config"
```

Inspect the pending release:

```bash
pyrls status
pyrls healthcheck
```

Open or update the release PR:

```bash
pyrls release pr
```

After the PR is merged, tag and release from CI:

```bash
pyrls release tag
```

If publishing is enabled:

```bash
pyrls release publish
```

## Local validation

Use snapshot mode to run the release pipeline locally without pushing tags or publishing:

```bash
pyrls release --snapshot
```

This writes output under `.pyrls/snapshot/`, including:

- `CHANGELOG_ENTRY.md`
- `RELEASE_PR_BODY.md`
- `manifest.json`
- built artifacts in `.pyrls/snapshot/dist/`

## First commands to learn

```bash
pyrls validate
pyrls status
pyrls healthcheck
pyrls generate-ci --dry-run
pyrls workspace
```
