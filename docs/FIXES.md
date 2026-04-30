# Critical Fixes Applied

This document summarizes the critical bugs that were fixed in the Rust rewrite.

## Issue #1: Old Packages Removed from Release

### The Problem

**Original Behavior:**
```yaml
# Old workflow would:
1. Build only changed packages
2. Delete entire release
3. Create new release with ONLY the newly built packages
4. Result: All unchanged packages disappear!
```

**Impact:**
- Users lose access to packages that didn't change
- Repository becomes incomplete
- Each build removes more packages
- Eventually only the latest changed package exists

### The Fix

**New Behavior:**
```yaml
# New workflow:
1. Download ALL existing packages from release
2. Build changed packages (overwrites existing if present)
3. Delete old release
4. Create new release with ALL packages (old + new)
5. Result: Complete package repository maintained
```

**Implementation:**
```yaml
- name: Download existing release assets
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: |
    if gh release view repository >/dev/null 2>&1; then
      cd repo/x86_64
      # Download all existing packages
      gh release download repository --pattern "*.pkg.tar.zst*"
      gh release download repository --pattern "foji.db*"
      gh release download repository --pattern "foji.files*"
    fi
```

**Why This Works:**
1. All existing packages are downloaded to `repo/x86_64/`
2. Docker build adds new/changed packages to same directory
3. Old unchanged packages remain untouched
4. Release upload includes the entire `repo/x86_64/` directory
5. Users always have access to all packages

### Testing the Fix

**Before Fix:**
```bash
# Initial state: 4 packages
$ curl -L github.com/.../releases/download/repository/ | grep .pkg.tar.zst
niri-0.1.0-1.pkg.tar.zst
valent-1.0.0-1.pkg.tar.zst
ly-0.6.0-2.pkg.tar.zst
connman-resolvd-1.2.0-1.pkg.tar.zst

# Update only niri
$ git commit -m "Update niri"
$ git push

# After build: Only niri remains! 😱
$ curl -L github.com/.../releases/download/repository/ | grep .pkg.tar.zst
niri-0.1.1-1.pkg.tar.zst
```

**After Fix:**
```bash
# Initial state: 4 packages
$ curl -L github.com/.../releases/download/repository/ | grep .pkg.tar.zst
niri-0.1.0-1.pkg.tar.zst
valent-1.0.0-1.pkg.tar.zst
ly-0.6.0-2.pkg.tar.zst
connman-resolvd-1.2.0-1.pkg.tar.zst

# Update only niri
$ git commit -m "Update niri"
$ git push

# After build: All packages present! 🎉
$ curl -L github.com/.../releases/download/repository/ | grep .pkg.tar.zst
niri-0.1.1-1.pkg.tar.zst        # Updated
valent-1.0.0-1.pkg.tar.zst      # Preserved
ly-0.6.0-2.pkg.tar.zst          # Preserved
connman-resolvd-1.2.0-1.pkg.tar.zst  # Preserved
```

## Issue #2: No Way to Rebuild All Packages

### The Problem

**Original Behavior:**
- Could only build changed packages
- No mechanism to rebuild everything
- Had to manually trigger builds or modify workflow
- Difficult to recover from infrastructure changes

**Use Cases That Were Impossible:**
1. Dependency update affects all packages → Can't rebuild all
2. Build environment changes → Can't verify all packages
3. Repository corruption → Can't regenerate everything
4. Toolchain update → Can't ensure compatibility

### The Fix

**Added `--all` Flag:**
```bash
# Get only changed packages
$ foji detect-changes
niri

# Get ALL packages for rebuild
$ foji detect-changes --all
connman-resolvd ly niri valent
```

**Added Slash Command:**
```bash
# In any PR or issue, comment:
/rebuild-all

# Bot responds:
🔨 Rebuild All Packages Triggered
Rebuilding all packages: connman-resolvd ly niri valent
Watch the progress: [Build Workflow](...)
```

**Implementation:**

