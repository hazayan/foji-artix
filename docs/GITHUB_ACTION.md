# Syspac GitHub Action

A reusable GitHub Action that sets up the foji package management tool in your workflows.

## Features

- **Fast Setup**: Downloads pre-built binaries instead of compiling
- **Smart Caching**: Caches binaries for even faster subsequent runs
- **Fallback Build**: Automatically builds from source if binary not available
- **Version Control**: Pin to specific versions or use latest
- **Zero Dependencies**: Statically-linked binaries (musl variant available)

## Usage

### Basic Usage

```yaml
name: My Workflow

on: [push]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Syspac
        uses: hazayan/foji@main
      
      - name: Use Syspac
        run: foji detect-changes --paths
```

### With Specific Version

```yaml
- name: Setup Syspac
  uses: hazayan/foji@main
  with:
    version: v0.2.0
```

### Build from Source

```yaml
- name: Setup Syspac (from source)
  uses: hazayan/foji@main
  with:
    version: build
```

### With Custom GitHub Token

```yaml
- name: Setup Syspac
  uses: hazayan/foji@main
  with:
    github-token: ${{ secrets.CUSTOM_TOKEN }}
```

## Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `version` | Version to use (`latest`, `vX.Y.Z`, or `build`) | No | `latest` |
| `github-token` | GitHub token for downloading releases | No | `${{ github.token }}` |

### Version Options

- **`latest`** (default): Downloads the most recent release
- **`v0.2.0`**: Downloads a specific version
- **`build`**: Compiles from source (requires Cargo.toml in repo)

## Outputs

| Output | Description |
|--------|-------------|
| `foji-path` | Full path to the foji binary |
| `foji-version` | Version of foji that was installed |

### Using Outputs

```yaml
- name: Setup Syspac
  id: foji
  uses: hazayan/foji@main

- name: Show version
  run: echo "Using foji ${{ steps.foji.outputs.foji-version }}"

- name: Use specific path
  run: ${{ steps.foji.outputs.foji-path }} detect-changes
```

## Complete Examples

### Example 1: Package Build Workflow

```yaml
name: Build Packages

on:
  push:
    branches: [main]

jobs:
  detect-changes:
    runs-on: ubuntu-latest
    outputs:
      packages: ${{ steps.changes.outputs.packages }}
    steps:
      - uses: actions/checkout@v3
        with:
          submodules: recursive
      
      - name: Setup Syspac
        uses: hazayan/foji@main
      
      - name: Detect changed packages
        id: changes
        run: |
          CHANGED=$(foji detect-changes --paths)
          echo "packages=$CHANGED" >> $GITHUB_OUTPUT
  
  build:
    needs: detect-changes
    if: needs.detect-changes.outputs.packages != ''
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Build packages
        run: |
          for pkg in ${{ needs.detect-changes.outputs.packages }}; do
            echo "Building $pkg"
            # Build logic here
          done
```

### Example 2: Package Listing

```yaml
name: List Packages

on:
  schedule:
    - cron: '0 0 * * 0'  # Weekly

jobs:
  list:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
        with:
          submodules: recursive
      
      - name: Setup Syspac
        uses: hazayan/foji@main
      
      - name: List all packages
        run: foji list-packages --verbose
      
      - name: Export as JSON
        run: foji list-packages --format json > packages.json
      
      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: package-list
          path: packages.json
```

### Example 3: Matrix Build

```yaml
name: Matrix Build

on: [push]

jobs:
  prepare:
    runs-on: ubuntu-latest
    outputs:
      packages: ${{ steps.packages.outputs.list }}
    steps:
      - uses: actions/checkout@v3
        with:
          submodules: recursive
      
      - name: Setup Syspac
        uses: hazayan/foji@main
      
      - name: Get package list as JSON
        id: packages
        run: |
          PACKAGES=$(foji detect-changes --all --format json)
          echo "list=$PACKAGES" >> $GITHUB_OUTPUT
  
  build:
    needs: prepare
    runs-on: ubuntu-latest
    strategy:
      matrix:
        package: ${{ fromJson(needs.prepare.outputs.packages) }}
    steps:
      - uses: actions/checkout@v3
      
      - name: Build ${{ matrix.package }}
        run: echo "Building ${{ matrix.package }}"
```

## Performance Comparison

### Before (Building from Source)

```yaml
- name: Install Rust
  uses: actions-rs/toolchain@v1
  # ~30 seconds

- name: Build foji
  run: cargo build --release
  # ~2-3 minutes (first time)
  # ~30-60 seconds (with cache)

Total: ~2.5-3.5 minutes (first time), ~1-1.5 minutes (cached)
```

