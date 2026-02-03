#!/bin/bash
#
# build-release.sh - Build and package ContactCMD for macOS distribution
#
# This script creates DMG installers for Intel Macs, Apple Silicon Macs,
# and a universal binary that runs on both architectures.
#
# PREREQUISITES:
#   - Rust toolchain with both targets installed:
#       rustup target add x86_64-apple-darwin aarch64-apple-darwin
#   - create-dmg (install via: brew install create-dmg)
#   - Optional: Developer ID certificate for code signing
#
# USAGE:
#   ./scripts/build-release.sh [OPTIONS]
#
# OPTIONS:
#   --intel-only      Build only for Intel Macs (x86_64)
#   --arm-only        Build only for Apple Silicon (aarch64)
#   --universal-only  Build only the universal binary
#   --skip-dmg        Build binaries but skip DMG creation
#   --sign            Code sign the binaries (requires DEVELOPER_ID env var)
#   --notarize        Notarize the DMGs (requires APPLE_ID, APPLE_PASSWORD, TEAM_ID)
#   --clean           Clean build artifacts before building
#   --help            Show this help message
#
# ENVIRONMENT VARIABLES:
#   DEVELOPER_ID      - Developer ID for code signing (e.g., "Developer ID Application: Your Name (TEAMID)")
#   APPLE_ID          - Apple ID email for notarization
#   APPLE_PASSWORD    - App-specific password for notarization
#   TEAM_ID           - Apple Developer Team ID
#
# OUTPUT:
#   release/
#   ├── contactcmd-VERSION-intel.dmg       # Intel-only DMG
#   ├── contactcmd-VERSION-arm64.dmg       # Apple Silicon DMG
#   ├── contactcmd-VERSION-universal.dmg   # Universal binary DMG
#   └── checksums.txt                      # SHA256 checksums for all files
#

set -euo pipefail

# =============================================================================
# Configuration
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RELEASE_DIR="$PROJECT_ROOT/release"
STAGING_DIR="$PROJECT_ROOT/.release-staging"

# Binary and package names
BINARY_NAME="contactcmd"
APP_NAME="ContactCMD"