1. **Tool Enhancement** (src/main.rs):
```rust
DetectChanges {
    // ...
    #[arg(short, long)]
    all: bool,  // New flag
}

// In handler:
let changes = if all {
    // Return all packages
    let packages = package::find_all_packages(&repo_path)?;
    packages.iter().map(|p| p.name.clone()).collect()
} else {
    // Return only changed packages
    git::detect_changed_packages(&repo_path, base_ref.as_deref())?
};
```

2. **Workflow Enhancement** (build-rust.yml):
```yaml
- name: Detect changed packages
  run: |
    if [[ "${{ github.event_name }}" == "repository_dispatch" ]] && \
       [[ "${{ github.event.action }}" == "rebuild-all" ]]; then
      # Full rebuild requested
      CHANGED=$(./target/release/foji detect-changes --all)
    else
      # Normal change detection
      CHANGED=$(./target/release/foji detect-changes)
    fi
```

3. **Slash Command Handler** (.github/workflows/slash-commands.yml):
```yaml
on:
  issue_comment:
    types: [created]

jobs:
  rebuild-all:
    if: contains(github.event.comment.body, '/rebuild-all')
    steps:
      - name: Trigger full rebuild
        uses: peter-evans/repository-dispatch@v2
        with:
          event-type: rebuild-all
```

### Testing the Fix

**Scenario 1: Normal Push**
```bash
# Modify one package
$ cd packages/niri
$ echo "# comment" >> PKGBUILD
$ git commit -am "Update niri"
$ git push

# Workflow detects only changed package
📦 Detecting changes...
Changed packages: niri

# Only niri is rebuilt
```

**Scenario 2: Slash Command**
```bash
# In PR or issue, comment:
/rebuild-all

# Workflow uses --all flag
📦 Rebuild all requested...
Changed packages: connman-resolvd ly niri valent

# All packages are rebuilt
```

**Scenario 3: Manual Trigger**
```bash
# On your machine
$ ./target/release/foji detect-changes --all
connman-resolvd ly niri valent

# Use in custom scripts
$ for pkg in $(foji detect-changes --all); do
    echo "Building $pkg"
    # ... build logic ...
  done
```

## Summary of Fixes

| Issue | Before | After |
|-------|--------|-------|
| **Package Preservation** | ❌ Packages disappear on each build | ✅ All packages always available |
| **Full Rebuild** | ❌ No way to rebuild everything | ✅ Simple `--all` flag or `/rebuild-all` |
| **Release Management** | ❌ Deletes and recreates (lossy) | ✅ Downloads, merges, uploads (preserves) |
| **Workflow Complexity** | ❌ 150+ lines of bash | ✅ Simple tool invocation |
| **Error Handling** | ❌ Silent failures | ✅ Detailed error messages |
| **Testability** | ❌ Can't test shell scripts | ✅ Full test coverage |

## Verification Checklist

Before deploying, verify these scenarios:

- [ ] Push change to one package → Only that package rebuilds
- [ ] Push change to multiple packages → All changed packages rebuild
- [ ] Old packages remain available after build
- [ ] Pacman database includes all packages
- [ ] `/rebuild-all` triggers full rebuild
- [ ] Repository dispatch works correctly
- [ ] Slash command permissions enforced
- [ ] Build failures don't remove existing packages

## Rollback Plan

If issues occur:

1. **Immediate**: Restore old workflow from git history
2. **Package Loss**: Manually upload missing packages to release
3. **Database Corruption**: Rebuild database with `repo-add`

## Future Enhancements

Potential improvements:

1. **Incremental Updates**: Use `repo-remove` for removed packages
2. **Versioning**: Keep multiple versions of packages
3. **Signing**: Verify package signatures before upload
4. **Caching**: Cache built packages between runs
5. **Parallel Builds**: Build multiple packages simultaneously
6. **Dependency Order**: Build in topological order

## Related Documentation

- [README.md](../README.md) - Usage guide
- [SLASH_COMMANDS.md](SLASH_COMMANDS.md) - Slash command details
- [MIGRATION.md](../MIGRATION.md) - Migration from shell scripts
- [ARCHITECTURE.md](../ARCHITECTURE.md) - Technical architecture
