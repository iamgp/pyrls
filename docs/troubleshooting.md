# Troubleshooting and Operations

## Start with healthcheck

Run:

```bash
pyrls healthcheck
```

This is the fastest way to catch:

- invalid config
- missing GitHub token
- wrong branch
- unreachable remotes
- missing build tools
- existing PyPI version conflicts

## Common problems

## `pyrls.toml could not be loaded`

Fix:

- ensure the file exists
- ensure TOML syntax is valid
- run `pyrls validate`

## `nothing to release`

Possible reasons:

- there are no commits since the latest tag
- commit messages do not produce a releasable bump
- in a monorepo, your commits did not touch any selected package

Inspect with:

```bash
pyrls status
pyrls status --json
```

## GitHub API failures

Check:

- `GITHUB_TOKEN` is set
- the token can access the target repo
- the configured `owner`, `repo`, and `api_base` are correct

If using GitHub Enterprise, set:

```toml
[github]
api_base = "https://github.example.com/api/v3"
```

## Release PR not found in status

`pyrls status` looks for an open PR on the generated release branch. If none is found:

- no PR has been created yet
- the release branch prefix differs from the configured one
- the PR was renamed or manually recreated on a different branch

## Build failures

Check:

- `uv` or `twine` is installed if needed
- `pyproject.toml` exists
- your build backend works outside `pyrls`

Try:

```bash
uv build
pyrls release --snapshot
```

## Publish failures

Check:

- artifacts exist in `dist_dir`
- the configured repository and repository URL are correct
- PyPI credentials are present
- trusted publishing is correctly configured on PyPI

## Workflow generation refuses overwrite

`pyrls generate-ci` intentionally refuses to overwrite a differing workflow automatically. Use:

```bash
pyrls generate-ci --dry-run
```

Review the output or diff, then replace the workflow manually.

## Operational recommendations

- keep release automation running only on the intended release branches
- use `pyrls healthcheck` before enabling auto-publish
- prefer OIDC trusted publishing over static PyPI tokens
- keep `pyrls.toml` small and explicit
- use `--dry-run` before changing release configuration
