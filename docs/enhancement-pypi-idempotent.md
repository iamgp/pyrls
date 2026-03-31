# ReleaseX Enhancement Proposal: PyPI Idempotent Publishing

## Current Gap

Phlo's release workflow includes a **65-line Python script** to check PyPI for already-published artifacts before publishing:

```python
# From phlo/.github/workflows/release.yml lines 103-167
def published(name: str, version: str) -> bool:
    with urllib.request.urlopen(f"https://pypi.org/pypi/{name}/json", timeout=20) as response:
        payload = json.load(response)
    return bool(payload.get("releases", {}).get(version))
```

This is needed because:
1. CI might retry a failed publish job
2. Multiple packages in a monorepo may have some already published
3. Partial releases need to be idempotent

## ReleaseX Already Has the Infrastructure

Looking at `src/pypi/mod.rs`, ReleaseX already has:

```rust
pub fn has_version(project_name: &str, version: &Version) -> Result<bool> {
    let response = fetch_project(project_name)?;
    Ok(response.releases.contains_key(&version.to_string()))
}
```

## Proposed Enhancement

### Option 1: Build-time PyPI Check (Recommended)

Add a `--skip-published` flag to `relx release publish` that:

1. Before building, queries PyPI for each selected package's target version
2. Skips building packages that already exist on PyPI
3. Only builds and publishes unpublished packages

**Benefits:**
- Faster CI (no wasted builds)
- True idempotency
- Simpler workflows (no custom Python)

**Usage in phlo:**
```yaml
- uses: iamgp/ReleaseX@v1
  with:
    command: release publish --skip-published
```

### Option 2: Post-build Filtering

Add `--skip-published` that:

1. Builds all packages normally
2. Before publishing each artifact, checks if version exists on PyPI
3. Skips publishing already-published artifacts

**Benefits:**
- Simpler to implement
- Artifacts are still available for inspection

**Trade-off:**
- Still spends time building packages that won't be published

## Simplified Phlo Workflow with Enhancement

```yaml
name: Release

on:
  push:
    tags: ["v*"]

jobs:
  publish:
    runs-on: ubuntu-latest
    environment: pypi
    steps:
      - uses: actions/checkout@v4

      - uses: astral-sh/setup-uv@v1
        with:
          version: "latest"

      # ReleaseX handles: building, PyPI check, publishing
      - uses: iamgp/ReleaseX@v1
        with:
          command: release publish --skip-published
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          PYPI_API_TOKEN: ${{ secrets.PYPI_API_TOKEN }}
```

**Total workflow: ~30 lines vs current 174 lines**

## Implementation Sketch

```rust
// In src/publish/mod.rs

pub fn execute_monorepo(
    repo_root: &Path,
    config: &Config,
    analysis: &ReleaseAnalysis,
) -> Result<()> {
    for (package_name, package_root) in monorepo_publish_targets(repo_root, analysis)? {
        // NEW: Check if already published
        if config.publish.skip_published {
            if let Some(version) = get_package_version(&package_root) {
                if pypi::has_version(package_name, &version)? {
                    println!("Skipping {package_name} {version}: already on PyPI");
                    continue;
                }
            }
        }
        
        // Build and publish as normal
        let plan = build_plan_for_package(&package_root, &config.publish, Some(package_name))?;
        // ... rest of publish logic
    }
}
```

## Alternative: Integration with `uv build`

Since phlo uses `uv`, we could also add a `relx build` command:

```bash
relx release build  # Build only changed packages
relx release publish  # Publish what was built
```

But this adds complexity. The `--skip-published` flag is simpler.

## Recommendation

1. **Short-term:** Phlo can already simplify significantly using current ReleaseX features
2. **Medium-term:** Add `--skip-published` flag to eliminate the custom PyPI dedupe script
3. **Long-term:** Consider `relx build` command for unified build/publish orchestration

## Related Files

- `src/publish/mod.rs` - Main publish logic
- `src/pypi/mod.rs` - PyPI API client (already has `has_version()`)
- `src/cli/release.rs` - CLI argument handling
- `src/config/mod.rs` - Config option for `skip_published`
