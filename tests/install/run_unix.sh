#!/usr/bin/env sh
# install.sh 검증 (TC-INS-001, 002, 003, TC-SH-009): 로컬 가짜 릴리스(file:// URL)로 설치, 재설치, 변조 감지, rc 파일 무변경 확인.
# 사용: tests/install/run_unix.sh <howtui 바이너리 경로>
set -eu
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN=${1:?howtui binary path}
VERSION=$("$BIN" --version | cut -d ' ' -f 2)
TARGET=$(sh "$ROOT/scripts/install.sh" --print-target)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
fail() { echo "FAIL $*"; exit 1; }

mkdir -p "$WORK/dist/v$VERSION" "$WORK/pkg" "$WORK/home"
cp "$BIN" "$WORK/pkg/howtui"
cp "$ROOT/README.md" "$ROOT/LICENSE" "$WORK/pkg/"
asset="howtui-$VERSION-$TARGET.tar.gz"
(cd "$WORK/pkg" && tar -czf "$WORK/dist/v$VERSION/$asset" howtui README.md LICENSE)
(cd "$WORK/dist/v$VERSION" && { sha256sum "$asset" 2>/dev/null || shasum -a 256 "$asset"; } > SHA256SUMS)
printf 'export TEST=1\n' > "$WORK/home/.bashrc"
before=$(cat "$WORK/home/.bashrc")

run_install() {
    env -i PATH=/usr/bin:/bin HOME="$WORK/home" HOWTUI_DOWNLOAD_BASE="file://$WORK/dist" HOWTUI_VERSION="$VERSION" \
        sh "$ROOT/scripts/install.sh" "$@"
}

run_install > "$WORK/out1" 2>&1 || { cat "$WORK/out1"; fail "install"; }
[ "$("$WORK/home/.local/bin/howtui" --version)" = "howtui $VERSION" ] || fail "installed version"
grep -q "checksum verified" "$WORK/out1" || fail "checksum message"
echo "PASS install ($TARGET)"

run_install --bin-dir "$WORK/custom" > "$WORK/out2" 2>&1 || { cat "$WORK/out2"; fail "reinstall"; }
[ -x "$WORK/custom/howtui" ] || fail "custom bin dir"
grep -q "not on your PATH" "$WORK/out2" || fail "PATH guidance"
echo "PASS bin-dir and PATH guidance"

# 변조: 체크섬 불일치 시 중단, 기존 바이너리 유지
printf 'tampered' >> "$WORK/dist/v$VERSION/$asset"
if run_install > "$WORK/out3" 2>&1; then fail "tampered archive accepted"; fi
grep -q "checksum mismatch" "$WORK/out3" || { cat "$WORK/out3"; fail "mismatch message"; }
[ "$("$WORK/home/.local/bin/howtui" --version)" = "howtui $VERSION" ] || fail "existing binary damaged"
echo "PASS tamper detection"

[ "$(cat "$WORK/home/.bashrc")" = "$before" ] || fail ".bashrc modified"
[ ! -e "$WORK/home/.zshrc" ] && [ ! -e "$WORK/home/.profile" ] || fail "profile created"
echo "PASS rc files untouched"
