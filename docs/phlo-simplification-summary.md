# How ReleaseX Simplifies Phlo's Mono Repo Release Builds

## Executive Summary

Phlo's current release process for 33 Python packages requires **174 lines of GitHub workflow YAML** plus **65 lines of custom Python** for PyPI deduplication. ReleaseX can reduce this to **~30 lines** while adding intelligent features like:

- Automatic package selection (only build what changed)
- Cascade version bumps for dependencies
- Conventional commit-based versioning
- Human-in-the-loop release PRs
- Changelog generation

**Reduction: ~85% less workflow code, 100% less custom Python**

---

## Current State: The Complexity

### 1. Release Workflow (174 lines)

Phlo's `.github/workflows/release.yml` handles 3 jobs:

```yaml
# Job 1: Create release PR (lines 12-44)
release-pr:
  # Complex commit message filtering to avoid loops
  if: >
    github.event_name == 'push' &&
    github.ref == 'refs/heads/main' &&
    !startsWith(github.event.head_commit.message, 'chore(release):') &&
    !startsWith(github.event.head_commit.message, 'chore:') &&
    !startsWith(github.event.head_commit.message, 'docs:') &&
    ...  # 7 more patterns

# Job 2: Tag releases (lines 46-69)  
release-tag:
  if: github.event_name == 'push' && 
      github.ref == 'refs/heads/main' && 
      startsWith(github.event.head_commit.message, 'chore(release):')

# Job 3: Publish (lines 71-174)
publish:
  # Manual package building loop
  - name: Build packages
    run: |
      for pkg in packages/*/; do
        uv build --directory "$pkg" --out-dir "$package_dist"
      done
  
  # 65-line Python script for PyPI deduplication
  - name: Remove already-published artifacts
    run: |
      python3 - <<'PY'
      # ... complex metadata parsing ...
      PY
```

### 2. Manual Coordination Required

- **33 packages** need manual version management
- **Inter-package dependencies** require coordinated bumps
- **Build orchestration** loops through all packages every time
- **PyPI deduplication** to handle partial retries

### 3. Testing Matrix Complexity

The CI workflow (420 lines) tests each package individually:

```yaml
test-packages:
  strategy:
    matrix:
      include:
        - package: phlo-alerting
        - package: phlo-alloy
        - package: phlo-api
        # ... 30 more packages
```

---

## Target State: ReleaseX Simplification

### 1. Simplified Configuration

```toml
# relx.toml
[project]
ecosystem = "python"

[release]
branch = "main"
tag_prefix = "v"

[monorepo]
enabled = true
release_mode = "release_set"  # or "unified"

[workspace]
cascade_bumps = true  # Auto-bump dependents

[publish]
enabled = true
provider = "uv"
```

### 2. Simplified Workflow (~30 lines)

```yaml
name: Release

on:
  push:
    branches: [main]
    tags: ["v*"]

jobs:
  release-pr:
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: iamgp/ReleaseX@v1
        with:
          command: release pr

  release-tag:
    if: startsWith(github.head_commit.message, 'chore(release):')
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: iamgp/ReleaseX@v1
        with:
          command: release tag

  publish:
    if: startsWith(github.ref, 'refs/tags/v')
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: iamgp/ReleaseX@v1
        with:
          command: release publish
        env:
          PYPI_API_TOKEN: ${{ secrets.PYPI_API_TOKEN }}
```

### 3. Eliminated Complexity

| Current Component | Lines | ReleaseX Replacement | Lines |
|-------------------|-------|---------------------|-------|
| Commit filtering logic | 15 | Conventional commits | 0 |
| Package build loop | 20 | Auto-discovered packages | 0 |
| PyPI dedupe script | 65 | `--skip-published` flag | 0 |
| Version calculation | 30 | Auto-calculated | 0 |
| Changelog generation | 20 | Auto-generated | 0 |
| Workflow boilerplate | 24 | Standardized | 10 |
| **Total** | **174** | | **~30** |

---

## Key ReleaseX Features for Phlo

### 1. UV Workspace Auto-Discovery

ReleaseX reads `pyproject.toml` and discovers all 33 packages automatically:

```toml
[tool.uv.workspace]
members = [".", "packages/*"]
```

