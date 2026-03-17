# pyrls Documentation

This directory contains the full product documentation for `pyrls`.

## Guides

- [Getting Started](./getting-started.md)
- [Configuration Reference](./configuration.md)
- [Command Reference](./commands.md)
- [CI and Automation](./ci.md)
- [Publishing](./publishing.md)
- [Channels and Pre-releases](./channels.md)
- [Monorepos and uv Workspaces](./workspaces.md)
- [Troubleshooting and Operations](./troubleshooting.md)

## What pyrls does

`pyrls` automates Python releases for Git repositories. It reads your project version from source files, inspects Conventional Commit history, generates changelog entries, opens or updates release pull requests, tags releases, creates GitHub Releases, and optionally publishes artifacts to PyPI.

The release model is intentionally conservative:

1. Commits accumulate on the release branch.
2. `pyrls release pr` prepares the next release as a PR.
3. A maintainer reviews and merges the PR.
4. CI runs `pyrls release tag`.
5. CI optionally runs `pyrls release publish`.

This preserves human approval for every release while removing the repetitive mechanics.
