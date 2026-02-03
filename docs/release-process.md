# ContactCMD Release Process

This document describes how to create and publish releases of ContactCMD for macOS.

## Overview

ContactCMD is distributed as DMG files containing the pre-built binary. We provide three variants:

| Variant | File | Target Macs |
|---------|------|-------------|
| Intel | `contactcmd-VERSION-intel.dmg` | Macs with Intel processors (pre-2020) |
| Apple Silicon | `contactcmd-VERSION-arm64.dmg` | Macs with M1/M2/M3/M4 chips |
| Universal | `contactcmd-VERSION-universal.dmg` | Any Mac (larger file size) |

## Prerequisites

### One-Time Setup

1. **Install Rust toolchain** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Add cross-compilation targets**:
   ```bash
   rustup target add x86_64-apple-darwin    # Intel
   rustup target add aarch64-apple-darwin   # Apple Silicon
   ```

3. **Install create-dmg**:
   ```bash
   brew install create-dmg
   ```

4. **(Optional) Code Signing**: For distribution outside the App Store, you need a "Developer ID Application" certificate from Apple. This requires:
   - An Apple Developer account ($99/year)
   - Creating a Developer ID certificate in Xcode or Apple Developer portal
   - Installing the certificate in your Keychain

## Creating a Release

### Quick Release (Recommended)

Build all variants with a single command:

```bash
./scripts/build-release.sh
```

This creates:
```
release/
├── contactcmd-0.1.0-intel.dmg
├── contactcmd-0.1.0-arm64.dmg
├── contactcmd-0.1.0-universal.dmg
└── checksums.txt
```

### Build Options

```bash
# Build only for current architecture testing
./scripts/build-release.sh --arm-only

# Build but skip DMG creation (faster iteration)
./scripts/build-release.sh --skip-dmg

# Clean build (removes all cached artifacts)
./scripts/build-release.sh --clean

# With code signing
DEVELOPER_ID="Developer ID Application: Your Name (TEAMID)" \
  ./scripts/build-release.sh --sign

# Full release with notarization
DEVELOPER_ID="Developer ID Application: Your Name (TEAMID)" \
APPLE_ID="your@email.com" \
APPLE_PASSWORD="xxxx-xxxx-xxxx-xxxx" \
TEAM_ID="XXXXXXXXXX" \
  ./scripts/build-release.sh --sign --notarize
```

## Version Management

The version is read from `Cargo.toml`. To create a new release:

1. **Update version in Cargo.toml**:
   ```toml
   [package]
   version = "0.2.0"  # Update this
   ```

2. **Commit the version bump**:
   ```bash
   git add Cargo.toml
   git commit -m "Bump version to 0.2.0"
   ```

3. **Create a git tag**:
   ```bash
   git tag -a v0.2.0 -m "Release v0.2.0"
   git push origin main --tags
   ```

4. **Build the release**:
   ```bash
   ./scripts/build-release.sh --clean
   ```

## Code Signing and Notarization

### Why Sign?

Unsigned binaries show scary warnings on macOS:
- "contactcmd cannot be opened because the developer cannot be verified"
- Users must right-click and select "Open" to bypass Gatekeeper

Signed and notarized binaries open without warnings.

### Setting Up Code Signing

1. **Create an App-Specific Password** (for notarization):
   - Go to https://appleid.apple.com
   - Sign in and go to "App-Specific Passwords"
   - Generate a password for "ContactCMD Notarization"

2. **Find your Team ID**:
   ```bash
   # List available signing identities
   security find-identity -v -p codesigning
   ```
   Look for "Developer ID Application: Your Name (XXXXXXXXXX)" - the XXXXXXXXXX is your Team ID.

3. **Store credentials securely** (optional but recommended):
   ```bash
   # Store in environment file (don't commit this!)
   cat > ~/.contactcmd-release-env << 'EOF'
   export DEVELOPER_ID="Developer ID Application: Your Name (TEAMID)"
   export APPLE_ID="your@email.com"
   export APPLE_PASSWORD="xxxx-xxxx-xxxx-xxxx"
   export TEAM_ID="TEAMID"
   EOF
   chmod 600 ~/.contactcmd-release-env
   ```

4. **Use during release**:
   ```bash
   source ~/.contactcmd-release-env
   ./scripts/build-release.sh --sign --notarize
   ```

## Testing Releases

Before publishing, test on both architectures:

### On Apple Silicon Mac

```bash
# Test arm64 DMG
hdiutil attach release/contactcmd-0.1.0-arm64.dmg
/Volumes/ContactCMD*/contactcmd --version
hdiutil detach /Volumes/ContactCMD*

# Test universal DMG
hdiutil attach release/contactcmd-0.1.0-universal.dmg
/Volumes/ContactCMD*/contactcmd --version
file /Volumes/ContactCMD*/contactcmd  # Should show "universal binary"
hdiutil detach /Volumes/ContactCMD*
```

### On Intel Mac (or via Rosetta)

```bash
# Test intel DMG
hdiutil attach release/contactcmd-0.1.0-intel.dmg
/Volumes/ContactCMD*/contactcmd --version
hdiutil detach /Volumes/ContactCMD*

# Test universal under Rosetta (on Apple Silicon)
arch -x86_64 /Volumes/ContactCMD*/contactcmd --version
```

### Verify Checksums

```bash
cd release
shasum -a 256 -c checksums.txt
```

## Publishing to GitHub Releases

After testing, create a GitHub release:

```bash
# Using GitHub CLI
gh release create v0.1.0 \
  --title "ContactCMD v0.1.0" \
  --notes "Release notes here..." \
  release/contactcmd-0.1.0-*.dmg \
  release/checksums.txt
```

Or manually:
1. Go to https://github.com/jnun/contactcmd/releases/new
2. Choose tag: `v0.1.0`
3. Upload the DMG files and checksums.txt
4. Write release notes
5. Publish

## Troubleshooting

### "error: linker `cc` failed" during Intel build on Apple Silicon

Install Rosetta and Xcode Command Line Tools:
```bash
softwareupdate --install-rosetta
xcode-select --install
```

### "create-dmg: command not found"

```bash
brew install create-dmg
```

### Notarization fails with "invalid credentials"

- Verify your App-Specific Password is correct
- Ensure you're using an App-Specific Password, not your Apple ID password
- Check that your Apple Developer account is in good standing

### DMG won't open on user's Mac

The binary may not be signed or notarized. Users can bypass Gatekeeper:
1. Right-click the app in Finder
2. Select "Open"
3. Click "Open" in the dialog

Or via Terminal:
```bash
xattr -d com.apple.quarantine /path/to/contactcmd
```

## Release Checklist

- [ ] Update version in `Cargo.toml`
- [ ] Update CHANGELOG.md (if maintained)
- [ ] Commit version bump
- [ ] Create and push git tag
- [ ] Run `./scripts/build-release.sh --clean`
- [ ] Test Intel DMG (on Intel Mac or via Rosetta)
- [ ] Test Apple Silicon DMG
- [ ] Test Universal DMG
- [ ] Verify checksums
- [ ] Create GitHub release
- [ ] Update download links in README
- [ ] Announce release (if applicable)
