#!/bin/sh
# howtui installer for Linux and macOS (SRS 14.1). No sudo required.
#
#   curl -fsSL https://raw.githubusercontent.com/hungryZoo/howtui/main/scripts/install.sh | sh
#
# Review first:  curl -fsSLO https://raw.githubusercontent.com/hungryZoo/howtui/main/scripts/install.sh
#                less install.sh && sh install.sh
#
# Options:
#   --version X.Y.Z   install a specific release (default: latest)
#   --bin-dir DIR     install directory (default: ~/.local/bin)
#   --print-target    print the detected release target and exit
# Environment:
#   HOWTUI_VERSION, HOWTUI_INSTALL_DIR
#
# This script only downloads the release archive and SHA256SUMS of this repository over HTTPS,
# verifies the checksum, and copies the binary. It never edits shell rc files.
set -eu

REPO="hungryZoo/howtui"
BIN="howtui"
BASE_URL="${HOWTUI_DOWNLOAD_BASE:-https://github.com/${REPO}/releases/download}"
LATEST_URL="${HOWTUI_LATEST_URL:-https://github.com/${REPO}/releases/latest}"
BIN_DIR="${HOWTUI_INSTALL_DIR:-${HOME}/.local/bin}"
VERSION="${HOWTUI_VERSION:-}"
PRINT_TARGET=0
TMP_DIR=""

cleanup() {
    if [ -n "$TMP_DIR" ] && [ -d "$TMP_DIR" ]; then
        rm -rf "$TMP_DIR"
    fi
}
trap cleanup EXIT INT TERM

err() {
    echo "howtui install: error: $*" >&2
    exit 1
}

info() {
    echo "==> $*"
}

need() {
    command -v "$1" >/dev/null 2>&1 || err "required command not found: $1"
}

usage() {
    cat <<'EOF'
howtui installer (Linux, macOS). No sudo required.

Usage: install.sh [--version X.Y.Z] [--bin-dir DIR] [--print-target]

Environment: HOWTUI_VERSION, HOWTUI_INSTALL_DIR
EOF
}

# Userland word size from the ELF class byte (1 = 32-bit, 2 = 64-bit) of a running binary.
elf_class() {
    if [ -n "${HOWTUI_TEST_ELF_CLASS:-}" ]; then
        echo "$HOWTUI_TEST_ELF_CLASS"
        return
    fi
    for f in /proc/self/exe /bin/sh; do
        if [ -r "$f" ]; then
            c=$(dd if="$f" bs=1 skip=4 count=1 2>/dev/null | od -An -tu1 | tr -d ' \n')
            if [ -n "$c" ]; then
                echo "$c"
                return
            fi
        fi
    done
    echo 0
}

is_musl() {
    if [ -n "${HOWTUI_TEST_MUSL:-}" ]; then
        [ "$HOWTUI_TEST_MUSL" = 1 ]
        return
    fi
    ldd --version 2>&1 | grep -qi musl
}

detect_target() {
    os="${HOWTUI_TEST_UNAME_S:-$(uname -s)}"
    arch="${HOWTUI_TEST_UNAME_M:-$(uname -m)}"
    case "$os" in
        Darwin)
            hw_arm64="${HOWTUI_TEST_HW_ARM64:-$(sysctl -n hw.optional.arm64 2>/dev/null || echo 0)}"
            if [ "$arch" = arm64 ] || [ "$arch" = aarch64 ] || [ "$hw_arm64" = 1 ]; then
                echo "aarch64-apple-darwin"
            elif [ "$arch" = x86_64 ]; then
                echo "x86_64-apple-darwin"
            else
                err "unsupported macOS architecture: $arch"
            fi
            ;;
        Linux)
            if is_musl; then
                err "musl-based Linux is not supported (release binaries require glibc 2.17 or newer)"
            fi
            class=$(elf_class)
            case "$arch" in
                x86_64 | amd64)
                    [ "$class" = 1 ] && err "32-bit x86 userland is not supported"
                    echo "x86_64-unknown-linux-gnu"
                    ;;
                aarch64 | arm64)
                    # 64-bit kernel with a 32-bit Raspberry Pi OS userland: use the ARMv7 build
                    if [ "$class" = 1 ]; then echo "armv7-unknown-linux-gnueabihf"; else echo "aarch64-unknown-linux-gnu"; fi
                    ;;
                armv7l | armv8l)
                    echo "armv7-unknown-linux-gnueabihf"
                    ;;
                armv6l)
                    err "ARMv6 (Raspberry Pi 1, Zero, Zero W) is not supported"
                    ;;
                *)
                    err "unsupported Linux architecture: $arch"
                    ;;
            esac
            ;;
        *)
            err "unsupported OS: $os (use install.ps1 on Windows)"
            ;;
    esac
}

