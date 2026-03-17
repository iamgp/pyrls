# Command Reference

## Global flags

```text
--config <PATH>   Path to config file (default: pyrls.toml)
--dry-run         Preview actions without mutating state
--verbose         Enable debug output
--no-color        Disable ANSI color output
```

## `pyrls init`

Create a starter config file by inspecting the repository.

```bash
pyrls init
pyrls init --dry-run
```

## `pyrls validate`

Parse and validate the configuration file.

```bash
pyrls validate
pyrls validate --config path/to/pyrls.toml
```

## `pyrls status`

Show release state for the current repository.

```bash
pyrls status
pyrls status --short
pyrls status --json
pyrls status --since=v1.2.0
pyrls status --channel
```

`status` includes:

- current and proposed versions
- unreleased commits
- package selection in monorepos
- release PR status when GitHub access is available
- latest published PyPI version when project metadata can be resolved

## `pyrls healthcheck`

Run release pre-flight validation.

```bash
pyrls healthcheck
pyrls healthcheck --only config
pyrls healthcheck --only git
pyrls healthcheck --only github
pyrls healthcheck --only build
pyrls healthcheck --only pypi
```

Exit codes:

- `0`: all checks passed
- `1`: one or more errors
- `2`: warnings only

## `pyrls workspace`

Print the detected monorepo or `uv` workspace structure.

```bash
pyrls workspace
```

## `pyrls generate-ci`

Generate a GitHub Actions release workflow.

```bash
pyrls generate-ci
pyrls generate-ci --dry-run
pyrls generate-ci --provider github
```

If the target workflow already exists and differs, `pyrls` prints a diff and refuses to overwrite automatically.

## `pyrls release`

Main release entrypoint.

### Snapshot mode

```bash
pyrls release --snapshot
```

Runs local release validation and writes outputs under `.pyrls/snapshot/`.

### `pyrls release pr`

Create or update the release PR.

```bash
pyrls release pr
pyrls release pr --dry-run
pyrls release pr --pre-release beta
pyrls release pr --channel beta
pyrls release pr --finalize
```

### `pyrls release tag`

Create the git tag and GitHub Release.

```bash
pyrls release tag
pyrls release tag --dry-run
pyrls release tag --channel beta
pyrls release tag --finalize
```

### `pyrls release publish`

Upload built distributions.

```bash
pyrls release publish
pyrls release publish --dry-run
```

## Pre-release kinds

Accepted values for `--pre-release`:

- `alpha`
- `beta`
- `rc`
- `post`
- `dev`