# Extract version from Cargo.toml
VERSION=$(grep '^version' "$PROJECT_ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')

# Target architectures
TARGET_INTEL="x86_64-apple-darwin"
TARGET_ARM="aarch64-apple-darwin"

# Build flags
BUILD_INTEL=true
BUILD_ARM=true
BUILD_UNIVERSAL=true
CREATE_DMG=true
SIGN_BINARY=false
NOTARIZE=false
CLEAN_FIRST=false

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# =============================================================================
# Helper Functions
# =============================================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

show_help() {
    head -50 "$0" | grep -E '^#' | sed 's/^# \?//'
    exit 0
}

check_prerequisites() {
    log_info "Checking prerequisites..."

    local missing=()

    # Check for Rust
    if ! command -v cargo &> /dev/null; then
        missing+=("cargo (Rust toolchain)")
    fi

    # Check for create-dmg
    if $CREATE_DMG && ! command -v create-dmg &> /dev/null; then
        missing+=("create-dmg (brew install create-dmg)")
    fi

    # Check for lipo (should be present on macOS)
    if $BUILD_UNIVERSAL && ! command -v lipo &> /dev/null; then
        missing+=("lipo (Xcode Command Line Tools)")
    fi

    # Check Rust targets (handle both rustup and Homebrew installations)
    if command -v rustup &> /dev/null; then
        # rustup-based installation: check installed targets
        if $BUILD_INTEL; then
            if ! rustup target list --installed | grep -q "$TARGET_INTEL"; then
                missing+=("Rust target $TARGET_INTEL (rustup target add $TARGET_INTEL)")
            fi
        fi

        if $BUILD_ARM; then
            if ! rustup target list --installed | grep -q "$TARGET_ARM"; then
                missing+=("Rust target $TARGET_ARM (rustup target add $TARGET_ARM)")
            fi
        fi
    else
        # Homebrew or other installation: check if targets are supported
        if $BUILD_INTEL; then
            if ! rustc --print target-list | grep -q "$TARGET_INTEL"; then
                missing+=("Rust target $TARGET_INTEL not supported by this Rust installation")
            fi
        fi

        if $BUILD_ARM; then
            if ! rustc --print target-list | grep -q "$TARGET_ARM"; then
                missing+=("Rust target $TARGET_ARM not supported by this Rust installation")
            fi
        fi
    fi

    if [ ${#missing[@]} -gt 0 ]; then
        log_error "Missing prerequisites:"
        for item in "${missing[@]}"; do
            echo "  - $item"
        done
        exit 1
    fi

    log_success "All prerequisites satisfied"
}

clean_build() {
    log_info "Cleaning previous build artifacts..."
    rm -rf "$STAGING_DIR"
    rm -rf "$RELEASE_DIR"
    cargo clean
    log_success "Clean complete"
}

build_target() {
    local target=$1
    local arch_name=$2

    log_info "Building for $arch_name ($target)..."

    cd "$PROJECT_ROOT"
    cargo build --release --target "$target"

    local binary_path="$PROJECT_ROOT/target/$target/release/$BINARY_NAME"

    if [ ! -f "$binary_path" ]; then
        log_error "Build failed: binary not found at $binary_path"
        exit 1
    fi

    log_success "Built $arch_name binary: $(du -h "$binary_path" | cut -f1)"
}

sign_binary() {
    local binary_path=$1

    if [ -z "${DEVELOPER_ID:-}" ]; then
        log_warn "DEVELOPER_ID not set, skipping code signing"
        return 0
    fi

    log_info "Signing binary: $binary_path"
    codesign --force --options runtime --sign "$DEVELOPER_ID" "$binary_path"

    # Verify signature
    if codesign --verify --verbose "$binary_path" 2>/dev/null; then
        log_success "Binary signed and verified"
    else
        log_error "Signature verification failed"
        exit 1
    fi
}

create_universal_binary() {
    log_info "Creating universal binary..."

    local intel_binary="$PROJECT_ROOT/target/$TARGET_INTEL/release/$BINARY_NAME"
    local arm_binary="$PROJECT_ROOT/target/$TARGET_ARM/release/$BINARY_NAME"
    local universal_binary="$STAGING_DIR/universal/$BINARY_NAME"

    mkdir -p "$STAGING_DIR/universal"

    lipo -create "$intel_binary" "$arm_binary" -output "$universal_binary"

    # Verify universal binary
    local archs=$(lipo -archs "$universal_binary")
    log_success "Created universal binary with architectures: $archs"
}

create_dmg_package() {
    local binary_path=$1
    local output_name=$2
    local arch_label=$3

    log_info "Creating DMG: $output_name"

    local dmg_staging="$STAGING_DIR/dmg-$arch_label"
    local dmg_output="$RELEASE_DIR/$output_name"

    # Create staging directory with binary and README
    mkdir -p "$dmg_staging"
    cp "$binary_path" "$dmg_staging/"

    # Create installation instructions
    cat > "$dmg_staging/INSTALL.txt" << 'EOF'
ContactCMD Installation
=======================

To install ContactCMD, copy the binary to a directory in your PATH.

Quick install (recommended):

    # Create local bin directory if it doesn't exist
    mkdir -p ~/.local/bin

    # Copy the binary
    cp contactcmd ~/.local/bin/

    # Add to PATH (add to ~/.zshrc or ~/.bashrc for persistence)
    export PATH="$HOME/.local/bin:$PATH"

Alternative: Install to /usr/local/bin (requires sudo):

    sudo cp contactcmd /usr/local/bin/

Verify installation:

    contactcmd --version

For more information, visit: https://github.com/jnun/contactcmd
EOF

    # Create the DMG
    # Remove existing DMG if present
    rm -f "$dmg_output"

    create-dmg \
        --volname "$APP_NAME $VERSION ($arch_label)" \
        --window-pos 200 120 \
        --window-size 500 350 \
        --icon-size 80 \
        --hide-extension "$BINARY_NAME" \
        --no-internet-enable \
        "$dmg_output" \
        "$dmg_staging/"

    if [ -f "$dmg_output" ]; then
        log_success "Created: $dmg_output ($(du -h "$dmg_output" | cut -f1))"
    else
        log_error "Failed to create DMG: $dmg_output"
        exit 1
    fi
}

notarize_dmg() {
    local dmg_path=$1

    if [ -z "${APPLE_ID:-}" ] || [ -z "${APPLE_PASSWORD:-}" ] || [ -z "${TEAM_ID:-}" ]; then
        log_warn "Notarization credentials not set, skipping"
        return 0
    fi

    log_info "Submitting for notarization: $dmg_path"

    xcrun notarytool submit "$dmg_path" \
        --apple-id "$APPLE_ID" \
        --password "$APPLE_PASSWORD" \
        --team-id "$TEAM_ID" \
        --wait

    # Staple the notarization ticket
    xcrun stapler staple "$dmg_path"

    log_success "Notarization complete"
}

generate_checksums() {
    log_info "Generating checksums..."

    cd "$RELEASE_DIR"
    shasum -a 256 *.dmg > checksums.txt

    log_success "Checksums written to checksums.txt"
    cat checksums.txt
}

# =============================================================================
# Argument Parsing
# =============================================================================

while [[ $# -gt 0 ]]; do
    case $1 in
        --intel-only)
            BUILD_INTEL=true
            BUILD_ARM=false
            BUILD_UNIVERSAL=false
            shift
            ;;
        --arm-only)
            BUILD_INTEL=false
            BUILD_ARM=true
            BUILD_UNIVERSAL=false
            shift
            ;;
        --universal-only)
            BUILD_INTEL=true
            BUILD_ARM=true
            BUILD_UNIVERSAL=true
            # Still need to build both for universal
            shift
            ;;
        --skip-dmg)
            CREATE_DMG=false
            shift
            ;;
        --sign)
            SIGN_BINARY=true
            shift
            ;;
        --notarize)
            NOTARIZE=true
            SIGN_BINARY=true  # Notarization requires signing
            shift
            ;;
        --clean)
            CLEAN_FIRST=true
            shift
            ;;
        --help|-h)
            show_help
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# =============================================================================
# Main Build Process
# =============================================================================