latest_version() {
    final=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "$LATEST_URL" 2>/dev/null || true)
    tag="${final##*/tag/}"
    if [ -n "$final" ] && [ "$tag" != "$final" ] && [ -n "$tag" ]; then
        echo "${tag#v}"
        return
    fi
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null \
        | grep '"tag_name"' | head -1 | cut -d '"' -f 4 | sed 's/^v//'
}

sha256_of() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d ' ' -f 1
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | cut -d ' ' -f 1
    else
        err "sha256sum or shasum is required to verify the download"
    fi
}

parse_args() {
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --version) [ "$#" -ge 2 ] || err "--version needs a value"; VERSION="$2"; shift 2 ;;
            --version=*) VERSION="${1#*=}"; shift ;;
            --bin-dir) [ "$#" -ge 2 ] || err "--bin-dir needs a value"; BIN_DIR="$2"; shift 2 ;;
            --bin-dir=*) BIN_DIR="${1#*=}"; shift ;;
            --print-target) PRINT_TARGET=1; shift ;;
            -h | --help) usage; exit 0 ;;
            *) err "unknown option: $1 (try --help)" ;;
        esac
    done
}

main() {
    parse_args "$@"
    target=$(detect_target)
    if [ "$PRINT_TARGET" = 1 ]; then
        echo "$target"
        return
    fi
    need curl
    need tar
    info "target: $target"

    if [ -z "$VERSION" ]; then
        VERSION=$(latest_version)
    fi
    VERSION="${VERSION#v}"
    [ -n "$VERSION" ] || err "could not determine the latest release version"
    info "version: $VERSION"

    asset="${BIN}-${VERSION}-${target}.tar.gz"
    TMP_DIR=$(mktemp -d)
    info "downloading $asset"
    curl -fsSL "${BASE_URL}/v${VERSION}/${asset}" -o "$TMP_DIR/$asset" || err "download failed: no release asset for $target (v$VERSION)"
    curl -fsSL "${BASE_URL}/v${VERSION}/SHA256SUMS" -o "$TMP_DIR/SHA256SUMS" || err "download failed: SHA256SUMS (v$VERSION)"

    expected=$(awk -v f="$asset" '$2 == f || $2 == "*" f { print $1 }' "$TMP_DIR/SHA256SUMS" | head -1)
    [ -n "$expected" ] || err "SHA256SUMS has no entry for $asset"
    actual=$(sha256_of "$TMP_DIR/$asset")
    [ "$expected" = "$actual" ] || err "checksum mismatch for $asset (expected $expected, got $actual)"
    info "checksum verified"

    tar -xzf "$TMP_DIR/$asset" -C "$TMP_DIR" "$BIN" || err "archive does not contain $BIN"
    mkdir -p "$BIN_DIR"
    # 같은 디렉터리 임시 파일 후 이름 변경 (실패 시 기존 바이너리 유지)
    staged="$BIN_DIR/.$BIN.new.$$"
    cp "$TMP_DIR/$BIN" "$staged" || err "cannot write to $BIN_DIR"
    chmod 0755 "$staged"
    mv -f "$staged" "$BIN_DIR/$BIN" || { rm -f "$staged"; err "cannot replace $BIN_DIR/$BIN"; }

    installed=$("$BIN_DIR/$BIN" --version 2>/dev/null || true)
    [ "$installed" = "$BIN $VERSION" ] || err "installed binary reports '$installed', expected '$BIN $VERSION'"
    info "installed $installed to $BIN_DIR/$BIN"

    case ":$PATH:" in
        *":$BIN_DIR:"*) ;;
        *)
            echo
            echo "Note: $BIN_DIR is not on your PATH. Add this line to your shell profile:"
            echo "  export PATH=\"$BIN_DIR:\$PATH\""
            ;;
    esac
    echo
    echo "Run: $BIN"
}

main "$@"
