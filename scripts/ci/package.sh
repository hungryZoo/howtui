#!/usr/bin/env sh
# 릴리스 압축 파일 생성 (SRS 13.3절): 루트에 howtui, uninstall.sh, README.md, LICENSE.
# 사용: scripts/ci/package.sh <version> <target>...
set -eu
cd "$(dirname "$0")/../.."
version=$1
shift
mkdir -p dist
for target in "$@"; do
    stage=$(mktemp -d)
    cp "target/$target/release/howtui" README.md LICENSE "$stage/"
    cp scripts/uninstall.sh "$stage/uninstall.sh"
    chmod 0755 "$stage/uninstall.sh"
    tar -czf "dist/howtui-$version-$target.tar.gz" -C "$stage" howtui uninstall.sh README.md LICENSE
    rm -rf "$stage"
    echo "dist/howtui-$version-$target.tar.gz"
done
