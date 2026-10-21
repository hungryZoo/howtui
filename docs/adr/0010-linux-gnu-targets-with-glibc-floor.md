# ADR-0010 Linux GNU 타깃과 glibc 2.17 하한

- 상태: 제안 (SPIKE-02 실측)
- 일자: 2026-10-22
- 관련: PRD-PLAT-002, PRD-PLAT-003, PRD-REL-004, SRS 13절, SRS-CMP-001

## 맥락

- 지시서 확정 타깃: `x86_64-unknown-linux-gnu`, `armv7-unknown-linux-gnueabihf`, `aarch64-unknown-linux-gnu` (musl 아님).
- Raspberry Pi OS glibc: Buster 2.28, Bullseye 2.31, Bookworm 2.36, Trixie 2.41. Ubuntu 20.04 2.31 (research/platform-build.md).
- zig 0.15 이상 기본 glibc는 2.31 (32비트 ARM 2.34). 접미사 없이 빌드한 tu v1.1.5 Linux 바이너리의 요구 glibc 2.30, 2.34 확인 (research/platform-build.md 2절).

## 결정

- ubuntu-24.04 러너에서 `cargo zigbuild --target <triple>.2.17`로 3개 Linux 타깃 빌드.
- cargo-zigbuild와 zig 버전 고정.
- CI에서 GLIBC 심볼 최대 버전 2.17 초과 시 실패 처리.
- C 의존성 링크 실패 시에만 2.28까지 상향 허용 (ADR 갱신 필수).

## 근거

- SPIKE-02: 3개 타깃 `GLIBC_2.17` 상한 확인, armv7 바이너리 `Tag_CPU_arch: v7`, qemu 10.2.3 + Debian 11 armhf에서 실행과 출력 바이트 일치.
- Rust 표준 라이브러리 하한(2.17)과 일치.

## 대안

- (a) musl 정적 빌드: 지시서 타깃 확정 사항과 상이. 필요 시 추가 자산으로 요구사항 변경 절차 후 검토.
- (b) cross-rs: 최신 릴리스 v0.2.5(2023) 이미지 glibc 2.23, 유지 상태 불확실로 기각.