### After (Using Action)

```yaml
- name: Setup Syspac
  uses: hazayan/foji@main
  # ~5-10 seconds (download)
  # ~1 second (cached)

Total: ~5-10 seconds (first time), ~1 second (cached)
```

**Improvement**: ~95% faster on first run, ~99% faster with cache!

## How It Works

1. **Check Version**: Determines which version to install
2. **Download Binary**: Attempts to download pre-built binary from releases
3. **Cache**: Stores binary in GitHub Actions cache
4. **Fallback**: If download fails, builds from source automatically
5. **PATH Update**: Adds foji to PATH for easy access

## Caching

The action automatically caches downloaded binaries using GitHub Actions cache:

- **Cache Key**: `foji-{version}-{os}`
- **Cache Location**: `~/.local/bin/foji`
- **Cache Duration**: Follows GitHub's cache retention policy (usually 7 days)

### Manual Cache Control

```yaml
# Disable caching by building from source
- uses: hazayan/foji@main
  with:
    version: build

# Clear cache by changing version
- uses: hazayan/foji@main
  with:
    version: v0.2.1  # New version = new cache key
```

## Troubleshooting

### Binary Not Found

**Problem**: "foji: command not found"

**Solution**: The action adds foji to PATH, but you need to run it after the setup step:

```yaml
- uses: hazayan/foji@main  # Must come first
- run: foji detect-changes  # Now available
```

### Download Fails

**Problem**: "Failed to download foji"

**Solution**: Action automatically falls back to building from source. If you want to force source build:

```yaml
- uses: hazayan/foji@main
  with:
    version: build
```

### Permission Denied

**Problem**: "Permission denied" when downloading

**Solution**: Provide a GitHub token with appropriate permissions:

```yaml
- uses: hazayan/foji@main
  with:
    github-token: ${{ secrets.GITHUB_TOKEN }}
```

### Version Not Found

**Problem**: "Release not found"

**Solution**: Check available versions:

```bash
gh release list --repo hazayan/foji
```

Or use `latest`:

```yaml
- uses: hazayan/foji@main
  with:
    version: latest
```

## Binary Releases

Pre-built binaries are available for:

- **Linux x86_64 (glibc)**: Most common, recommended
- **Linux x86_64 (musl)**: Static binary, no dependencies

### Binary Naming

- Standard: `foji-linux-x86_64`
- Static: `foji-linux-x86_64-musl`

The action automatically selects the appropriate binary for your platform.

## Building and Releasing Binaries

To create a new release with pre-built binaries:

### Automatic (on code push)

Binaries are automatically built and released when you push changes to `src/` or `Cargo.toml`:

```bash
git add src/
git commit -m "feat: add new feature"
git push
```

### Manual Release

Trigger a manual release with a specific version:

1. Go to **Actions** → **Release Syspac Tool**
2. Click **Run workflow**
3. Enter version (e.g., `v0.2.0`)
4. Click **Run workflow**

### Via Command Line

```bash
gh workflow run release-foji.yml -f version=v0.2.0
```

## Advanced Usage

### Custom Installation Location

```yaml
- name: Setup Syspac
  uses: hazayan/foji@main
  id: foji

- name: Copy to custom location
  run: |
    mkdir -p /opt/tools
    cp ${{ steps.foji.outputs.foji-path }} /opt/tools/
```

### Multiple Versions

```yaml
- name: Setup Latest
  uses: hazayan/foji@main
  id: latest

- name: Setup Specific
  uses: hazayan/foji@main
  with:
    version: v0.1.0
  id: specific

- name: Compare versions
  run: |
    echo "Latest: ${{ steps.latest.outputs.foji-version }}"
    echo "Specific: ${{ steps.specific.outputs.foji-version }}"
```

### Conditional Setup

```yaml
- name: Setup Syspac (only on main branch)
  if: github.ref == 'refs/heads/main'
  uses: hazayan/foji@main
```

## Related Documentation

- [README.md](../README.md) - General usage
- [QUICKSTART.md](../QUICKSTART.md) - Getting started
- [ARCHITECTURE.md](../ARCHITECTURE.md) - Technical details
- [Release Workflow](.github/workflows/release-foji.yml) - Binary build process

## Contributing

To improve the action:

1. Edit `action.yml`
2. Test locally with [act](https://github.com/nektos/act)
3. Submit a pull request

## License

See [LICENSE](../LICENSE) for details.
