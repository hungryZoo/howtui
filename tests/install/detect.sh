#!/usr/bin/env sh
# install.sh 플랫폼 판별 표 검증 (TC-INS-004, SRS 14.2절).
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
fail=0
check() {
    expect=$1
    shift
    got=$(env -i PATH=/usr/bin:/bin HOME=/tmp "$@" sh "$ROOT/scripts/install.sh" --print-target 2>&1)
    case "$got" in
        *"$expect"*) echo "PASS $* -> $expect" ;;
        *) echo "FAIL $* -> $got (expected $expect)"; fail=1 ;;
    esac
}
L="HOWTUI_TEST_UNAME_S=Linux HOWTUI_TEST_MUSL=0"
check x86_64-unknown-linux-gnu $L HOWTUI_TEST_UNAME_M=x86_64 HOWTUI_TEST_ELF_CLASS=2
check aarch64-unknown-linux-gnu $L HOWTUI_TEST_UNAME_M=aarch64 HOWTUI_TEST_ELF_CLASS=2
check armv7-unknown-linux-gnueabihf $L HOWTUI_TEST_UNAME_M=aarch64 HOWTUI_TEST_ELF_CLASS=1
check armv7-unknown-linux-gnueabihf $L HOWTUI_TEST_UNAME_M=armv7l HOWTUI_TEST_ELF_CLASS=1
check armv7-unknown-linux-gnueabihf $L HOWTUI_TEST_UNAME_M=armv8l HOWTUI_TEST_ELF_CLASS=1
check "ARMv6" $L HOWTUI_TEST_UNAME_M=armv6l HOWTUI_TEST_ELF_CLASS=1
check "unsupported Linux architecture" $L HOWTUI_TEST_UNAME_M=riscv64 HOWTUI_TEST_ELF_CLASS=2
check "musl-based Linux is not supported" HOWTUI_TEST_UNAME_S=Linux HOWTUI_TEST_MUSL=1 HOWTUI_TEST_UNAME_M=x86_64 HOWTUI_TEST_ELF_CLASS=2
check aarch64-apple-darwin HOWTUI_TEST_UNAME_S=Darwin HOWTUI_TEST_UNAME_M=arm64 HOWTUI_TEST_HW_ARM64=1
check aarch64-apple-darwin HOWTUI_TEST_UNAME_S=Darwin HOWTUI_TEST_UNAME_M=x86_64 HOWTUI_TEST_HW_ARM64=1
check x86_64-apple-darwin HOWTUI_TEST_UNAME_S=Darwin HOWTUI_TEST_UNAME_M=x86_64 HOWTUI_TEST_HW_ARM64=0
check "unsupported OS" HOWTUI_TEST_UNAME_S=FreeBSD HOWTUI_TEST_UNAME_M=amd64
exit $fail
