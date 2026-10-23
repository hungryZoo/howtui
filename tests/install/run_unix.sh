#!/usr/bin/env sh
# install.sh 검증 (TC-INS-001, 002, 003, TC-SH-009, TC-INS-011): 로컬 가짜 릴리스(file:// URL)로 설치, 재설치, 변조 감지,
# rc 파일 무변경(터미널 없음, 거절), 동의 시 표식 구간 추가 확인.
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
cp "$ROOT/scripts/uninstall.sh" "$WORK/pkg/uninstall.sh"
asset="howtui-$VERSION-$TARGET.tar.gz"
(cd "$WORK/pkg" && tar -czf "$WORK/dist/v$VERSION/$asset" howtui uninstall.sh README.md LICENSE)
(cd "$WORK/dist/v$VERSION" && { sha256sum "$asset" 2>/dev/null || shasum -a 256 "$asset"; } > SHA256SUMS)
printf 'export TEST=1\n' > "$WORK/home/.bashrc"
before=$(cat "$WORK/home/.bashrc")

# 기본: 대화형 터미널 없음 (로컬 터미널에서 실행해도 질문이 나오지 않도록 고정)
run_install() {
    env -i PATH=/usr/bin:/bin HOME="$WORK/home" SHELL="${TEST_SHELL:-/bin/bash}" HOWTUI_TEST_NO_TTY=1 ${EXTRA_ENV:-} \
        HOWTUI_DOWNLOAD_BASE="file://$WORK/dist" HOWTUI_VERSION="$VERSION" \
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
grep -q "howtui init bash --install" "$WORK/out1" || fail "no-terminal hint"
echo "PASS rc files untouched without a terminal"

# 동의 질문 (CR-004): 거절 시 무변경, 동의 시 표식 구간 1개, 재실행 시 중복 없음, 제거 시 원래 내용
(cd "$WORK/pkg" && tar -czf "$WORK/dist/v$VERSION/$asset" howtui uninstall.sh README.md LICENSE)
(cd "$WORK/dist/v$VERSION" && { sha256sum "$asset" 2>/dev/null || shasum -a 256 "$asset"; } > SHA256SUMS)
rcfile=.bashrc
[ "$(uname -s)" = Darwin ] && rcfile=.bash_profile
[ -e "$WORK/home/$rcfile" ] || printf 'export TEST=1\n' > "$WORK/home/$rcfile"
orig=$(cat "$WORK/home/$rcfile")
EXTRA_ENV="HOWTUI_TEST_ANSWER=n" run_install > "$WORK/out4" 2>&1 || { cat "$WORK/out4"; fail "install (answer n)"; }
[ "$(cat "$WORK/home/$rcfile")" = "$orig" ] || fail "rc modified after answering no"
grep -q "Skipped" "$WORK/out4" || fail "skip message"
echo "PASS answer no keeps rc"
EXTRA_ENV="HOWTUI_TEST_ANSWER=y" run_install > "$WORK/out5" 2>&1 || { cat "$WORK/out5"; fail "install (answer y)"; }
EXTRA_ENV="HOWTUI_TEST_ANSWER=yes" run_install > "$WORK/out6" 2>&1 || { cat "$WORK/out6"; fail "install (answer yes again)"; }
[ "$(grep -c '>>> howtui shell integration >>>' "$WORK/home/$rcfile")" = 1 ] || fail "block count"
grep -q 'eval "$(howtui init bash)"' "$WORK/home/$rcfile" || fail "init line"
grep -q "__howtui_dir='$WORK/home/.local/bin'" "$WORK/home/$rcfile" || fail "PATH line for a bin dir outside PATH"
grep -q "already in" "$WORK/out6" || fail "second run reports existing block"
echo "PASS answer yes adds one block"
env -i PATH=/usr/bin:/bin HOME="$WORK/home" "$WORK/home/.local/bin/howtui" init bash --uninstall > /dev/null || fail "uninstall"
[ "$(cat "$WORK/home/$rcfile")" = "$orig" ] || fail "uninstall did not restore rc"
echo "PASS uninstall restores rc"
TEST_SHELL=/bin/zsh run_install --shell-integration > "$WORK/out7" 2>&1 || { cat "$WORK/out7"; fail "install --shell-integration"; }
grep -q 'eval "$(howtui init zsh)"' "$WORK/home/.zshrc" || fail "zsh block"
EXTRA_ENV="HOWTUI_TEST_ANSWER=y" run_install --no-shell-integration > "$WORK/out8" 2>&1 || fail "install --no-shell-integration"
[ "$(cat "$WORK/home/$rcfile")" = "$orig" ] || fail "--no-shell-integration modified rc"
echo "PASS --shell-integration and --no-shell-integration"

# 제거 스크립트 (TC-INS-012): 함께 설치, 터미널 없으면 -y 요구, 실행 후 바이너리와 rc 구간과 데이터 삭제, 설정 유지
[ -x "$WORK/home/.local/bin/howtui-uninstall.sh" ] || fail "uninstall script installed"
grep -q "Uninstall: .*howtui-uninstall.sh" "$WORK/out8" || fail "uninstall hint"
EXTRA_ENV="HOWTUI_TEST_ANSWER=y" run_install > /dev/null 2>&1 || fail "reinstall with integration"
mkdir -p "$WORK/home/.local/share/howtui/logs" "$WORK/home/.config/howtui"
printf '{}' > "$WORK/home/.local/share/howtui/state.json"
printf 'x' > "$WORK/home/.local/share/howtui/logs/howtui.log"
printf '[ui]\n' > "$WORK/home/.config/howtui/config.toml"
un() { env -i PATH=/usr/bin:/bin HOME="$WORK/home" HOWTUI_TEST_NO_TTY=1 sh "$WORK/home/.local/bin/howtui-uninstall.sh" "$@"; }
if un > "$WORK/out9" 2>&1; then fail "uninstall ran without confirmation"; fi
[ -x "$WORK/home/.local/bin/howtui" ] || fail "binary removed without confirmation"
un -y > "$WORK/out10" 2>&1 || { cat "$WORK/out10"; fail "uninstall -y"; }
[ ! -e "$WORK/home/.local/bin/howtui" ] && [ ! -e "$WORK/home/.local/bin/howtui-uninstall.sh" ] || fail "binary or script left"
[ "$(cat "$WORK/home/$rcfile")" = "$orig" ] || fail "rc block left after uninstall"
! grep -q "howtui shell integration" "$WORK/home/.zshrc" || fail "zsh block left after uninstall"
[ ! -e "$WORK/home/.local/share/howtui" ] || fail "data dir left"
[ -f "$WORK/home/.config/howtui/config.toml" ] || fail "config removed without --purge"
grep -q "Kept: .*config/howtui" "$WORK/out10" || fail "kept config message"
echo "PASS uninstall script"
