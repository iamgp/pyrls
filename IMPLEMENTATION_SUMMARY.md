# Implementation Summary: `--skip-published` Flag

## Changes Made

### 1. CLI Changes (`src/cli/mod.rs`)
- Added `PublishArgs` struct with `skip_published` flag
- Changed `ReleaseSubcommand::Publish` to accept `PublishArgs`

### 2. Config Changes (`src/config/mod.rs`)
- Added `skip_published: bool` field to `PublishConfig`
- Updated `Default` implementation

### 3. Release Command Handler (`src/cli/release.rs`)
- Updated `ReleaseSubcommand::Publish` match arm to pass `skip_published` to publish functions
- Updated dry-run output to show skip status

### 4. Publish Module (`src/publish/mod.rs`)
- Updated `execute()` to accept `skip_published` parameter and check PyPI before publishing
- Updated `execute_monorepo()` to check each package before publishing
- Updated `print_dry_run()` to display skip status
- Added helper functions:
  - `check_already_published()`: Checks PyPI (and future: crates.io) for existing versions
  - `get_current_version()`: Reads version from version_files
  - `get_package_name()`: Reads package name from pyproject.toml or Cargo.toml

### 5. Documentation Updates
- `docs/configuration.md`: Added `skip_published` to `[publish]` section docs
- `README.md`: Added example config comment and CLI reference

## Usage

### CLI Flag
```bash
relx release publish --skip-published
```

### Config File
```toml
[publish]
skip_published = true
```

## How It Works

1. Before publishing each package, ReleaseX checks if the target version already exists on the registry:
   - For Python (uv/twine): Uses PyPI JSON API via `pypi::has_version()`
   - For Rust (cargo): Currently returns false (TODO: implement crates.io check)
   - For Go (goreleaser): Returns false (no standard registry check)

2. If already published:
   - Prints "Skipping {package} {version}: already published"
   - Continues to next package
   - Does not fail the overall publish

3. If not published (or check fails):
   - Proceeds with normal publish flow
   - If check fails, prints warning but continues

## Benefits for Phlo

Replaces this 65-line Python script in `.github/workflows/release.yml`:

```python
import email
import json
import tarfile
import urllib.request
import zipfile
from pathlib import Path

def artifact_metadata(path: Path) -> tuple[str, str]:
    # ... parse wheel/sdist metadata ...

def published(name: str, version: str) -> bool:
    with urllib.request.urlopen(...) as response:
        payload = json.load(response)
    return bool(payload.get("releases", {}).get(version))

# ... iterate through all dist directories ...
# ... remove already-published artifacts ...
```

With this single flag:

```yaml
- uses: iamgp/ReleaseX@v1
  with:
    command: release publish --skip-published
```

## Testing

All 56 unit tests pass. The feature:
- Checks PyPI for Python packages before publishing
- Handles monorepos (checks each package individually)
- Continues on API errors (with warning)
- Shows skip status in dry-run mode
