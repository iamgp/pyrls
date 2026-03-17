# Publishing

`relx` can publish using ecosystem-aware providers:

- Python: `uv` or `twine`
- Rust: `cargo`
- Go: `goreleaser`

## Enable publishing

```toml
[publish]
enabled = true
provider = "uv"
repository = "pypi"
dist_dir = "dist"
```

## Providers

### `uv`

```toml
[publish]
enabled = true
provider = "uv"
repository = "pypi"
token_env = "PYPI_TOKEN"
```

### `twine`

```toml
[publish]
enabled = true
provider = "twine"
repository = "pypi"
username_env = "PYPI_USERNAME"
password_env = "PYPI_PASSWORD"
```

### `cargo`

```toml
[project]
ecosystem = "rust"

[publish]
enabled = true
provider = "cargo"
repository = "crates-io"
```

### `goreleaser`

```toml
[project]
ecosystem = "go"

[publish]
enabled = true
provider = "goreleaser"
repository = "github"
```

## TestPyPI or custom repositories

```toml
[publish]
enabled = true
provider = "twine"
repository = "testpypi"
repository_url = "https://test.pypi.org/legacy/"
```

## Trusted publishing with OIDC

For GitHub Actions trusted publishing:

```toml
[publish]
enabled = true
provider = "uv"
trusted_publishing = true
oidc = true
```

Requirements:

- GitHub Actions job must have `id-token: write`
- the PyPI project must trust the GitHub repository as a trusted publisher

## Build artifacts

For Python providers, `relx release publish` expects built artifacts to already exist under `dist_dir`.

For Rust, `relx release publish` runs `cargo publish --locked` and does not require a `dist/` directory.

For Go, `relx release publish` runs `goreleaser release --clean`.

Typical CI sequence:

```bash
uv build
relx release publish
```

Rust example:

```bash
cargo build --locked
relx release publish
```

Go example:

```bash
goreleaser release --clean
```

## Dry run

Use:

```bash
relx release publish --dry-run
```

This prints:

- chosen provider
- target repository
- discovered artifact files
- relevant environment variable names
- the publish command that would be executed

## Safety checks

`relx healthcheck` can validate publish prerequisites before release:

- provider tool availability
- build success
- existing tag conflicts
- existing registry version conflicts where supported
- OIDC environment readiness for Python trusted publishing
