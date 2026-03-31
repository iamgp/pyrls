# ReleaseX Simplification Analysis for Phlo Monorepo

## Current State: Release Build Complexity in Phlo

Phlo is a Python monorepo with **33 packages** managed via uv workspaces. The current release process requires significant custom automation:

### Current Pain Points

1. **Manual Package Building Loop** (release.yml lines 92-101)
   ```yaml
   - name: Build packages
     run: |
       repo_root="$PWD"
       rm -rf dist
       uv build --out-dir "$repo_root/dist"
       for pkg in packages/*/; do
         package_dist="$repo_root/${pkg%/}/dist"
         rm -rf "$package_dist"
         uv build --directory "$pkg" --out-dir "$package_dist"
       done
   ```

2. **Custom PyPI Deduplication Script** (release.yml lines 103-167)
   - 65 lines of Python to check if artifacts already exist on PyPI
   - Parses wheel metadata and sdist PKG-INFO manually
   - Removes already-published artifacts before publishing

3. **Complex Workflow Orchestration**
   - 174 lines in release.yml across 3 jobs (release-pr, release-tag, publish)
   - Manual environment protection (release, pypi)
   - Token management across multiple secrets

4. **No Built-in Package Selection**
   - Release.yml builds ALL packages every time
   - No detection of which packages actually changed
   - Wasted build cycles for unchanged packages

5. **Manual Version Coordination**
   - 33 packages need version bumps coordinated
   - Root package + each workspace package has separate version
   - No automated cascade bumping when dependencies change

## How ReleaseX Already Simplifies This

ReleaseX already handles many of these concerns:

| Complexity | ReleaseX Solution | Status |
|------------|------------------|--------|
| Version bumping from commits | Conventional Commits parsing | ✅ Working |
| Changelog generation | Keep a Changelog format | ✅ Working |
| Release PR creation | `relx release pr` | ✅ Working |
| Tag creation on merge | `relx release tag` | ✅ Working |
| UV workspace discovery | Auto-detects from `tool.uv.workspace.members` | ✅ Working |
| Monorepo package selection | Only builds changed packages | ✅ Working |
| Cascade bumps | `cascade_bumps = true` config | ✅ Working |
| PyPI publishing | `relx release publish` | ✅ Working |

## Recommended Configuration for Phlo

### 1. Simplified relx.toml

```toml
[project]
ecosystem = "python"

[release]
branch = "main"
tag_prefix = "v"
changelog_file = "CHANGELOG.md"
pr_title = "chore(release): {version}"

[versioning]
strategy = "conventional_commits"
initial_version = "0.7.0"

# Single version file for root package
[[version_files]]
path = "pyproject.toml"
key = "project.version"

[changelog]
sections.feat = "Added"
sections.fix = "Fixed"
sections.refactor = "Changed"
sections.perf = "Changed"
sections.docs = false
sections.chore = false
sections.ci = false
sections.build = false
sections.style = false
sections.test = false

[publish]
enabled = true
provider = "uv"
repository = "pypi"
dist_dir = "dist"
trusted_publishing = false
token_env = "PYPI_API_TOKEN"

[github]
token_env = "GITHUB_TOKEN"
release_branch_prefix = "relx/release"
pending_label = "autorelease: pending"
tagged_label = "autorelease: tagged"

[monorepo]
enabled = true
# Leave packages empty - auto-discover from uv workspace
packages = []
release_mode = "release_set"

[workspace]
# Enable cascade bumps for inter-package dependencies
cascade_bumps = true
```

### 2. Simplified GitHub Workflow

Replace 174 lines with ~50 lines:

