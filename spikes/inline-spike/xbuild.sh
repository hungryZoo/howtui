#!/usr/bin/env bash
# SPIKE-02 교차 빌드 검증. cargo-zigbuild로 Linux GNU 3개 타깃을 glibc 2.17 기준으로 빌드한다.
# 사용: ZIG_CMD="<zig 실행 명령>" spikes/inline-spike/xbuild.sh
set -u
cd "$(dirname "$0")"
export CARGO_ZIGBUILD_ZIG_PATH="${ZIG_CMD:?ZIG_CMD 필요 (예: '/path/.venv/bin/python -m ziglang')}"
for t in armv7-unknown-linux-gnueabihf.2.17 aarch64-unknown-linux-gnu.2.17 x86_64-unknown-linux-gnu.2.17; do
  echo "=== $t"
  cargo zigbuild --release --target "$t" 2>&1 | tail -3
  echo "rc=${PIPESTATUS[0]}"
done