main() {
    echo ""
    echo "=========================================="
    echo "  $APP_NAME Release Builder v$VERSION"
    echo "=========================================="
    echo ""

    # Setup
    check_prerequisites

    if $CLEAN_FIRST; then
        clean_build
    fi

    mkdir -p "$RELEASE_DIR"
    mkdir -p "$STAGING_DIR"

    # Build binaries
    if $BUILD_INTEL; then
        build_target "$TARGET_INTEL" "Intel"
        if $SIGN_BINARY; then
            sign_binary "$PROJECT_ROOT/target/$TARGET_INTEL/release/$BINARY_NAME"
        fi
    fi

    if $BUILD_ARM; then
        build_target "$TARGET_ARM" "Apple Silicon"
        if $SIGN_BINARY; then
            sign_binary "$PROJECT_ROOT/target/$TARGET_ARM/release/$BINARY_NAME"
        fi
    fi

    # Create universal binary if both architectures were built
    if $BUILD_UNIVERSAL && $BUILD_INTEL && $BUILD_ARM; then
        create_universal_binary
        if $SIGN_BINARY; then
            sign_binary "$STAGING_DIR/universal/$BINARY_NAME"
        fi
    fi

    # Create DMGs
    if $CREATE_DMG; then
        if $BUILD_INTEL; then
            create_dmg_package \
                "$PROJECT_ROOT/target/$TARGET_INTEL/release/$BINARY_NAME" \
                "$BINARY_NAME-$VERSION-intel.dmg" \
                "Intel"

            if $NOTARIZE; then
                notarize_dmg "$RELEASE_DIR/$BINARY_NAME-$VERSION-intel.dmg"
            fi
        fi

        if $BUILD_ARM; then
            create_dmg_package \
                "$PROJECT_ROOT/target/$TARGET_ARM/release/$BINARY_NAME" \
                "$BINARY_NAME-$VERSION-arm64.dmg" \
                "Apple Silicon"

            if $NOTARIZE; then
                notarize_dmg "$RELEASE_DIR/$BINARY_NAME-$VERSION-arm64.dmg"
            fi
        fi

        if $BUILD_UNIVERSAL && $BUILD_INTEL && $BUILD_ARM; then
            create_dmg_package \
                "$STAGING_DIR/universal/$BINARY_NAME" \
                "$BINARY_NAME-$VERSION-universal.dmg" \
                "Universal"

            if $NOTARIZE; then
                notarize_dmg "$RELEASE_DIR/$BINARY_NAME-$VERSION-universal.dmg"
            fi
        fi

        # Generate checksums
        generate_checksums
    fi

    # Cleanup staging
    rm -rf "$STAGING_DIR"

    # Summary
    echo ""
    echo "=========================================="
    echo "  Build Complete!"
    echo "=========================================="
    echo ""
    echo "Release artifacts in: $RELEASE_DIR/"
    ls -lh "$RELEASE_DIR/"
    echo ""

    if $CREATE_DMG; then
        echo "Next steps:"
        echo "  1. Test the DMGs on both Intel and Apple Silicon Macs"
        echo "  2. Upload to GitHub Releases or your distribution channel"
        echo "  3. Update download links in documentation"
        echo ""
    fi
}

main
