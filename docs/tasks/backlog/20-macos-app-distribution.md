# Task 20: macOS App Distribution

**Feature:** none
**Created:** 2026-01-27

## Problem

The CLI currently runs as a raw binary that inherits permissions from the terminal. This requires users to grant Full Disk Access to their terminal app (Terminal.app, iTerm, etc.) to enable features like Messages integration - which is overly broad and a security concern.

Packaging contactcmd as a signed macOS application bundle (.app) would allow it to request its own specific permissions, separate from the terminal. Users would grant Full Disk Access to `contactcmd.app` only, not their entire terminal.

## Success criteria

- [ ] Application bundle structure created (`contactcmd.app`)
- [ ] Binary embedded in `Contents/MacOS/`
- [ ] Proper `Info.plist` with bundle identifier, version, entitlements
- [ ] Code signing with Developer ID (or self-signed for local use)
- [ ] App can be invoked from command line via symlink or PATH
- [ ] App requests its own Full Disk Access permission (separate from Terminal)
- [ ] Notarization for distribution outside App Store (optional)
- [ ] Homebrew formula or install script for easy installation
- [ ] Documentation for manual installation

## Notes

The app bundle approach allows:
- Specific permission grants to `contactcmd.app` rather than Terminal
- Proper macOS integration with System Settings > Privacy & Security
- Distribution via Homebrew cask or direct download

Reference: https://developer.apple.com/documentation/bundleresources/placing_content_in_a_bundle

Consider using `cargo-bundle` or a custom build script:
```bash
cargo install cargo-bundle
```

Alternative: Create a minimal Swift wrapper app that embeds the Rust binary.
