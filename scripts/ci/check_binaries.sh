#!/usr/bin/env sh
# Linux 릴리스 바이너리 검사 (SRS-BLD-009, SRS-CMP-001): 아키텍처, GLIBC 심볼 최대 버전, 동적 라이브러리 의존.
set -eu
cd "$(dirname "$0")/../.."
FLOOR="${GLIBC_FLOOR:-2.17}"
fail=0
check() {
    target=$1 expect=$2
    bin="target/$target/release/howtui"
    [ -f "$bin" ] || { echo "missing $bin"; fail=1; return; }
    desc=$(file -b "$bin")
    case "$desc" in *"$expect"*) ;; *) echo "$target: unexpected format: $desc"; fail=1 ;; esac
    max=$(objdump -T "$bin" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -uV | tail -1)
    if [ "$(printf '%s\n%s\n' "$max" "$FLOOR" | sort -V | tail -1)" != "$FLOOR" ]; then
        echo "$target: glibc floor exceeded: $max > $FLOOR"; fail=1
    fi
    needed=$(objdump -p "$bin" | awk '/NEEDED/ {print $2}' | grep -vE '^(libc|libm|libpthread|libdl|librt|libgcc_s|ld-linux.*)\.so' || true)
    if [ -n "$needed" ]; then
        echo "$target: unexpected shared libraries: $needed"; fail=1
    fi
    echo "$target: ok (max GLIBC_$max, $(stat -c %s "$bin") bytes)"
}
check x86_64-unknown-linux-gnu "x86-64"
check aarch64-unknown-linux-gnu "ARM aarch64"
check armv7-unknown-linux-gnueabihf "ARM, EABI5"
exit $fail
