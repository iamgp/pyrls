# Publishing

`pyrls` can publish artifacts with either `uv` or `twine`.

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

`pyrls release publish` expects built artifacts to already exist under `dist_dir`.

Typical CI sequence:

```bash
uv build
pyrls release publish
```

## Dry run

Use:

```bash
pyrls release publish --dry-run
```

This prints:

- chosen provider
- target repository
- discovered artifact files
- relevant environment variable names
- the publish command that would be executed

## Safety checks

`pyrls healthcheck` can validate publish prerequisites before release:

- provider tool availability
- build success
- existing tag conflicts
- existing PyPI version conflicts
- OIDC environment readiness