```yaml
name: Release

on:
  push:
    branches: [main]
    tags: ["v*"]

permissions:
  contents: read

jobs:
  release-pr:
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    environment: release
    permissions:
      contents: write
      pull-requests: write
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
          token: ${{ secrets.RELEASE_PLEASE_TOKEN }}

      - uses: astral-sh/setup-uv@v1
        with:
          version: "latest"
          enable-cache: false

      - uses: iamgp/ReleaseX@v1
        with:
          command: release pr
        env:
          GITHUB_TOKEN: ${{ secrets.RELEASE_PLEASE_TOKEN }}

  release-tag:
    if: github.event_name == 'push' && startsWith(github.event.head_commit.message, 'chore(release):')
    runs-on: ubuntu-latest
    environment: release
    permissions:
      contents: write
      pull-requests: write
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
          token: ${{ secrets.RELEASE_PLEASE_TOKEN }}

      - uses: iamgp/ReleaseX@v1
        with:
          command: release tag
        env:
          GITHUB_TOKEN: ${{ secrets.RELEASE_PLEASE_TOKEN }}

  publish:
    if: startsWith(github.ref, 'refs/tags/v')
    runs-on: ubuntu-latest
    environment: pypi
    permissions:
      contents: read
    steps:
      - uses: actions/checkout@v4

      - uses: astral-sh/setup-uv@v1
        with:
          version: "latest"
          enable-cache: false

      - uses: iamgp/ReleaseX@v1
        with:
          command: release publish
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          PYPI_API_TOKEN: ${{ secrets.PYPI_API_TOKEN }}
```

## What ReleaseX Eliminates

| Current Phlo Release | ReleaseX Equivalent | Lines Saved |
|---------------------|---------------------|-------------|
| Manual package build loop | Auto-discovered from uv workspace | ~20 lines |
| Custom PyPI dedupe script | Built-in publish idempotency | ~65 lines |
| Version bump calculation | Conventional commits | ~30 lines |
| Changelog generation | Auto-generated | ~20 lines |
| Manual commit filtering | Auto-skips chore/ci/docs commits | ~10 lines |
| Package selection logic | release_set mode | ~15 lines |
| **Total** | | **~160 lines** |

## Additional Simplifications Possible

### 1. Unified Release Mode (Optional)

If Phlo prefers one coordinated release for all packages:

```toml
[monorepo]
release_mode = "unified"  # Instead of release_set
```

This creates:
- Single release PR for all changed packages
- Single git tag (e.g., `v0.8.0`)
- Unified changelog entry
- All packages published together

### 2. Release Set Mode (Current Best Fit)

Phlo's current `release_mode = "release_set"` provides:
- One PR for whatever packages changed
- Short release titles per package
- Only publish packages that actually changed
- Independent versioning per package

### 3. Pre-release Channel Support

For beta/alpha releases:

```toml
[[channels]]
branch = "develop"
prerelease = "b"  # Beta versions: 0.8.0b1, 0.8.0b2
publish = false   # Don't publish pre-releases to PyPI

[[channels]]
branch = "main"
# No prerelease = normal releases
```

### 4. Local Dry-Run Testing

Developers can test releases locally before CI:

```bash
# See what would change without making changes
relx status --dry-run

# Preview release PR content
relx release pr --dry-run

# Test the full workflow locally
relx release tag --dry-run
```

## Migration Path

### Phase 1: Validate Current Setup
```bash
cd /path/to/phlo
relx validate
relx workspace  # See discovered packages
relx status     # See current analysis
```

### Phase 2: Update Configuration
1. Update `relx.toml` with simplified config
2. Test with `--dry-run` flags
3. Update GitHub workflows

### Phase 3: Monitor First Release
1. Merge a feature/fix with conventional commit
2. Watch release PR creation
3. Merge release PR
4. Verify tagging and publishing

## Summary

**ReleaseX can reduce phlo's release complexity by ~70%**:

| Metric | Before | After |
|--------|--------|-------|
| Workflow lines | 174 | ~50 |
| Custom scripts | 2 (PyPI dedupe, build loop) | 0 |
| Manual version management | 33 packages | Auto-calculated |
| Build selection | All packages always | Only changed packages |
| Human coordination | High | Low (review/merge PRs) |

The key insight: **ReleaseX's monorepo support with uv workspace auto-discovery eliminates the need for custom build orchestration**. The `release_set` mode matches Phlo's current approach but with automated package selection and cascade bumping for dependencies.