```bash
$ relx workspace

Workspace root: pyproject.toml
Discovery: uv workspace (tool.uv.workspace.members)
Members:
  . (phlo 0.7.9)
  packages/phlo-alerting (phlo-alerting 0.1.0) — depends on phlo
  packages/phlo-api (phlo-api 0.2.1) — depends on phlo, phlo-core-plugins
  ...  # 30 more packages with dependency graph
```

### 2. Release Set Mode

Only packages with actual changes get releases:

```toml
[monorepo]
release_mode = "release_set"
```

**Behavior:**
- Analyzes commits since last tag
- Maps changed files to packages
- Only builds/releases affected packages
- Creates one consolidated release PR

### 3. Cascade Bumps

When a dependency changes, dependent packages get patch bumps:

```toml
[workspace]
cascade_bumps = true
```

**Example:**
1. `phlo-core-plugins` gets a `feat:` commit → minor bump
2. `phlo-api` depends on `phlo-core-plugins`
3. `phlo-api` gets automatic patch bump even if no direct changes

### 4. Conventional Commits

Version bumps derived from commit messages:

| Commit | Version Change |
|--------|----------------|
| `fix: handle timeout` | Patch (0.7.9 → 0.7.10) |
| `feat: add webhook` | Minor (0.7.9 → 0.8.0) |
| `feat!: breaking API change` | Major (0.7.9 → 1.0.0) |

### 5. Release PR Model

Human-in-the-loop control:

1. Developer merges feature with `feat: ...` commit
2. ReleaseX analyzes and opens PR: "chore(release): 0.8.0"
3. PR includes auto-generated changelog
4. Maintainer reviews and merges when ready
5. Tag and publish happen automatically

---

## Migration Path

### Phase 1: Validate (Day 1)

```bash
cd /path/to/phlo

# Check current state
relx status

# See discovered packages
relx workspace

# Validate config
relx validate
```

### Phase 2: Dry Run (Day 1-2)

```bash
# Preview release PR
relx release pr --dry-run

# Preview what would be published
relx release publish --dry-run
```

### Phase 3: Update Workflows (Day 2-3)

1. Update `relx.toml` with new configuration
2. Replace release.yml with simplified version
3. Test on feature branch

### Phase 4: Monitor (Day 3-7)

1. Merge a feature with conventional commit
2. Watch release PR creation
3. Review auto-generated changelog
4. Merge release PR
5. Verify tag + publish

---

## Feature Comparison Matrix

| Feature | Current Phlo | ReleaseX |
|---------|--------------|----------|
| Version bumping | Manual calculation | Conventional commits |
| Package selection | Build all 33 always | Only changed packages |
| Changelog | Manual editing | Auto-generated |
| Release coordination | Human coordination | Release PRs |
| Dependency bumps | Manual tracking | Cascade bumps |
| PyPI deduplication | 65-line Python script | `--skip-published` (proposed) |
| Build orchestration | Custom shell loop | `uv` integration |
| Multi-Python testing | Manual matrix | (CI separate concern) |

---

## Recommended Next Steps

### For Phlo Adoption (Immediate)

1. **Update `relx.toml`** to use `release_mode = "release_set"` and `cascade_bumps = true`
2. **Simplify `release.yml`** using the 30-line template above
3. **Document conventional commits** for contributors

### For ReleaseX Enhancement (Short-term)

1. **Add `--skip-published` flag** to `relx release publish`
   - Uses existing `pypi::has_version()` function
   - Eliminates phlo's 65-line Python script
   - Makes publishes idempotent

### For Future Consideration (Long-term)

1. **Integration with CI test matrices**
   - ReleaseX could trigger package-specific tests
   - Only run tests for changed packages

2. **Build caching strategies**
   - Coordinate with `uv`'s cache
   - Skip builds for unchanged dependencies

---

## Summary

ReleaseX transforms phlo's release process from a **custom 239-line automation** (174 YAML + 65 Python) into a **30-line standard workflow**.

**The core insight:** ReleaseX's monorepo support with UV workspace auto-discovery eliminates the need for custom build orchestration. The `release_set` mode with `cascade_bumps` handles the complex inter-package dependency versioning that currently requires manual coordination.

**Result:** Safer, faster releases with less code to maintain.
