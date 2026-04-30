# Quick Start Guide

Get up and running with Syspac in 5 minutes.

## Prerequisites

- Rust toolchain (1.70+)
- Git
- GitHub repository with package submodules
- GitHub Actions enabled

## Step 1: Build the Tool (1 minute)

```bash
cd /path/to/foji
cargo build --release
```

The binary will be at `target/release/foji`.

## Step 2: Test Locally (2 minutes)

```bash
# List all packages
./target/release/foji list-packages --verbose

# Detect changes (will show all packages if this is first commit)
./target/release/foji detect-changes

# Get version of a package
./target/release/foji package-version packages/niri
```

Expected output:
```
connman-resolvd: 1.2.0-1
ly: 0.6.0-2
niri: 0.1.0-1
valent: 1.0.0-1
```

## Step 3: Update GitHub Workflow (2 minutes)

### Option A: Use the Example Workflow

```bash
# Copy the example workflow
cp .github/workflows/build-rust.yml.example .github/workflows/build.yml

# Add slash commands (optional)
cp .github/workflows/slash-commands.yml .github/workflows/

# Commit and push
git add .github/workflows/
git commit -m "feat: migrate to Rust-based build system"
git push
```

### Option B: Update Existing Workflow

Replace the "Detect changed packages" step in your existing workflow:

**Old (remove this):**
```yaml
- name: Detect changed packages
  id: changes
  run: |
    # ~60 lines of bash
    get_base_ref() { ... }
    # ... complex logic ...
```

**New (replace with this):**
```yaml
- name: Set up Rust
  uses: actions-rs/toolchain@v1
  with:
    profile: minimal
    toolchain: stable

- name: Build foji tool
  run: cargo build --release

- name: Detect changed packages
  id: changes
  run: |
    if [[ "${{ github.event_name }}" == "repository_dispatch" ]] && \
       [[ "${{ github.event.action }}" == "rebuild-all" ]]; then
      CHANGED=$(./target/release/foji detect-changes --all)
    else
      CHANGED=$(./target/release/foji detect-changes)
    fi
    echo "packages=${CHANGED}" >> $GITHUB_OUTPUT
```

**Critical: Add Package Preservation**

Add this step BEFORE building packages:

```yaml
- name: Download existing release assets
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: |
    if gh release view repository >/dev/null 2>&1; then
      cd repo/x86_64
      gh release download repository --pattern "*.pkg.tar.zst*"
      gh release download repository --pattern "foji.db*"
      gh release download repository --pattern "foji.files*"
    fi
```

## Step 4: Test the Workflow (1 minute)

### Test Normal Build

```bash
# Make a small change to a package
cd packages/niri
echo "# Update" >> PKGBUILD
git add .
git commit -m "test: update niri"
git push
```

Go to GitHub Actions tab and verify:
- ✅ Only `niri` is detected as changed
- ✅ Only `niri` is rebuilt
- ✅ All other packages remain in release

### Test Full Rebuild (with slash command)

1. Create a PR or issue
2. Comment: `/rebuild-all`
3. Wait for bot response
4. Verify all packages are rebuilt

## Common Issues

### "repository not found"

**Problem**: Tool can't find git repository

**Solution**: Run from repository root, or use `--repo-path`:
```bash
foji detect-changes --repo-path /path/to/repo
```

### "bash: foji: command not found"

**Problem**: Binary not in PATH

**Solution**: Use full path or add to PATH:
```bash
./target/release/foji detect-changes
# or
export PATH="$PATH:$(pwd)/target/release"
foji detect-changes
```

### "No packages found"

**Problem**: No PKGBUILD files detected

**Solution**: Check package structure:
```bash
# Packages should be in subdirectories with PKGBUILD
ls packages/*/PKGBUILD
# or
find . -name PKGBUILD
```

### Workflow fails: "No existing release found"

**Problem**: First run, no release exists yet

**Solution**: This is normal! The workflow will create the first release.

### Packages missing after update

**Problem**: Forgot to add "Download existing release assets" step

**Solution**: Add the download step to your workflow (see Step 3).

## Next Steps

### Learn More
- [README.md](README.md) - Full documentation
- [ARCHITECTURE.md](ARCHITECTURE.md) - How it works
- [docs/SLASH_COMMANDS.md](docs/SLASH_COMMANDS.md) - Slash commands

### Customize
- Add more packages as git submodules
- Customize build container
- Add notification webhooks
- Set up monitoring

### Advanced Usage

#### JSON Output
```bash
foji detect-changes --format json | jq .
```

#### Custom Scripts
```bash
#!/bin/bash
PACKAGES=$(foji detect-changes)
for pkg in $PACKAGES; do
    echo "Building $pkg..."
    # Custom build logic
done
```

#### Version Checking
```bash
for pkg in $(foji list-packages); do
    version=$(foji package-version "packages/$pkg")
    echo "$pkg: $version"
done
```

## Tips

### Speed Up Builds
Cache Rust dependencies in GitHub Actions:
```yaml
- uses: actions/cache@v3
  with:
    path: |
      ~/.cargo/registry
      target/
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
```

### Debug Mode
Set `RUST_LOG` for detailed logs:
```bash
RUST_LOG=debug ./target/release/foji detect-changes
```

### Pre-commit Hook
Add to `.git/hooks/pre-commit`:
```bash
#!/bin/bash
# Ensure all PKGBUILDs are valid
for pkgbuild in $(find packages -name PKGBUILD); do
    if ! bash -n "$pkgbuild"; then
        echo "Invalid PKGBUILD: $pkgbuild"
        exit 1
    fi
done
```

## Success Criteria

After completing this guide, you should have:

- ✅ Built the foji tool
- ✅ Listed all packages in your repo
- ✅ Updated GitHub workflow
- ✅ Tested change detection
- ✅ Verified package preservation
- ✅ (Optional) Tested `/rebuild-all` command

## Getting Help

- **Issues**: Check [docs/FIXES.md](docs/FIXES.md) for common problems
- **Questions**: Open a GitHub issue
- **Bugs**: Report with `RUST_BACKTRACE=1` output

## Comparison: Before vs After

### Before (Shell)
```bash
# Workflow file: 150+ lines
# Change detection: ~60 lines of bash
# Error messages: Cryptic
# Testing: Manual only
# Package preservation: ❌ Broken
```

### After (Rust)
```bash
# Workflow file: 50 lines
# Change detection: 1 command
# Error messages: Clear and actionable
# Testing: Automated
# Package preservation: ✅ Working
```

You've successfully migrated to the Rust-based build system! 🎉
