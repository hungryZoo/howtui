#!/bin/sh
# howtui uninstaller for Linux and macOS. Installed next to the binary as howtui-uninstall.sh.
#
#   howtui-uninstall.sh            ask, then remove
#   howtui-uninstall.sh -y         remove without asking
#   howtui-uninstall.sh --purge    also delete the config directory
#
# Removes: the saved sign-in (OS credential store or file), the shell integration blocks that
# `howtui init <shell> --install` added, the binary, this script and the data directory.
# Keeps the config directory unless --purge is given. Never touches anything else.
set -eu

YES=0
PURGE=0
for arg in "$@"; do
    case "$arg" in
        -y | --yes) YES=1 ;;
        --purge) PURGE=1 ;;
        -h | --help)
            sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *)
            echo "howtui uninstall: unknown option: $arg (try --help)" >&2
            exit 2
            ;;
    esac
done

SELF=$0
DIR=$(cd "$(dirname "$SELF")" && pwd)
BIN="$DIR/howtui"
[ -x "$BIN" ] || BIN=$(command -v howtui 2>/dev/null || true)
DATA_DIR="${HOWTUI_DATA_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/howtui}"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/howtui"

echo "This removes howtui:"
echo "  - saved sign-in (signs out and deletes the tokens)"
echo "  - shell integration blocks in your bash and zsh startup files (only the marked blocks)"
if [ -n "$BIN" ]; then
    echo "  - $BIN"
    echo "  - $(dirname "$BIN")/howtui-uninstall.sh"
fi
echo "  - $DATA_DIR"
if [ "$PURGE" = 1 ]; then
    echo "  - $CONFIG_DIR"
elif [ -d "$CONFIG_DIR" ]; then
    echo "Kept: $CONFIG_DIR (use --purge to delete it)"
fi

if [ "$YES" != 1 ]; then
    if [ "${HOWTUI_TEST_NO_TTY:-0}" != 1 ] && (exec </dev/tty) 2>/dev/null; then
        printf 'Continue? [y/N] '
        read -r answer </dev/tty || answer=n
    else
        echo "howtui uninstall: no terminal to confirm; run again with -y" >&2
        exit 1
    fi
    case "$answer" in
        y | Y | yes | Yes | YES) ;;
        *)
            echo "Nothing removed."
            exit 0
            ;;
    esac
fi

if [ -n "$BIN" ] && [ -x "$BIN" ]; then
    # 실패해도 계속 진행 (네트워크 없음, 이미 로그아웃 등)
    "$BIN" logout --forget </dev/null || true
    for shell in bash zsh; do
        "$BIN" init "$shell" --uninstall || true
    done
fi
if [ -n "$BIN" ]; then
    rm -f "$BIN" "$(dirname "$BIN")/howtui-uninstall.sh"
fi
# howtui가 만드는 파일만 삭제 후 빈 디렉터리 제거 (경로 재정의가 잘못되어도 다른 파일 보존)
rm -f "$DATA_DIR/state.json" "$DATA_DIR/credentials.json" "$DATA_DIR/auth.lock" "$DATA_DIR"/.*.tmp-* "$DATA_DIR/logs/howtui.log"
rmdir "$DATA_DIR/logs" 2>/dev/null || true
rmdir "$DATA_DIR" 2>/dev/null || [ ! -d "$DATA_DIR" ] || echo "Kept $DATA_DIR (it contains other files)"
if [ "$PURGE" = 1 ]; then
    rm -f "$CONFIG_DIR/config.toml"
    rmdir "$CONFIG_DIR" 2>/dev/null || [ ! -d "$CONFIG_DIR" ] || echo "Kept $CONFIG_DIR (it contains other files)"
fi
echo "howtui removed. Open a new terminal so the shell integration is gone from this session too."
