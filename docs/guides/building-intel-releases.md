# Building Intel Releases

This guide covers how to build ContactCMD for Intel Macs when your development machine is Apple Silicon.

## The Problem

Homebrew-installed Rust on Apple Silicon cannot cross-compile to Intel without additional setup. The compiler supports the target, but the pre-compiled standard library for x86_64 isn't included.

## Solution Options

### Option A: Install rustup (Local Cross-Compilation)

Replace or supplement Homebrew Rust with rustup-managed Rust:

```bash
# 1. Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Restart terminal or source the env
source "$HOME/.cargo/env"

# 3. Add Intel target
rustup target add x86_64-apple-darwin

# 4. Verify
rustup target list --installed
# Should show:
#   aarch64-apple-darwin
#   x86_64-apple-darwin

# 5. Build Intel version
./scripts/build-release.sh --intel-only
```

**Note:** If you want to keep Homebrew Rust, ensure `~/.cargo/bin` comes before `/opt/homebrew/bin` in your PATH when building releases.

### Option B: GitHub Actions (Automated CI Builds)

Use GitHub's Intel Mac runners to build automatically on each release.

#### Setup Steps

1. **Create the workflow file** at `.github/workflows/release.yml`:

```yaml
name: Build Release

on:
  push:
    tags:
      - 'v*'  # Triggers on version tags like v0.1.0

jobs:
  build-macos:
    strategy:
      matrix:
        include:
          - target: x86_64-apple-darwin
            os: macos-13          # Intel runner
            artifact: intel
          - target: aarch64-apple-darwin
            os: macos-14          # Apple Silicon runner
            artifact: arm64

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: ${{ matrix.target }}

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}

      - name: Install create-dmg
        run: brew install create-dmg

      - name: Create DMG
        run: |
          VERSION=${GITHUB_REF#refs/tags/v}
          mkdir -p release
          create-dmg \
            --volname "ContactCMD $VERSION (${{ matrix.artifact }})" \
            "release/contactcmd-$VERSION-${{ matrix.artifact }}.dmg" \
            "target/${{ matrix.target }}/release/contactcmd"

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: contactcmd-${{ matrix.artifact }}
          path: release/*.dmg

  create-release:
    needs: build-macos
    runs-on: ubuntu-latest
    permissions:
      contents: write

    steps:
      - name: Download artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts

      - name: Create checksums
        run: |
          cd artifacts
          find . -name "*.dmg" -exec mv {} . \;
          shasum -a 256 *.dmg > checksums.txt

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            artifacts/*.dmg
            artifacts/checksums.txt
          generate_release_notes: true
```

2. **Create a release**:

```bash
# Update version in Cargo.toml, then:
git add Cargo.toml
git commit -m "Bump version to 0.2.0"
git tag v0.2.0
git push origin main --tags
```

3. **GitHub Actions will automatically**:
   - Build on Intel Mac (macos-13 runner)
   - Build on Apple Silicon Mac (macos-14 runner)
   - Create DMGs for both architectures
   - Generate checksums
   - Create a GitHub Release with all artifacts

### Option C: Universal Binary via GitHub Actions

Build both architectures in CI and combine into a universal binary:

```yaml
# Add this job after both builds complete
create-universal:
  needs: build-macos
  runs-on: macos-14

  steps:
    - name: Download artifacts
      uses: actions/download-artifact@v4

    - name: Extract binaries from DMGs
      run: |
        for dmg in */*.dmg; do
          hdiutil attach "$dmg" -nobrowse
        done
        mkdir -p binaries
        cp /Volumes/*/contactcmd binaries/ 2>/dev/null || true

    - name: Create universal binary
      run: |
        lipo -create binaries/* -output contactcmd-universal

    - name: Create universal DMG
      run: |
        VERSION=${GITHUB_REF#refs/tags/v}
        brew install create-dmg
        create-dmg \
          --volname "ContactCMD $VERSION (Universal)" \
          "contactcmd-$VERSION-universal.dmg" \
          contactcmd-universal

    - name: Upload universal artifact
      uses: actions/upload-artifact@v4
      with:
        name: contactcmd-universal
        path: "*.dmg"
```

## Verification

After building, verify the binary architecture:

```bash
# Check a specific binary
file contactcmd
# Intel: Mach-O 64-bit executable x86_64
# ARM:   Mach-O 64-bit executable arm64
# Universal: Mach-O universal binary with 2 architectures

# List architectures in universal binary
lipo -archs contactcmd
# x86_64 arm64
```

## Recommended Approach

For regular releases:
1. **Development**: Build and test locally on Apple Silicon (`--arm-only`)
2. **Release**: Use GitHub Actions to build all variants automatically
3. **Distribution**: GitHub Releases hosts the DMGs with checksums

This keeps local setup simple while ensuring Intel users get native binaries.
