# 플랫폼별 빌드, CI 및 배포 제약 조사

- 조사일: 2026-10-22
- 대상 저장소: https://github.com/hungryZoo/howtui (public, 생성 2026-10-21, 기본 브랜치 `main`) [확인(실행) E0]
- 고정 릴리스 대상 (7종, 변경 불가)
  - `x86_64-unknown-linux-gnu`, `armv7-unknown-linux-gnueabihf`, `aarch64-unknown-linux-gnu`
  - `x86_64-apple-darwin`, `aarch64-apple-darwin`
  - `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`
- 지원 범위: Raspberry Pi 2-5 (TUI와 원격 LLM 한정). Raspberry Pi 1, Zero, Zero W (ARMv6) 제외.
- 배포 경로: GitHub Releases 단일 바이너리, `curl | sh` 설치 스크립트 (Linux, macOS), `irm | iex` 설치 스크립트 (Windows PowerShell 5.1, PowerShell 7)
- 참고 자료
  - 소유자 스킬 `cross-platform-cli-distribution` (SKILL.md, reference/install.sh) (S18)
  - 소유자 프로젝트 `hungryZoo/tu` 워크플로, 스크립트, v1.1.5 릴리스 자산 (S17)
  - 프로젝트 문서 `docs/research/spikes.md` SPIKE-02, `docs/SRS.md` SRS-CMP-001, SRS-CMP-005 (S64)
- 근거 표기
  - 확인(문서): 공식 문서 직접 확인. S 번호는 10절 출처.
  - 확인(소스): 공식 저장소 소스 파일 또는 공개 설치 스크립트 직접 확인.
  - 확인(실행): 이번 조사의 명령 실행 결과. E 번호는 10절 실행 기록.
  - 2차 자료: 공식 문서 외 자료 (보안 연구, 기술 기사, 포럼).
  - 미확인: 문서 기재 없음, 또는 실기기, 러너 실측 필요.
  - 판단: 근거 기반 권고. 사실 주장 아님.

## 0. 핵심 결론

| 항목 | 결론 | 근거 |
| --- | --- | --- |
| 러너 | 7개 대상 전부 public 저장소 무료 표준 러너로 빌드와 실행 검사 가능. macOS Intel 러너 지원 종료 공지 (2027년 가을) | S1, S3 |
| Linux gnu 빌드 | 단일 `ubuntu-24.04` 러너, cargo-zigbuild glibc 접미사 지정 교차 빌드 | S12, E5 |
| glibc 하한 | 2.17 유지 (SRS-CMP-001과 일치). C 의존 crate 링크 실패 시 2.28까지 상향 허용. 2.28 초과 금지 | E5, S24, S64 |
| 접미사 누락 위험 | 소유자 tu v1.1.5 실측: x86_64, aarch64 자산 GLIBC_2.30, armv7 자산 GLIBC_2.34 요구. 원인: zig 0.15 이후 기본 glibc 변경 (64비트 2.31, 32비트 arm 2.34) | E2, S13 |
| Pi 유저랜드 판별 | `uname -m` 단독 판별 불가. Pi 4, 400, CM4의 32비트 OS 기본 64비트 커널, Pi 5 64비트 커널 전용. ELF 클래스 판별 기반 armv7 선택 | S22, S27 |
| macOS 빌드 | macOS 러너 Apple 툴체인 빌드, `MACOSX_DEPLOYMENT_TARGET=11.0` 명시, ad-hoc 서명 검사 단계 | S34, S39, E3, E4 |
| macOS 배포 | curl 설치 경로 공증 불필요 (판단). 브라우저 다운로드 사용자 대상 quarantine 해제 안내 | S34, S36, S38 |
| Windows CRT | 두 msvc 대상 `+crt-static`, VC++ 재배포 패키지 의존 제거 | S11, S33, S40 |
| PS 5.1 설치 스크립트 | TLS 1.2 조건부 추가, `-UseBasicParsing` 필수 (2025-12-09 보안 업데이트 확인 프롬프트), 진행률 표시 끄기, 레지스트리 기반 아키텍처 판별, 레지스트리 기반 PATH 수정 | S41, S42, S44, S52, S55 |
| 자산 이름 | 태그 `vX.Y.Z`, `howtui-X.Y.Z-<target>.tar.gz` (Unix), `howtui-X.Y.Z-<target>.zip` (Windows), `SHA256SUMS`, `install.sh`, `install.ps1` | S17, S18 |
| 무결성 | 설치 스크립트 `SHA256SUMS` 필수 검증, immutable releases 활성화, `actions/attest@v4` 증명 생성, `gh release verify-asset`, `gh attestation verify` 선택 안내 | S56, S57, S58 |

## 1. GitHub Actions 호스티드 러너 현황

### 1.1 표준 러너 라벨 (public 저장소, 2026-10-22 기준)

| 용도 | 권장 라벨 | 그 외 라벨 | 사양 (문서 표기) | 상태 | 근거 |
| --- | --- | --- | --- | --- | --- |
| Linux x64 | `ubuntu-24.04` | `ubuntu-latest` (= 24.04), `ubuntu-22.04`, `ubuntu-26.04`, `ubuntu-slim` (1 CPU, 5 GB) | x64, 4 CPU, 16 GB, SSD 14 GB | 표준 러너 표 포함 | 확인(문서) S1, S2 |
| Linux arm64 | `ubuntu-24.04-arm` | `ubuntu-22.04-arm`, `ubuntu-26.04-arm` | arm64, 4 CPU, 16 GB | 표준 러너 표 포함 | 확인(문서) S1, S2 |
| macOS arm64 | `macos-15` | `macos-latest` (= macOS 26 arm64), `macos-26`, `macos-14` (deprecated), `xcode-27` (preview) | arm64 (M1), 3 CPU, 7 GB | macos-14: 2026-07-06 deprecation 시작, 2026-11-02 retire | 확인(문서) S1, S2, S4 |
| macOS x64 | `macos-15-intel` | `macos-26-intel` | Intel, 4 CPU, 14 GB | macos-13: 2025-12-04 retire. Intel 아키텍처 지원 종료 공지 (macOS 15 이미지 retire 시점, 2027년 가을) | 확인(문서) S1, S3 |
| Windows x64 | `windows-2025` | `windows-latest` (= Server 2025), `windows-2025-vs2026`, `windows-2022` | x64, 4 CPU, 16 GB | 표준 러너 표 포함 | 확인(문서) S1, S2 |
| Windows arm64 | `windows-11-arm` | `windows-11-vs2026-arm` | arm64, 4 CPU, 16 GB | 표준 러너 표 포함 | 확인(문서) S1, S2 |

- 과금: "Use of the standard GitHub-hosted runners is free and unlimited on public repositories." [확인(문서) S1]. howtui 저장소 public [확인(실행) E0]. 위 표 전체 라벨 무료 대상.
- `-latest` 라벨 이전: "The `-latest` migration process is gradual and happens over 1-2 months." 고정 버전 라벨 권장 [확인(문서) S2].
- 이미지 지원 정책: "We support (at maximum) 2 GA images and 1 beta image at a time." [확인(문서) S2]
- macos-13 retire 공지 원문: "The macOS 13 runner image will be retired by December 4th, 2025." 대체 라벨 `macos-15-intel`, `macos-14-large`, `macos-15-large` [확인(문서) S3].
- Intel 종료 공지 원문: "GitHub will no longer support this architecture on macOS after the macOS 15 runner image is retired in Fall 2027." [확인(문서) S3]
- `macos-26-intel` 라벨과 위 공지의 관계, `macos-26-intel` 종료 일정: 미확인.
- 권고 (판단): 신규 워크플로의 `-latest` 라벨, `macos-14` 사용 금지. x86_64 macOS 실행 검사 의존을 2027년 이후 대비 최소화.

### 1.2 이미지 구성 요소 (빌드 관련)

| 이미지 | 이미지 버전 | Rust | 그 외 | 근거 |
| --- | --- | --- | --- | --- |
| Ubuntu 24.04 Arm64 | 20261012.149.1 | Rust 1.99.0, Rustup 1.29.1 | Ubuntu 24.04.5, 커널 6.17.0-1022-azure, Docker 28.0.4 | 확인(문서) S5 |
| macOS 15 Arm64 | 20260907.0337.1 | Rust 1.98.1, Rustup 1.29.0 | macOS 15.7.9, 기본 Xcode 16.4, Xcode 16.0-26.3 설치, SDK macosx15.0-26.2 | 확인(문서) S6 |
| Windows Server 2025 (VS2026) | 20261007.264.1 | Rust 1.99.0, Rustup 1.29.1 | Visual Studio Enterprise 2026 18.10, `VC.Tools.ARM64`, `VC.Tools.ARM64EC` 구성 요소, PowerShell 7.6.6 | 확인(문서) S7 |
| Windows 11 Arm64 | 20261006.188.1 | Rust 1.99.0, Rustup 1.29.1 | OS 10.0.26200, Visual Studio Enterprise 2022 17.14, `VC.Tools.ARM64`, `VC.Tools.x86.x64`, PowerShell 7.6.6 | 확인(문서) S8 |

- GitHub Actions `shell` 키워드: `pwsh` = PowerShell Core (Windows 기본), `powershell` = PowerShell Desktop (Windows PowerShell) [확인(문서) S9]. 설치 스크립트 5.1, 7 동시 검사 수단.

### 1.3 러너 관련 미확인 사항

- macOS arm64 러너 Rosetta 2 설치 여부: README 기재 없음 [S6]. 미확인. x86_64 macOS 실행 검사는 `macos-15-intel` 사용 (판단).
- `ubuntu-24.04-arm` 러너의 AArch32 (armv7) 바이너리 직접 실행 가능 여부: README 기재 없음 [S5]. 미확인. armv7 실행 검사는 QEMU 사용 (판단).
- Windows 11 Arm64 이미지 Rust 기본 호스트 triple, Windows PowerShell 5.1 버전 표기: README 기재 없음 [S8]. 미확인.

## 2. 대상별 빌드 방식

### 2.1 Rust 대상 등급과 최소 요구

| 대상 | 등급 | 공식 비고 (원문) | 대상 기본 기능 (target-spec) | 근거 |
| --- | --- | --- | --- | --- |
| `x86_64-unknown-linux-gnu` | Tier 1 with Host Tools | 64-bit Linux (kernel 3.2+, glibc 2.17+) | `cpu: x86-64` | 확인(문서) S10, 확인(실행) E4 |
| `aarch64-unknown-linux-gnu` | Tier 1 with Host Tools | ARM64 Linux (kernel 4.1+, glibc 2.17+) | `+v8a,+outline-atomics` | S10, E4 |
| `armv7-unknown-linux-gnueabihf` | Tier 2 with Host Tools | Armv7-A Linux, hardfloat (kernel 3.2+, glibc 2.17) | `+v7,+vfp3d16,+thumb2,-neon` | S10, E4 |
| `x86_64-apple-darwin` | Tier 2 with Host Tools | 64-bit macOS (10.12+, Sierra+) | 기본 배포 대상 10.12 | S10, E4 |
| `aarch64-apple-darwin` | Tier 1 with Host Tools | ARM64 macOS (11.0+, Big Sur+) | `cpu: apple-m1`, 기본 배포 대상 11.0 | S10, E4 |
| `x86_64-pc-windows-msvc` | Tier 1 with Host Tools | 64-bit MSVC (Windows 10+, Windows Server 2016+) | `+cmpxchg16b,+sse3,+lahfsahf`, `crt-static-respected: true` | S10, E4 |
| `aarch64-pc-windows-msvc` | Tier 1 with Host Tools | ARM64 Windows MSVC | `+v8a,+neon`, `crt-static-respected: true` | S10, E4 |

- `aarch64-pc-windows-msvc` Tier 1 승격 시점: Rust 1.91 [2차 자료 S61, RFC 3817 S61]. 현재 Tier 1 표기는 확인(문서) S10.
- 실행 기록 E4: 로컬 rustc 1.96.0의 `--print target-spec-json` 설명 문자열과 S10 표기 일치.

### 2.2 glibc 하한 제어 방식 비교

| 방식 | glibc 하한 결정 요인 | 장점 | 한계 | 근거 |
| --- | --- | --- | --- | --- |
| (a) 네이티브 러너 직접 빌드 | 러너 OS glibc (ubuntu-24.04: 2.39, ubuntu-22.04: 2.35) | 설정 최소 | Buster (2.28), Bullseye (2.31), Ubuntu 20.04 (2.31) 미충족 가능 | glibc 버전 확인(실행) E9, 하한 상승 영향 판단 |
| (b) 구형 배포판 컨테이너 빌드 | 컨테이너 glibc | 실제 glibc 링크 | armv7 교차 툴체인 별도 구성, EOL 배포판 저장소 아카이브 상태 | 판단 |
| (c) cargo-zigbuild 접미사 (`<target>.2.17`) | 접미사 값 | 단일 러너 3개 대상, 실험 확인 | zig 버전별 기본값 변화 (접미사 누락 시), glibc `+crt-static` 미지원 | 확인(문서) S12, 확인(실행) E5 |
| (d) cross-rs Docker 이미지 | 이미지 glibc | QEMU 실행 검사 통합 | 최신 릴리스 v0.2.5 (2023-02-04). v0.2.5 이미지 glibc 2.23 (arm-gnueabihf 2.17), main 브랜치 이미지 2.39, centos 변형 2.17. MSVC, Darwin 이미지 미제공 | 확인(문서) S15, 확인(실행) E7 |

- cargo-zigbuild README 원문: "To build for a specific minimum glibc version, add that version as a suffix to the `--target` value." 예시 `cargo zigbuild --target aarch64-unknown-linux-gnu.2.17` [확인(문서) S12].
- cargo-zigbuild README 기본값 기재: "v12 to v14 releases default to glibc 2.28" [확인(문서) S12]. zig 0.15 이후 변경 내용 미반영 상태 (2.3절).
- cargo-zigbuild README 제약: "`-C target-feature=+crt-static` for statically linking to a glibc version is **not supported**" [확인(문서) S12].
- cross-rs README 원문: "These include MSVC and Apple Darwin targets, which we cannot ship pre-built images of." [확인(문서) S15]

### 2.3 zig 버전별 기본 glibc와 macOS 최소 버전

| zig 버전 | Linux gnu 기본 glibc | macOS 기본 최소 버전 | 근거 |
| --- | --- | --- | --- |
| 0.14.1 | 2.28 (전 아키텍처) | 미조회 | 확인(소스) S13 `lib/std/Target.zig` |
| 0.15.2 | 2.31 (64비트), 2.34 (`arm`, `x86` 등 32비트 time64 대상) | 13.0 | 확인(소스) S13 |
| 0.16.0 | 2.31, 2.34 (동일 구조) | 미조회 | 확인(소스) S13 |
| 0.17.0 | 2.31, 2.34 (동일 구조) | 15.0 (최대 27.0) | 확인(소스) S13 |

- 0.15.2 이후 소스 주석 원문: "For 32-bit targets that traditionally used 32-bit time, we require glibc 2.34 for full 64-bit time support. For everything else, we only require glibc 2.31." [확인(소스) S13]
- zig 0.17.0 `available_libcs`: `aarch64-linux-gnu` glibc_min 2.17, `arm-linux-gnueabihf`, `x86_64-linux-gnu` glibc_min 지정 없음 [확인(소스) S14]. 2.17 접미사 지정 가능 근거.
- PyPI 최신: `ziglang` 0.17.0 (2026-10-08 업로드), `cargo-zigbuild` 0.24.0 [확인(실행) E5].

### 2.4 소유자 tu v1.1.5 자산 실측

- tu 릴리스 빌드 방식: `pip install --quiet cargo-zigbuild ziglang` (버전 미고정), `cargo zigbuild --release --target "$t"` (glibc 접미사 없음) [확인(소스) S17 `release.yml`].
- 자산 `.comment` 섹션: `rustc version 1.99.1 (58cf8f9c3 2026-10-13)`, `clang version 22.1.8`, `Linker: LLD 22.1.8` [확인(실행) E2].

| 자산 | 요구 glibc 최댓값 | 2.28 초과 심볼 예 | Buster (2.28) | Bullseye (2.31) | 근거 |
| --- | --- | --- | --- | --- | --- |
| `tu-1.1.5-x86_64-unknown-linux-gnu` | GLIBC_2.30 | `gettid@GLIBC_2.30`, `posix_spawn_file_actions_addchdir_np@GLIBC_2.29` | 실행 불가 (판단) | 실행 가능 (판단) | 확인(실행) E2 |
| `tu-1.1.5-aarch64-unknown-linux-gnu` | GLIBC_2.30 | 동일 | 실행 불가 (판단) | 실행 가능 (판단) | E2 |
| `tu-1.1.5-armv7-unknown-linux-gnueabihf` | GLIBC_2.34 | `__libc_start_main@GLIBC_2.34`, `__clock_gettime64@GLIBC_2.34`, `pthread_create@GLIBC_2.34`, `stat64@GLIBC_2.33`, `pthread_getattr_np@GLIBC_2.32` | 실행 불가 (판단) | 실행 불가 (판단) | E2 |

- `.gnu.version_r` 버전 요구 항목 `Flags: none` (약한 버전 아님) [확인(실행) E2]. `gettid` 등 심볼 자체는 약한 참조 (`w`)이나 버전 노드 요구는 필수 상태.
- 실행 불가 판정 근거: 요구 버전 노드 부재 glibc의 동적 로더 거부 동작 (ELF 심볼 버전 규칙 기반 판단, 실기 미검증).
- 해석 (판단): zig 0.17 계열 기본값 (64비트 2.31, 32비트 arm 2.34) 적용 결과와 일치. 소유자 스킬의 Linux musl 우선 방침 [확인(소스) S18]과 tu의 musl 자산 동시 배포 [확인(실행) E1] 덕분에 tu 설치 스크립트 기본 경로 영향 제한. howtui는 gnu 고정 대상이므로 접미사 누락 영향 직접 노출.

### 2.5 cargo-zigbuild 접미사 실험

- 조건: 로컬 Ubuntu 20.04 x86_64, rustc 1.96.0, `uv tool run --from 'cargo-zigbuild<0.24' --with 'ziglang==0.17.0'` (cargo-zigbuild 0.23.4, zig 0.17.0). 시험 crate: 스레드 생성 (이름 지정), 파일 메타데이터, 프로세스 생성 (`current_dir`), 시간 API.
- cargo-zigbuild 0.24.0: PyPI linux x86_64 wheel 부재 메시지로 설치 실패 (2026-10-22 확인 시점) [확인(실행) E5].

| `--target` 값 | 요구 glibc 최댓값 | 근거 |
| --- | --- | --- |
| `armv7-unknown-linux-gnueabihf` (접미사 없음) | GLIBC_2.34 | 확인(실행) E5 |
| `armv7-unknown-linux-gnueabihf.2.28` | GLIBC_2.28 | E5 |
| `armv7-unknown-linux-gnueabihf.2.17` | GLIBC_2.17 | E5 |
| `x86_64-unknown-linux-gnu` (접미사 없음) | GLIBC_2.30 | E5 |
| `x86_64-unknown-linux-gnu.2.28` | GLIBC_2.28 | E5 |
| `x86_64-unknown-linux-gnu.2.17` | GLIBC_2.17 | E5 |
| `aarch64-unknown-linux-gnu` (접미사 없음) | GLIBC_2.30 | E5 |
| `aarch64-unknown-linux-gnu.2.28` | GLIBC_2.28 | E5 |

- 기존 SPIKE-02 결과 (cargo-zigbuild 0.22.3, zig 0.17.0, `.2.17`, 3개 대상 GLIBC_2.17, qemu 실행 통과)와 일치 [S64].
- 출력 경로: 접미사 대상도 `target/<접미사 제외 triple>/release/` 사용. 같은 `CARGO_TARGET_DIR`에서 접미사 변경 빌드 시 동일 경로 산출물 관찰 [확인(실행) E5].
- armv7 산출물 `Tag_Advanced_SIMD_arch: NEONv1` 표기 관찰 (tu 자산, 실험 산출물 공통) [확인(실행) E2, E5]. Rust 대상 기능 `-neon` [E4]과 다른 표기. 표기 유래 미확인. NEON 미지원 ARMv7 CPU 실행 가능 여부 미확인. Pi 2-5 실기 실행 검사 항목.

### 2.6 glibc 하한 권고

- 요구 범위 최저 glibc: Raspberry Pi OS Buster 2.28 [확인(실행) E10]. Ubuntu 20.04 2.31 [확인(실행) E9].
- 권고 (판단)
  - (1) 기본 접미사 `.2.17`. SRS-CMP-001, SPIKE-02와 일치, Rust 문서 하한과 동일 [S10, S64].
  - (2) C 코드 포함 의존성 (TLS 암호 백엔드 등)의 2.17 미존재 심볼 링크 실패 시 `.2.28`까지 상향 허용. 2.28 초과 금지 (Buster 실행 불가).
  - (3) CI 관문: 빌드 직후 `readelf -W --version-info --dyn-syms` 기반 최댓값 추출 (cargo-zigbuild README 스크립트 S12), 설정 하한 초과 시 실패.
  - (4) `cargo-zigbuild`, `ziglang` 버전 고정. 갱신 시 (3) 관문 결과 재확인.
  - (5) 실행 검사: 구형 사용자 공간 컨테이너 (Debian 10 또는 11 계열) 내 `--version` 실행. armv7 실행 검사는 최신 QEMU 사용. SPIKE-02 기록 기준 qemu-user 4.2.1 termios ioctl 실패, 10.2.3 통과 [S64].
- 커널 하한: armv7 3.2, aarch64 4.1 [S10]. 현행 Raspberry Pi OS 커널 6.12 (Bookworm), 6.18 (Trixie) [확인(문서) S21].

### 2.7 대상별 권장 빌드 방식

| 대상 | 빌드 러너 | 빌드 방식 | 실행 검사 | 근거 |
| --- | --- | --- | --- | --- |
| `x86_64-unknown-linux-gnu` | `ubuntu-24.04` | `cargo zigbuild --release --locked --target x86_64-unknown-linux-gnu.2.17` | 러너 직접 실행, Debian 10 또는 11 컨테이너 실행 | E5, 판단 |
| `aarch64-unknown-linux-gnu` | `ubuntu-24.04` | `cargo zigbuild ... --target aarch64-unknown-linux-gnu.2.17` | `ubuntu-24.04-arm` 직접 실행과 arm64 구형 컨테이너 실행 | E5, S1, 판단 |
| `armv7-unknown-linux-gnueabihf` | `ubuntu-24.04` | `cargo zigbuild ... --target armv7-unknown-linux-gnueabihf.2.17` | QEMU (binfmt) armv7 컨테이너 실행. uv 사례: `arm32v7/ubuntu:20.04` 이미지 실행 검사 | E5, S33 |
| `aarch64-apple-darwin` | `macos-15` | Apple 툴체인 네이티브 `cargo build --release --target aarch64-apple-darwin` | 러너 직접 실행, `codesign -v` | S6, S34 |
| `x86_64-apple-darwin` | `macos-15` | 같은 러너 교차 빌드 (`rustup target add x86_64-apple-darwin`) | `macos-15-intel` 실행 (2027년 가을 종료 전까지) | S1, S3, 판단 |
| `x86_64-pc-windows-msvc` | `windows-2025` | 네이티브 MSVC, `+crt-static` | 러너 직접 실행 | S7, S11 |
| `aarch64-pc-windows-msvc` | `windows-11-arm` | 네이티브 MSVC, `+crt-static` | 러너 직접 실행 | S8 |

- macOS를 zigbuild 교차 빌드로 만들지 않는 근거: tu v1.1.5 macOS 자산 `minos 15.0`, `sdk 27.0` 표기 (zig 0.17.0 기본 macOS 최소 15.0과 일치) [확인(실행) E3, 확인(소스) S13]. macOS 11-14 실행 불가 가능성 (실기 미확인). x86_64 자산 서명 없음 [E3]. Apple 툴체인 사용 시 rustc 배포 대상 직접 적용 [E4].
- `aarch64-pc-windows-msvc` 대안: `windows-2025` x64 러너 교차 빌드. 근거: VS ARM64 빌드 도구 구성 요소 설치 [S7], uv 사례 (x64 Windows 러너에서 `aarch64-pc-windows-msvc` 빌드 후 Windows 11 aarch64 러너에서 검사) [확인(소스) S33]. howtui 교차 빌드 성공 여부는 미확인 (미실행).
- `aarch64-unknown-linux-gnu` 대안: `ubuntu-24.04-arm` 러너 zigbuild 빌드. 동일 접미사 적용 가능 (판단).

### 2.8 TLS 구현 선택 제약

| 배포판 | OpenSSL 버전 | 근거 |
| --- | --- | --- |
| Debian 11 bullseye | 1.1.1w | 확인(실행) E9 |
| Debian 12 bookworm | 3.0.20 | E9 |
| Debian 13 trixie | 3.5.7 | E9 |
| Ubuntu 20.04 focal | 1.1.1f | E9 |
| Ubuntu 22.04 jammy | 3.0.2 | E9 |
| Ubuntu 24.04 noble | 3.0.13 | E9 |
| Debian 10 buster (Pi OS Buster) | 미조회 | 미확인 |

- 판단: OpenSSL 동적 링크 시 1.1 계열과 3 계열 공유 라이브러리 분리. 단일 gnu 바이너리의 전 배포판 공용 불가. rustls 계열 (OpenSSL 비의존) 사용 권고.
- 미확인: rustls 암호 백엔드 (ring, aws-lc-rs) C 코드의 zigbuild 교차 빌드 가능 여부, glibc 2.17 링크 여부. 2.6절 (3) 관문으로 검출.

### 2.9 musl 정적 링크 대안 (참고 기록)

- 전제: 릴리스 대상 gnu 고정. 대안 비교 목적 기록.
- 등급: `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` Tier 2 with Host Tools (musl 1.2.5), `armv7-unknown-linux-musleabihf` Tier 2 without Host Tools [확인(문서) S10].
- 기본 정적 C 런타임 대상 목록: `x86_64-unknown-linux-musl`, `armv7-unknown-linux-musleabihf` 등 포함 [확인(문서) S11]. `aarch64-unknown-linux-musl` 목록 미포함 (명시 설정 필요 여부 미확인).
- 장점: glibc 하한 무관. 64비트 커널과 armhf 유저랜드 조합의 aarch64 정적 바이너리 실행 가능 (스킬 기재 S18, 실기 미확인).
- 사례: tu (musl 자산 동시 배포, 설치 스크립트 musl 우선) [S17, S18], starship (Linux musl 고정, 원문 "use the statically compiled musl bins on linux to avoid linking issues") [확인(소스) S28].
- howtui 설치 스크립트 영향: 소유자 스킬의 "aarch64 static build runs fine there" 규칙 적용 불가. gnu 동적 바이너리는 armhf 유저랜드에 `ld-linux-aarch64.so.1` 부재 시 실행 불가 (판단). 3.6절 판별 절차 필요.

## 3. Raspberry Pi 사실 정리

### 3.1 모델 요약표 (cheat sheet)

| 모델 | SoC | CPU | 명령어 집합 | 64비트 OS 공식 호환 | 32비트 OS 커널 기본 | howtui 자산 (32비트 OS / 64비트 OS) | 근거 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Pi 1 (A, B, A+, B+), Zero, Zero W, CM1 | BCM2835 | ARM1176JZF-S 단일 코어 | ARMv6 | 미포함 | `kernel.img` | 지원 제외 | S19, S21, S22 |
| Pi 2 B (초기, v1.1) | BCM2836 | Cortex-A7 4코어 | Armv7 | 미포함 | `kernel7.img` | armv7 / 해당 없음 | S19, S22 |
| Pi 2 B v1.2 | BCM2837 | Cortex-A53 4코어 | Armv8 | 미포함 (공식 목록 기준) | `kernel7.img` | armv7 / 해당 없음 | S19, S20, S21 |
| Pi 3 B, CM3 | BCM2837 | Cortex-A53 4코어 | Armv8 | 포함 | `kernel7.img` | armv7 / aarch64 | S19, S21 |
| Pi 3 B+, Pi 3 A+, CM3+ | BCM2837B0 | Cortex-A53 4코어 (BCM2837과 동일 코어) | Armv8 | 포함 | `kernel7.img` | armv7 / aarch64 | S19, S21 |
| Zero 2 W | RP3A0 (BCM2710A1 다이) | Cortex-A53 4코어 | 64비트 (문서 표기) | 포함 | `kernel7.img` | armv7 / aarch64 | S19, S21, S22 |
| Pi 4 B, 400, CM4, CM4S | BCM2711 | Cortex-A72 4코어 | Armv8-A | 포함 | `kernel8.img` (64비트, `arm_64bit` 기본 1) | armv7 / aarch64 | S19, S21, S22 |
| Pi 5, 500, 500+, CM5 | BCM2712 | Cortex-A76 4코어 2.4 GHz | Armv8-A (문서 표기) | 포함 | `kernel_2712.img` (64비트 커널 전용) | armv7 / aarch64 | S19, S21, S22 |

- Pi 2 B v1.2의 BCM2837 근거 원문: "BCM2837-based Raspberry Pi devices – that is, Raspberry Pi 2B version 1.2, Raspberry Pi 3B, and Raspberry Pi 3B+" [확인(문서) S20].
- 64비트 OS 호환 목록 원문: "3B, 3B+, 3A+, 4B, 400, 5, 500, 500+, CM3, CM3+, CM4, CM4S, CM5, Zero 2 W" [확인(문서) S21]. Pi 2 B v1.2 미포함. 64비트 OS 실행 가능 여부 미확인 (비공식).
- 32비트 OS (Trixie) 호환 표기: "All Raspberry Pi models". 32비트 Legacy (Bookworm) 호환 목록: "Zero, Zero W, 1A+, 1B+, 2B, 3B, 3B+, 3A+, 4B, 400, CM1, CM3, CM3+, CM4, CM4S, Zero 2 W" (Pi 5 미포함) [확인(문서) S21].
- 32비트 OS의 `uname -m` 값 (판단, 실기 미확인): `kernel.img` 계열 `armv6l`, `kernel7.img` 계열 `armv7l`, 64비트 커널 `aarch64`.

### 3.2 Raspberry Pi OS 릴리스와 glibc

| 릴리스 | Debian 기반 | 32비트 이미지 첫 릴리스 노트 날짜 | glibc (archive.raspberrypi.com `libc6`) | 2026-10 다운로드 페이지 상태 | 근거 |
| --- | --- | --- | --- | --- | --- |
| Buster | Debian 10 | 2019-06-20 ("Based on Debian Buster") | 2.28 (`2.28-10+rpt2+rpi1+deb10u2` 등) | 미게시 | 확인(실행) E10, 확인(문서) S23 |
| Bullseye | Debian 11 | 2021-10-30 ("Based on Debian version 11 (bullseye)") | 2.31 (`2.31-13+rpt2+rpi1+deb11u13` 등) | 미게시 | E10, S23 |
| Bookworm | Debian 12 | 2023-10-10 ("Based on Debian bookworm release") | 2.36 (`2.36-9+rpt2+deb12u14` 등) | Legacy 이미지 (2026-10-06, 커널 6.12) | E10, S21, S23 |
| Trixie | Debian 13 | 2025-10-01 ("Based on Debian Trixie release") | 2.41 (`2.41-12+rpt1+deb13u4` 등) | 현행 이미지 (2026-10-06, 커널 6.18) | E10, S21, S23 |

- 32비트 (armhf), 64비트 (arm64) 이미지: Bookworm, Trixie 모두 양쪽 제공 [확인(문서) S21]. archive.raspberrypi.com glibc 패키지 armhf, arm64 동시 존재 [확인(실행) E10].
- 64비트 이미지 디렉터리 첫 항목: `raspios_arm64-2020-05-28` [확인(실행) E10]. 정식 출시 공지 시점 미확인.
- Raspbian 저장소 (armhf) glibc: 2.31, 2.36, 2.41, 2.43 패키지 확인 [확인(실행) E10].

### 3.3 비교 배포판 glibc

| 배포판 | glibc | 근거 |
| --- | --- | --- |
| Ubuntu 20.04 focal | 2.31 (`2.31-0ubuntu9.18`) | 확인(실행) E9 |
| Ubuntu 22.04 jammy | 2.35 (`2.35-0ubuntu3.15`) | E9 |
| Ubuntu 24.04 noble | 2.39 (`2.39-0ubuntu8.9`) | E9 |
| Ubuntu 26.04 resolute | 2.43 (`2.43-2ubuntu2.4`) | E9 |
| Debian 11 bullseye | 2.31 (`2.31-13+deb11u14`) | 확인(문서) S25 |
| Debian 12 bookworm | 2.36 (`2.36-9+deb12u14`) | S25 |
| Debian 13 trixie | 2.41 (`2.41-12+deb13u4`) | S25 |
| Debian forky (testing) | 2.43 (`2.43-7`) | S25 |

### 3.4 64비트 커널과 32비트 유저랜드 조합

- `arm_64bit` 원문: "If set to 1, the kernel will be started in 64-bit mode." 기본값: "Defaults to 1 on Raspberry Pi 4, 400, and Compute Module 4 and 4S platforms." 그 외 모델 기본 0 [확인(문서) S22].
- Pi 5 계열: 64비트 커널 전용, `arm_64bit` 무시, 기본 `kernel_2712.img`, 부재 시 `kernel8.img` [확인(문서) S22].
- 결과 (판단): Pi 4, 400, CM4, Pi 5 계열의 32비트 Raspberry Pi OS 기본 구성에서 `uname -m` = `aarch64`, 유저랜드 armhf. aarch64 gnu 바이너리 선택 시 `/lib/ld-linux-aarch64.so.1` 부재로 실행 실패.
- 동적 로더 경로 (tu 자산 `file` 출력): x86_64 `/lib64/ld-linux-x86-64.so.2`, aarch64 `/lib/ld-linux-aarch64.so.1`, armv7 `/lib/ld-linux-armhf.so.3` [확인(실행) E2].

### 3.5 유저랜드 아키텍처 판별 방법 비교

| 방법 | 명령 | 판별 대상 | 의존성 | 사용 사례 | 한계 |
| --- | --- | --- | --- | --- | --- |
| (a) 자기 프로세스 ELF 클래스 | `head -c 5 /proc/self/exe` 결과와 `printf '\177ELF\001'` (32비트), `printf '\177ELF\002'` (64비트) 비교 | 실행 중인 `head` 바이너리의 비트 수 | `head`, `/proc` | rustup-init.sh `get_bitness` [확인(소스) S27] | `/proc` 부재 환경. rustup 대체 경로: `$SHELL`, `/bin/sh` |
| (b) 시스템 셸 ELF 클래스 | `head -c 5 /bin/sh` | `/bin/sh` 실체 (심볼릭 링크 추적) 비트 수 | `head` | rustup 대체 경로 [S27] | 셸 교체 환경 대표성 미확인 |
| (c) `getconf LONG_BIT` | `getconf LONG_BIT` | `getconf` 바이너리 기준 `long` 비트 수 | `getconf` | starship install.sh (`aarch64` + 32 → `arm`) [확인(소스) S28] | 최소 이미지의 `getconf` 부재 여부 미확인 |
| (d) `dpkg --print-architecture` | `dpkg --print-architecture` | dpkg 기본 아키텍처 (`armhf`, `arm64`) | dpkg | 없음 (이번 조사 범위) | Debian 계열 한정 |
| (e) 동적 로더 존재 | `test -e /lib/ld-linux-armhf.so.3` 등 | gnu 바이너리 실행 전제 직접 확인 | 없음 | 없음 (판단) | multiarch 구성에서 양쪽 존재 가능 |

- rustup 추가 규칙 [확인(소스) S27]
  - `armv7l`, `armv8l` → `armv7-unknown-linux-gnueabihf`
  - `aarch64` + 32비트 유저랜드 → `armv7-unknown-linux-gnueabihf`
  - armv7 판정 후 `/proc/cpuinfo` `Features`에 `neon` 또는 `simd` 부재 시 `arm` (ARMv6) 대상으로 하향
  - musl 판별: `ldd --version 2>&1 | grep -q 'musl'`
  - macOS Rosetta 판별: `uname -m` = `x86_64`이고 `sysctl hw.optional.arm64` = 1이면 arm64
- uv install.sh: 같은 `sysctl hw.optional.arm64` 판별 [확인(소스) S29].
- 소유자 스킬 기재 "On macOS use the native arch, no Rosetta guessing" [S18]와 rustup, uv 방식 차이. 권고 (판단): Rosetta 셸 사용자 대상 arm64 선택.
- 로컬 실행 예: `getconf GNU_LIBC_VERSION` 출력 `glibc 2.31`, `getconf LONG_BIT` 출력 `64` (Ubuntu 20.04 x86_64) [확인(실행) E11].

### 3.6 설치 스크립트 판별 절차 초안 (POSIX sh, 미실행)

```sh
# ELF 클래스 판별 (rustup-init.sh 방식): 32 또는 64 출력
elf_bits() {
    h=$(head -c 5 "$1" 2>/dev/null) || return 1
    if [ "$h" = "$(printf '\177ELF\001')" ]; then echo 32
    elif [ "$h" = "$(printf '\177ELF\002')" ]; then echo 64
    else return 1
    fi
}

detect_target() {
    os=$(uname -s)
    arch=$(uname -m)
    case "$os" in
    Linux)
        if ldd --version 2>&1 | grep -qi musl; then
            err "musl libc unsupported (glibc builds only)"
        fi
        bits=$(elf_bits /proc/self/exe || elf_bits /bin/sh) \
            || err "cannot detect userland bitness"
        case "$arch" in
        x86_64|amd64)
            [ "$bits" = 64 ] || err "32-bit x86 userland unsupported"
            echo x86_64-unknown-linux-gnu ;;
        aarch64|arm64)
            if [ "$bits" = 64 ]; then echo aarch64-unknown-linux-gnu
            else echo armv7-unknown-linux-gnueabihf   # 64-bit kernel + armhf userland
            fi ;;
        armv7l|armv8l)
            echo armv7-unknown-linux-gnueabihf ;;
        armv6l)
            err "ARMv6 (Raspberry Pi 1, Zero, Zero W) unsupported" ;;
        *)
            err "unsupported architecture: $arch" ;;
        esac ;;
    Darwin)
        case "$arch" in
        arm64) echo aarch64-apple-darwin ;;
        x86_64)
            if (sysctl hw.optional.arm64 2>/dev/null || true) | grep -q ': 1'; then
                echo aarch64-apple-darwin   # Rosetta shell on Apple silicon
            else
                echo x86_64-apple-darwin
            fi ;;
        *) err "unsupported macOS architecture: $arch" ;;
        esac ;;
    *) err "unsupported OS: $os" ;;
    esac
}
```

- 후속 점검 단계 (판단)
  - (1) 대상별 동적 로더 존재 확인 (3.4절 경로). 부재 시 경고와 대상 재확인 안내.
  - (2) `getconf GNU_LIBC_VERSION` 결과와 릴리스 glibc 하한 비교. 미달 시 중단.
  - (3) 재정의 환경변수 `HOWTUI_TARGET` 제공 (판별 실패 대비).
  - (4) armv7 대상의 NEON 미지원 CPU 처리: 2.5절 미확인 사항 해소 전 경고 수준.

### 3.7 Pi 5 페이지 크기 참고

- Pi 5 기본 커널 `kernel_2712.img` [확인(문서) S22]. 16K 페이지 크기 관련 공식 문서 기재: 미확인.
- 2차 자료: 16K 페이지 커널과 4K 페이지 가정 메모리 할당 라이브러리 충돌 사례, `kernel=kernel8.img` 우회, `getconf PAGESIZE` 확인 [2차 자료 S62].
- 권고 (판단): jemalloc 등 페이지 크기 고정 할당자 도입 시 Pi 5 실기 검사 필수. 기본 시스템 할당자 유지.

## 4. macOS 서명, Gatekeeper, 최소 버전

### 4.1 arm64 서명 요구

- Apple 원문: "New in macOS 11 on Macs with Apple silicon, ... the operating system enforces that any executable must be signed before it's allowed to run. There isn't a specific identity requirement for this signature: a simple ad-hoc signature is sufficient." [확인(문서) S34]
- 적용 제외 원문: "This new policy doesn't apply to translated x86 binaries running under Rosetta 2, nor does it apply to macOS 11 running on Intel-based platforms." [S34]
- 링크 시점 자동 서명: "the toolchain will now automatically sign your executables whenever you build from Xcode, or use command-line utilities such as clang(1) or ld(1)." 링크 후 수정 도구 (`strip`, `install_name_tool`) 사용 시 수동 `codesign` 필요 가능 [S34].
- Gatekeeper 원문: "binaries signed this way cannot pass through Gatekeeper." [S34]
- tu v1.1.5 실측 [확인(실행) E3]
  - `aarch64-apple-darwin`: `LC_CODE_SIGNATURE` 존재, CodeDirectory flags `0x20002` (ad-hoc 0x2, linker-signed 0x20000 비트 해석은 xnu 헤더 값 기준, 이번 조사 헤더 미조회)
  - `x86_64-apple-darwin`: 코드 서명 없음 (Intel 정책상 허용 범위)

### 4.2 quarantine과 Gatekeeper

- quarantine 부여 주체: 앱 단위 설정. Apple 문서 `LSFileQuarantineEnabled` 원문 "A Boolean value indicating whether the files this app creates are quarantined by default." [확인(문서) S36]
- Apple DTS 원문: "Download your product in a way that quarantines it (for example, using Safari)." [확인(문서) S37]
- curl 다운로드 quarantine 미부여: 보안 연구 기재 "curl does not set the quarantine attribute" [2차 자료 S38]. Apple 공식 문서 명시 미확인.
- 결론 (판단)
  - `curl | sh` 설치 경로: quarantine 미부여, ad-hoc 서명 arm64 바이너리 실행 가능.
  - 브라우저 다운로드 경로: quarantine 부여, ad-hoc 서명 Gatekeeper 통과 불가 [S34]. 소유자 스킬 방침대로 플랫폼 절에 `xattr -d com.apple.quarantine <경로>` 한 줄 안내 [S18].

### 4.3 공증 (notarization) 필요성

- Apple 원문: "Beginning in macOS 10.15, all software built after June 1, 2019, and distributed with Developer ID must be notarized." [확인(문서) S35]
- 단독 바이너리 원문: "Although tickets are created for standalone binaries, it's not currently possible to staple tickets to them." ZIP 공증 가능, ZIP 직접 staple 불가 [확인(문서) S35].
- 판단: howtui 1차 배포 (curl 설치, ad-hoc 서명)는 Developer ID 배포 아님. 공증 대상 아님. 브라우저 다운로드 또는 pkg 배포 도입 시 Developer ID 서명과 공증 검토. Apple Developer Program 가입 비용과 조건: 미조회.

### 4.4 최소 macOS 버전

| 항목 | 값 | 근거 |
| --- | --- | --- |
| rustc 기본 배포 대상 x86_64 | `MACOSX_DEPLOYMENT_TARGET=10.12` | 확인(실행) E4 (rustc 1.96.0 `--print deployment-target`) |
| rustc 기본 배포 대상 aarch64 | `MACOSX_DEPLOYMENT_TARGET=11.0` | E4 |
| 환경변수 반영 | x86_64: `MACOSX_DEPLOYMENT_TARGET=10.15` 지정 시 10.15. aarch64: 10.15 지정 시 11.0 (하한 고정) | E4 |
| Xcode 16.4 지원 배포 대상 | macOS 10.13-15 (SDK 15.5) | 확인(문서) S39 |
| Xcode 26.x 지원 배포 대상 | macOS 11-26.x | S39 |
| Xcode 27 지원 배포 대상 | macOS 12-27 | S39 |
| macos-15 arm64 러너 기본 Xcode | 16.4 | 확인(문서) S6 |

- 불일치 (판단): Rust x86_64 기본값 10.12가 Xcode 16.4 하한 10.13, Xcode 26 하한 11 미만. 하한 미만 지정 시 링커 동작 미확인.
- SRS-CMP-005 ("Rust 타깃 기본값 기준") 영향: 8절 반영 후보.

### 4.5 macOS 권고 (판단)

- (1) 빌드: `macos-15` 러너 Apple 툴체인, 두 대상 `MACOSX_DEPLOYMENT_TARGET=11.0` 명시. Xcode 16.4, 26.x 공통 지원 범위.
- (2) 검사 단계: `codesign -v --verbose` (aarch64), `vtool -show-build` 또는 `otool -l`의 `minos` 확인, `file` 아키텍처 확인. Cargo `strip` 설정 사용 시 서명 유효성 재확인.
- (3) 공증: 1차 배포 제외. 브라우저 다운로드 사용자용 quarantine 해제 안내만 제공.
- (4) universal2 바이너리: 고정 대상 목록 외 항목. 미채택.

## 5. Windows 빌드, 실행 환경, 설치 스크립트

### 5.1 MSVC 빌드 러너와 aarch64 교차 빌드

- `windows-2025`: Visual Studio Enterprise 2026 18.10, `Microsoft.VisualStudio.Component.VC.Tools.ARM64` 구성 요소 [확인(문서) S7].
- `windows-11-arm`: Visual Studio Enterprise 2022 17.14, ARM64, x64 MSVC 도구 [확인(문서) S8].
- 교차 빌드 사례: uv `build-release-binaries.yml`, x64 Windows 러너 (`namespace-profile-windows-2022-x86-64-16x32`)에서 `aarch64-pc-windows-msvc` 빌드, `github-windows-11-aarch64-8` 러너에서 검사 [확인(소스) S33].
- 권고 (판단): 대상별 네이티브 러너 빌드 (실행 검사 동시 수행). 교차 빌드는 `windows-11-arm` 대기열 지연 시 대안.

### 5.2 CRT 링크 방식 권고

- Rust 기본: MSVC 대상 동적 C 런타임 (정적 기본 대상 목록 미포함) [확인(문서) S11]. `-C target-feature=+crt-static` 지정 시 `target_feature="crt-static"` cfg 활성 [확인(실행) E4].
- UCRT 원문: "The Universal CRT (UCRT) is a Microsoft Windows operating system component. It's included as part of the operating system in Windows 10 or later" [확인(문서) S40]. VC++ 런타임 (`vcruntime140.dll`) 동적 링크 의존 여부: dumpbin 실측 미실행, 미확인.
- 사례
  - uv `.cargo/config.toml`: `[target.'cfg(all(target_env = "msvc", target_os = "windows"))']` `rustflags = ["-C", "target-feature=+crt-static"]` [확인(소스) S33]
  - ripgrep `.cargo/config.toml`: "statically link the C runtime so that the resulting EXE does not depend on the vcruntime DLL" (x86_64, i686 대상) [확인(소스) S33]
  - bun install.ps1: `STATUS_DLL_NOT_FOUND` 종료 코드 감지 시 VC++ 재배포 패키지 안내 [확인(소스) S30]
- 권고 (판단): uv 형식 `cfg` 조건으로 두 msvc 대상 일괄 `+crt-static`. 릴리스 검사에 `dumpbin /dependents howtui.exe` 결과의 `VCRUNTIME` 부재 확인 추가.

```toml
# .cargo/config.toml
[target.'cfg(all(target_env = "msvc", target_os = "windows"))']
rustflags = ["-C", "target-feature=+crt-static"]
```

### 5.3 SmartScreen, Mark-of-the-Web, Smart App Control

- MOTW 원문: "Other methods of downloading files may not mark the files as coming from the Internet Zone. Some examples include: `curl.exe`, `Invoke-RestMethod`, `Invoke-WebRequest`" [확인(문서) S49].
- SmartScreen 평판 검사와 Zone.Identifier 관계: Windows 11 셸 실행 시 평판 검사 후 Zone.Identifier 제거 기재 [2차 자료 S63]. MOTW 없는 파일의 SmartScreen 경고 비발생 여부: 미확인.
- Smart App Control 원문: "Malware, Potentially Unwanted Apps (PUA), and unknown, unsigned code are blocked by default." 적용 모드: evaluation, enforcement. 신뢰 루트 프로그램 CA 서명 시 허용 [확인(문서) S50]. MOTW 한정 적용 여부 문서 기재 없음, 미확인.
- 판단
  - `irm | iex` 설치 경로: MOTW 미부여 가능성. SmartScreen 경고 가능성 낮음 (미확인).
  - Smart App Control enforcement 환경: 무서명 `howtui.exe` 차단 가능성. 코드 서명 (신뢰 CA) 도입은 후속 검토 항목.
  - 설치 스크립트의 `Unblock-File` 호출: 스트림 부재 시 영향 없음. 브라우저 경유 zip 수동 설치 대비 안내 문구 포함.

### 5.4 PowerShell 5.1 설치 스크립트 점검표

| 번호 | 항목 | 위험 | 대응 | 근거 |
| --- | --- | --- | --- | --- |
| (1) | TLS 1.2 | GitHub TLS 1.0, 1.1 제거 (2018-02-22). .NET Framework 4.7 미만 대상 프로세스의 SystemDefault 비활성 가능 | `SecurityProtocol`이 SystemDefault(0)이 아니고 Tls12(3072) 비트 부재 시에만 OR 추가. SystemDefault 상태 강제 변경 금지 (TLS 1.3 협상 유지) | 확인(문서) S44, S45. powershell.exe 실제 기본값 미확인 |
| (2) | `Invoke-WebRequest` 파싱 | 5.1 기본 DOM 파싱, IE 미설치 환경 실패. 2025-12-09 CVE-2025-54100 업데이트 후 확인 프롬프트, 비대화형 실행 정지 가능 | 모든 호출에 `-UseBasicParsing`, 또는 `Net.WebClient`, `curl.exe` 사용 | 확인(문서) S41, S42 |
| (3) | 진행률 표시 | 5.1 진행률 표시로 다운로드 50배 이상 지연 보고. PowerShell 6 이후 수정, Windows PowerShell 미수정 언급 | `$ProgressPreference = 'SilentlyContinue'` 임시 설정 후 복원 | 확인(소스) S55 (issue #2138) |
| (4) | `Get-FileHash` | 5.1 포함, 기본 SHA256, 대문자 hex 출력 | `SHA256SUMS` 소문자 hex와 대소문자 무시 비교 | 확인(문서) S43. 도입 버전 (4.0) 문서 기재 미확인 |
| (5) | `Expand-Archive` | 5.1 포함 (`Microsoft.PowerShell.Archive`), zip 전용, 2 GB 제한, 기본 덮어쓰기 안 함 | Windows 자산 zip 형식, `-Force`, 진행률 끄기 | 확인(문서) S43, bun 사례 S30. 도입 버전 (5.0) 문서 기재 미확인 |
| (6) | 아키텍처 판별 | `PROCESSOR_ARCHITECTURE`는 프로세스 기준. x64 에뮬레이션 프로세스 `AMD64`. `cmd.exe`, `powershell.exe` ARM64X 바이너리의 부모 아키텍처 상속 | 5.5절 방식 | 확인(문서) S47, 2차 자료 S48 |
| (7) | 실행 정책 | Windows 클라이언트 기본 Restricted. 원문 "Permits individual commands, but doesn't allow scripts." | `irm \| iex` 문자열 실행 경로 유지. 설치 과정의 `.ps1` 파일 생성, 실행 회피 | 확인(문서) S49. `iex` 문자열 실행의 정책 비적용은 문서 기반 판단 |
| (8) | 인코딩 | raw.githubusercontent.com: `text/plain; charset=utf-8`. 릴리스 자산: `application/octet-stream` (charset 없음) | `install.ps1` ASCII 전용 작성. README 명령은 raw URL 사용 | 확인(실행) E6. 5.1의 charset 부재 응답 해석 방식 미확인 |
| (9) | 세션 종료 | `irm \| iex` 실행 코드의 `exit` 호출 시 사용자 세션 종료 가능성 | `throw`, `return` 사용, 전체 로직 함수 내부 배치 | uv install.ps1 `throw` 사용 확인(소스) S29. `exit` 동작 문서 미조회, 미확인 |
| (10) | API 속도 제한 | 비인증 REST API 시간당 60회, IP 기준 | `releases/latest` 리다이렉트 또는 `SHA256SUMS` 파일명 해석 (6.4절) | 확인(문서) S59 |
| (11) | PATH 수정 | `setx` 1024자 절단, `SetEnvironmentVariable` 왕복 시 REG_EXPAND_SZ 소실 | 5.6절 레지스트리 방식 | 확인(문서) S51, S53, 확인(소스) S52 |
| (12) | PowerShell 7 차이 | `-UseBasicParsing` 6.0 이후 효과 없음. `OSArchitecture` 에뮬레이션 보정 .NET 7 이후 (PowerShell 7.3 이후) | `shell: powershell`, `shell: pwsh` 양쪽 CI 검사 | 확인(문서) S41, S46, S9, uv 주석 S29 |

TLS 처리 초안 (미실행):

```powershell
$p = [int][Net.ServicePointManager]::SecurityProtocol
if ($p -ne 0 -and ($p -band 3072) -eq 0) {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]($p -bor 3072)
}
```

- 수치 근거: `SystemDefault` 0, `Tls12` 3072, `Tls13` 12288 [확인(문서) S44].
- Microsoft 권고 원문: "If you must set ServicePointManager.SecurityProtocol, then set it to SecurityProtocolType.SystemDefault." [S44]. 위 초안은 SystemDefault 상태 무변경, 구형 상태 한정 Tls12 추가.

다운로드와 검증 초안 (미실행):

```powershell
$oldProgress = $ProgressPreference
$ProgressPreference = 'SilentlyContinue'
try {
    Invoke-WebRequest -UseBasicParsing -Uri $sumsUrl -OutFile $sumsPath
    Invoke-WebRequest -UseBasicParsing -Uri $zipUrl -OutFile $zipPath
} finally {
    $ProgressPreference = $oldProgress
}
$line = Get-Content -LiteralPath $sumsPath | Where-Object { $_ -match ('\s\*?' + [regex]::Escape($asset) + '$') }
if (-not $line) { throw "no checksum entry for $asset" }
$expected = ($line -split '\s+')[0]
$actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $zipPath).Hash
if ($actual -ne $expected) { throw "checksum mismatch for $asset" }
```

- PowerShell 문자열 `-ne` 비교의 기본 대소문자 무시 동작: 이번 조사 문서 미조회 (판단 근거 일반 동작).

### 5.5 아키텍처 판별 방식 비교

| 방식 | x64 에뮬레이션 (ARM64 기기) 결과 | 5.1 사용 가능 | 사용 사례 | 근거 |
| --- | --- | --- | --- | --- |
| `$env:PROCESSOR_ARCHITECTURE` | `AMD64` | 가능 | deno (x86_64 고정, 미사용) | 2차 자료 S48 ("For an x64 binary, it will always be set to AMD64") |
| `$env:PROCESSOR_ARCHITEW6432` | 32비트 WOW64 프로세스 한정 설정 | 가능 | 없음 | 확인(문서) S47 |
| `RuntimeInformation.OSArchitecture` | .NET 7 이전 동작과 이후 동작 상이. uv 주석: "It works correctly starting in PowerShell Core 7.3 and Windows PowerShell in Win 11 22H2." | .NET Framework 4.7.1 이후 | uv (`LoadWithPartialName`로 형식 로드, 실패 시 `Is64BitOperatingSystem`) | 확인(문서) S46, 확인(소스) S29 |
| 레지스트리 `HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment` `PROCESSOR_ARCHITECTURE` | 기기 고유 값 (bun 주석: "works even under x64 emulation on ARM64") | 가능 | bun | 확인(소스) S30. Microsoft 공식 문서 미확인 |

- 권고 순서 (판단): (1) `HOWTUI_ARCH` 재정의, (2) 레지스트리 값, (3) `PROCESSOR_ARCHITEW6432`, (4) `PROCESSOR_ARCHITECTURE`. 판별 오류로 x64 자산 설치 시 Windows 11 ARM64 x64 에뮬레이션 실행 가능 (uv 주석 "which is OK since emulation is available", 확인(소스) S29).

```powershell
function Get-HowtuiArch {
    if ($env:HOWTUI_ARCH) { return $env:HOWTUI_ARCH.ToUpperInvariant() }
    try {
        $k = 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment'
        $a = (Get-ItemProperty -Path $k -Name PROCESSOR_ARCHITECTURE).PROCESSOR_ARCHITECTURE
        if ($a) { return $a.ToUpperInvariant() }
    } catch { }
    if ($env:PROCESSOR_ARCHITEW6432) { return $env:PROCESSOR_ARCHITEW6432.ToUpperInvariant() }
    return $env:PROCESSOR_ARCHITECTURE.ToUpperInvariant()
}
# AMD64 -> x86_64-pc-windows-msvc, ARM64 -> aarch64-pc-windows-msvc, 그 외 -> 중단
```

### 5.6 사용자 PATH 수정

| 방식 | 문제 | 근거 |
| --- | --- | --- |
| `setx PATH "%PATH%;<dir>"` | 1024자 초과 절단, 기존 변수 참조 확장 후 기록 | 확인(문서) S51 ("there's a limit of 1024 characters", "Running this command on an existing variable removes any variable references and uses expanded values.") |
| `[Environment]::SetEnvironmentVariable('Path', [Environment]::GetEnvironmentVariable('Path','User') + ';<dir>', 'User')` | 읽기 단계: `RegistryKey.GetValue(name)` 호출, REG_EXPAND_SZ 확장 결과 반환. 쓰기 단계: `RegistryKey.SetValue(name, value)` 호출, 문자열 REG_SZ 저장. `%USERPROFILE%` 등 참조 소실, 값 종류 변경 | 확인(소스) S52 (.NET Framework `environment.cs` 678, 990, 1007행, .NET `Environment.Windows.cs` 411-441행), 확인(문서) S53 ("stores all string values as RegistryValueKind.String, even if they contain expandable references") |
| deno install.ps1 | 위 두 번째 방식 사용 | 확인(소스) S31 |
| HKCU `Environment` 직접 수정 | 미확장 값 읽기 (`DoNotExpandEnvironmentNames`), REG_EXPAND_SZ 쓰기, WM_SETTINGCHANGE 방송 | uv (cargo-dist) 확인(소스) S29, bun 확인(소스) S30 |

- 값 길이: .NET Framework 참조 소스 `MaxEnvVariableValueLength = 32767` 검사 (절단 아님, 초과 시 예외) [확인(소스) S52].
- 권고 초안 (uv 방식 기반, 미실행):

```powershell
function Add-HowtuiUserPath([string]$Dir) {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $true)
    try {
        $raw = $key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        $parts = @($raw -split ';' | Where-Object { $_ -ne '' })
        $norm = $Dir.TrimEnd('\')
        if ($parts | Where-Object { $_.TrimEnd('\') -ieq $norm }) { return $false }
        $key.SetValue('Path', ((@($Dir) + $parts) -join ';'), [Microsoft.Win32.RegistryValueKind]::ExpandString)
    } finally {
        $key.Close()
    }
    # WM_SETTINGCHANGE 방송: uv 방식 (임시 변수 생성 후 삭제)
    $dummy = 'howtui-' + [guid]::NewGuid().ToString()
    [Environment]::SetEnvironmentVariable($dummy, 'howtui-dummy', 'User')
    [Environment]::SetEnvironmentVariable($dummy, [NullString]::Value, 'User')
    $env:Path = "$Dir;$env:Path"
    return $true
}
```

- 현재 세션 반영: 레지스트리 수정은 새 프로세스 대상. 현재 세션 `$env:Path` 별도 갱신 (deno 사례 `$Env:Path += ";${BinDir}"`, 확인(소스) S31).
- 기존 열린 터미널: 재시작 안내 필요 (판단).

### 5.7 설치 위치

| 후보 | 근거 | 특성 |
| --- | --- | --- |
| `%LOCALAPPDATA%\Programs\howtui\bin` | `FOLDERID_UserProgramFiles` 기본 경로 `%LOCALAPPDATA%\Programs` (PERUSER) [확인(문서) S54] | Windows 사용자 단위 프로그램 규약, 앱 전용 디렉터리, 제거 단순 |
| `$HOME\.local\bin` (`XDG_BIN_HOME` 우선) | uv (cargo-dist) 기본 [확인(소스) S29] | Unix 설치 경로와 동일 구조, 다른 도구와 공유 디렉터리 |
| `$HOME\.howtui\bin` | bun `$HOME\.bun`, deno `$HOME\.deno\bin` [확인(소스) S30, S31] | 앱 전용, 비표준 위치 |

- 권고 (판단): 기본 `%LOCALAPPDATA%\Programs\howtui\bin`, 재정의 `HOWTUI_INSTALL_DIR`. 데이터 경로 `%LOCALAPPDATA%\howtui\` (SRS 12.1절)와 분리 유지.

### 5.8 참고 설치 스크립트 비교

| 항목 | uv 0.13.4 (cargo-dist 생성) | bun | deno | starship | atuin v18.24.1 | rustup |
| --- | --- | --- | --- | --- | --- | --- |
| Windows 스크립트 | `install.ps1` | `install.ps1` | `install.ps1` | 없음. `install.sh`에서 MSYS, Cygwin, Git Bash 감지 시 `pc-windows-msvc` | `atuin-installer.ps1` 릴리스 자산 (cargo-dist 형식) | 공식 `rustup-init.ps1` 미발견. `sh.rustup.rs` POSIX sh 전용 |
| 아키텍처 판별 | `RuntimeInformation.OSArchitecture`, 실패 시 `Is64BitOperatingSystem` | HKLM 레지스트리 `PROCESSOR_ARCHITECTURE` | x86_64 고정 | `uname -m`, `getconf LONG_BIT` | 내용 미조회 | ELF 클래스, `/proc/cpuinfo` NEON |
| 다운로드 | `Net.WebClient.DownloadFile` | `curl.exe`, 실패 시 `Invoke-RestMethod -OutFile` | `curl.exe --ssl-revoke-best-effort` | curl 또는 wget | 미조회 | curl 또는 wget |
| 압축 해제 | zip: `Expand-Archive`, tar: `tar` | `Expand-Archive` (진행률 끄기) | `tar.exe` | tar, unzip | 미조회 | 해당 없음 |
| PATH | HKCU 미확장 읽기, ExpandString 쓰기, 임시 변수 방송 | HKCU 직접 쓰기 (`%` 포함 시 ExpandString, 기존 종류 유지), `SendMessageTimeout` 방송 | `SetEnvironmentVariable` 왕복 | 안내 문구 | 미조회 | 해당 없음 |
| 설치 위치 | `XDG_BIN_HOME` 또는 `$HOME/.local/bin` | `$HOME\.bun\bin` | `$HOME\.deno\bin` | `/usr/local/bin` 기본 | 미조회 | 해당 없음 |
| 사전 검사 | PS 5 미만 거부, 실행 정책 Unrestricted, RemoteSigned, Bypass 요구, `Tls12` 열거형 존재 확인 | Windows 10 1809 (빌드 17763) 미만 거부 | 없음 | 없음 | 미조회 | 해당 없음 |
| 체크섬 검증 (스크립트 내) | ps1: 검색 결과 없음. sh: 자산별 sha256 값 내장 | 검색 결과 없음 | 검색 결과 없음 | 검색 결과 없음 | 자산별 `.sha256`, `sha256.sum` 배포 | 1건 (용도 미조회) |
| 근거 | 확인(소스) S29 | 확인(소스) S30 | 확인(소스) S31 | 확인(소스) S28 | 확인(실행) E1b, 확인(소스) S32 | 확인(소스) S27 |

- 시사점 정리 (판단)
  - 아키텍처: bun 레지스트리 방식과 uv `OSArchitecture` 방식 결합.
  - PATH: uv 방식 채택. deno 방식 회피.
  - 체크섬: 참고 PowerShell 설치 스크립트 3종 모두 스크립트 내 검증 부재. howtui `SHA256SUMS` 검증 추가.
  - uv 실행 정책 검사 (Restricted 거부): howtui 미채택. 근거: `iex` 문자열 실행 경로, 설치 과정 `.ps1` 생성 없음.

## 6. 릴리스 자산 이름과 무결성 검증

### 6.1 자산 이름 규칙 권고

| 항목 | 권고 | 근거 |
| --- | --- | --- |
| 태그 | `vX.Y.Z` (SemVer, `Cargo.toml` 버전과 일치 검사) | tu `release.yml` 태그와 Cargo.toml 일치 검사 확인(소스) S17 |
| Unix 자산 | `howtui-X.Y.Z-<target>.tar.gz` | tu 규칙 `tu-<ver>-<triple>.tar.gz` S17, 스킬 S18 |
| Windows 자산 | `howtui-X.Y.Z-<target>.zip` | `Expand-Archive` zip 전용 S43 |
| 압축 내부 | 최상위 `howtui` 또는 `howtui.exe`, `README.md`, `LICENSE` | tu `package-all.sh` 확인(소스) S17 |
| 체크섬 | `SHA256SUMS` (`<hex>  <파일명>` 형식, 전 자산과 설치 스크립트 포함) | tu `SHA256SUMS` S17 |
| 설치 스크립트 | `install.sh`, `install.ps1` 릴리스 자산 동시 첨부 | 스킬 "Ship `install.sh` as a release asset too" S18 |
| 패키지 별칭 | `.deb`, `.rpm` 미배포 시 불필요 | 스킬 S18 |

- 대안 (참고): 버전 없는 자산 이름 (`uv-<target>.tar.gz`, `starship-<target>.zip`) [확인(실행) E1b]. 장점: `releases/latest/download/<asset>` 직접 사용, 버전 해석 생략. 단점: 다운로드 파일명의 버전 정보 부재, 소유자 기존 규칙과 불일치. 판단: 소유자 규칙 (버전 포함) 유지, 6.4절 방식으로 버전 해석.

### 6.2 체크섬과 증명 신뢰 수준

| 수단 | 검출 범위 | 한계 | 설치 스크립트 사용 | 근거 |
| --- | --- | --- | --- | --- |
| 같은 릴리스 `SHA256SUMS` | 전송 손상, 잘림, 잘못된 자산 선택 | 릴리스 쓰기 권한 보유자의 자산과 `SHA256SUMS` 동시 교체 미검출 (판단) | 필수 | 판단 |
| GitHub 자산 `digest` 필드 | 업로드 이후 변경 여부 (업로드 시점 계산, 불변 표기) | API 조회 필요 (비인증 시간당 60회). 자산 삭제 후 재업로드 시 새 digest 계산 여부 미확인 | 선택 (API 의존으로 기본 경로 제외) | 확인(문서) S56, S59 |
| immutable releases | 게시 후 자산 추가, 수정, 삭제 차단, 태그 이동, 삭제 차단, 릴리스 증명 자동 생성 | 활성화 이후 신규 릴리스 한정. 게시 전 draft 단계 자산 첨부 필요 | 사용자 수동 검증 (`gh release verify`, `gh release verify-asset`) | 확인(문서) S57 |
| artifact attestation (`actions/attest@v4`) | 산출물 digest와 빌드 워크플로 신원 결합 (SLSA provenance, Sigstore) | `gh` CLI 필요, POSIX sh 내 검증 비현실적 (판단) | 사용자 수동 검증 (`gh attestation verify <파일> -R hungryZoo/howtui`) | 확인(문서) S58 |

- `digest` 필드 실측: tu v1.1.5 자산 전체 `sha256:` 접두 digest 노출 [확인(실행) E1]. 로컬 sha256과 API digest 일치 (3개 자산) [확인(실행) E8].
- immutable 필드 실측: tu v1.1.5 `"immutable": false` [확인(실행) E1c]. tu 아티팩트 증명 조회 결과 404 [E1c].
- immutable releases GA: 2025-10-28 changelog [S57]. 설정 위치: 저장소 Settings 의 Releases 절 "Enable release immutability" (검색 결과 요약 기준, 문서 원문 미조회).
- artifact attestation 가용성 원문: "Artifact attestations are available in public repositories for all current GitHub plans." [확인(소스) S58 attest-build-provenance README]
- `actions/attest-build-provenance` v4: "simply a wrapper on top of `actions/attest`", 신규 구현 `actions/attest` 권장 [확인(소스) S58].
- `actions/attest` 필요 권한: `id-token: write`, `attestations: write`, `artifact-metadata: write`. 입력 `subject-path` (glob, 목록, 최대 1024개), `subject-checksums` (체크섬 파일) [확인(소스) S58].
- 권고 (판단)
  - (1) 설치 스크립트: `SHA256SUMS` 다운로드와 검증 필수, 검증 도구 부재 시 중단 (Linux `sha256sum`, macOS `shasum -a 256`, Windows `Get-FileHash`).
  - (2) 저장소: immutable releases 첫 릴리스 전 활성화.
  - (3) 릴리스 워크플로: `actions/attest@v4` + `subject-checksums: SHA256SUMS` 증명 생성.
  - (4) README 고급 검증 절: `gh release verify-asset vX.Y.Z <파일>`, `gh attestation verify <파일> -R hungryZoo/howtui`.

### 6.3 설치 URL (최신, 버전 고정)

| 용도 | URL 형식 | 관찰 결과 | 근거 |
| --- | --- | --- | --- |
| 최신 sh | `https://raw.githubusercontent.com/hungryZoo/howtui/main/scripts/install.sh` | tu 동일 형식: `text/plain; charset=utf-8`, `cache-control: max-age=300` | 확인(실행) E6 |
| 최신 sh (자산) | `https://github.com/hungryZoo/howtui/releases/latest/download/install.sh` | tu 동일 형식: 302 → `releases/download/v1.1.5/install.sh` | E6 |
| 고정 sh (자산) | `https://github.com/hungryZoo/howtui/releases/download/vX.Y.Z/install.sh` | tu 동일 형식: 302 → `release-assets.githubusercontent.com`, `application/octet-stream`, `content-disposition: attachment` | E6 |
| 고정 sh (저장소 태그) | `https://raw.githubusercontent.com/hungryZoo/howtui/vX.Y.Z/scripts/install.sh` | 태그 참조 raw URL 동작 howtui 실측 없음 | 미확인 |
| 최신 ps1 | `irm https://raw.githubusercontent.com/hungryZoo/howtui/main/scripts/install.ps1 \| iex` | raw URL charset utf-8 응답 | 판단 (E6 형식 기준) |
| 고정 ps1 | `$env:HOWTUI_VERSION='X.Y.Z'; irm <위 URL> \| iex` 또는 태그 raw URL | 릴리스 자산 URL의 `application/octet-stream` 응답과 5.1 `irm \| iex` 조합 동작 미확인 | 미확인 |

- 판단
  - README 기본 명령: raw `main` URL (설치 스크립트 수정 즉시 반영, 최대 5분 캐시).
  - 버전 고정 수단: 환경변수 (`HOWTUI_VERSION`) 우선. 스크립트 자체 고정 필요 시 태그 raw URL 또는 릴리스 자산 URL.
  - 저장소 경로: 소유자 스킬은 저장소 최상위 `install.sh` [S18], 본 조사 요청은 `scripts/install.sh`. 경로 확정 후 README, 릴리스 업로드, CI 검사 경로 일치 필요.

### 6.4 설치 스크립트 버전 해석 방식

| 방식 | sh | PowerShell | API 제한 | 근거 |
| --- | --- | --- | --- | --- |
| (a) `releases/latest` 리다이렉트 추적 | `curl -fsSLI -o /dev/null -w '%{url_effective}'` | 리다이렉트 비추적 요청 후 `Location` 헤더 해석 (구현 방식 미검증) | 없음 | 확인(소스) S18 |
| (b) REST API `releases/latest` | `tag_name` 추출 | `Invoke-RestMethod` JSON | 시간당 60회 (IP 기준) | 확인(문서) S59 |
| (c) `releases/latest/download/SHA256SUMS` 파일명 해석 | 파일명의 `howtui-X.Y.Z-` 부분 추출 | 동일 | 없음 | 판단 |

- 권고 (판단): sh 기존 방식 (a) 우선, (b) 대체 (스킬 규칙). PowerShell (c) 우선, (b) 대체. (c) 장점: 체크섬 파일 선행 다운로드와 버전 해석 동시 처리, 리다이렉트 처리 불필요. (c) 한계: 두 요청 사이 신규 릴리스 게시 시 불일치 가능 (해석 버전 고정 URL로 이후 요청 수행 시 영향 제거).

## 7. CI 구성 권고

### 7.1 PR CI 매트릭스

| job | 러너 | 내용 | 비고 |
| --- | --- | --- | --- |
| lint | `ubuntu-24.04` | `cargo fmt --all -- --check`, `cargo clippy --all-targets --locked -- -D warnings` | tu `ci.yml` 구성 계승 S17 |
| test | `ubuntu-24.04`, `ubuntu-24.04-arm`, `macos-15`, `windows-2025`, `windows-11-arm` | `cargo test --locked` | 전 라벨 무료 S1 |
| linux-cross | `ubuntu-24.04` | armv7 `.2.17` zigbuild 빌드, glibc 최댓값 관문 | 접미사 누락 회귀 조기 검출 |
| installer | `ubuntu-24.04`, `macos-15`, `windows-2025` | `sh -n`, shellcheck, `dash` 실행 검사, `install.ps1` 구문 검사 (`shell: powershell`, `shell: pwsh`) | 실제 설치 검사는 첫 릴리스 이후 |

```yaml
name: ci
on:
  pull_request:
  push:
    branches: [main]
permissions:
  contents: read
concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true
jobs:
  lint:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v7
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
        with:
          save-if: ${{ github.ref == 'refs/heads/main' }}
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-targets --locked -- -D warnings
  test:
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-24.04, ubuntu-24.04-arm, macos-15, windows-2025, windows-11-arm]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v7
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          save-if: ${{ github.ref == 'refs/heads/main' }}
      - run: cargo test --locked
```

- 위 YAML: 구성 초안 (미실행). 액션 참조는 7.4절 버전의 커밋 SHA 고정 권장 (uv 사례: `actions/checkout@<sha> # v7.0.1`, 확인(소스) S33).

### 7.2 릴리스 워크플로 단계

| 단계 | 러너 | 내용 | 근거 |
| --- | --- | --- | --- |
| (1) 버전 검사 | `ubuntu-24.04` | 태그 `vX.Y.Z`와 `Cargo.toml` 버전 일치 | tu `release.yml` S17 |
| (2) Linux 빌드 | `ubuntu-24.04` | 3개 gnu 대상 zigbuild `.2.17`, glibc 관문, tar.gz 생성 | 2.6절 |
| (3) macOS 빌드 | `macos-15` | 2개 대상, `MACOSX_DEPLOYMENT_TARGET=11.0`, `codesign -v`, `minos` 검사, tar.gz 생성 | 4.5절 |
| (4) Windows 빌드 | `windows-2025` (x86_64), `windows-11-arm` (aarch64) | `+crt-static`, `dumpbin /dependents` 검사, zip 생성 | 5.2절 |
| (5) 실행 검사 | `ubuntu-24.04` (x86_64, armv7 QEMU), `ubuntu-24.04-arm` (aarch64), `macos-15`, `macos-15-intel`, Windows 2종 | `--version` 실행, 구형 사용자 공간 컨테이너 포함 | 2.7절 |
| (6) 게시 | `ubuntu-24.04` | 산출물 수집, `SHA256SUMS` 생성, `actions/attest@v4`, draft 릴리스 생성, 전 자산 업로드, 게시 | S57, S58 |

- 게시 job 권한: `contents: write`, `id-token: write`, `attestations: write`, `artifact-metadata: write` [S58].
- immutable releases 호환 (판단): draft 생성 후 전 자산 업로드, 마지막 단계 게시. 게시 후 재업로드 불가 [S57] → (2)-(5) 전 검사 통과 후 (6) 진입.
- `gh release create --draft`, `gh release upload`, `gh release edit --draft=false` 조합: 이번 조사 미실행, 미확인.
- 설치 스크립트 실설치 검사: 게시 후 별도 job (`install.sh` Ubuntu, macOS, `install.ps1` `shell: powershell`, `shell: pwsh`).

### 7.3 캐시

- `Swatinem/rust-cache@v2`: 입력 `shared-key`, `key`, `cache-on-failure`, `save-if`, `cache-bin` 등 [확인(소스) S60].
- 캐시 한도 원문: "caches are currently limited to 10 GB in total and exceeding that limit will cause eviction of older caches." [S60]
- 권고 (판단): `save-if: ${{ github.ref == 'refs/heads/main' }}`로 PR 캐시 저장 억제. 릴리스 job 캐시 미사용 (재현성 우선).

### 7.4 액션과 도구 버전 (2026-10-22 조회)

| 액션 또는 도구 | 최신 릴리스 | 게시일 | 비고 | 근거 |
| --- | --- | --- | --- | --- |
| `actions/checkout` | v7.0.1 | 2026-07-20 | tu `@v4` 사용 중 | 확인(실행) E7, S17 |
| `actions/upload-artifact` | v7.0.2 | 2026-10-07 | | E7 |
| `actions/download-artifact` | v8.0.2 | 2026-10-07 | | E7 |
| `actions/cache` | v6.1.0 | 2026-06-26 | 직접 사용 불필요 (rust-cache 사용 시) | E7 |
| `actions/attest` | v4.2.2 | 2026-08-04 | 신규 구현 권장 | E7, S58 |
| `actions/attest-build-provenance` | v4.2.2 | 2026-08-06 | v4부터 `actions/attest` 래퍼 | E7, S58 |
| `Swatinem/rust-cache` | v2.9.3 | 2026-10-20 | `@v2` | E7 |
| `dtolnay/rust-toolchain` | 릴리스 없음 | | 브랜치 참조 (`@stable`) | E7 |
| `taiki-e/install-action` | v2.87.29 | 2026-10-16 | 도구 설치 대안 | E7 |
| `softprops/action-gh-release` | v3.0.3 | 2026-08-30 | `gh` CLI 대안 | E7 |
| `cargo-bins/cargo-binstall` | v1.26.0 | 2026-10-17 | | E7 |
| `axodotdev/cargo-dist` | v0.33.1 | 2026-10-21 | 미채택 참고 | E7 |
| `rust-cross/cargo-zigbuild` | v0.24.0 | 2026-10-21 | PyPI linux x86_64 wheel 부재 (조회 시점), 0.23.4 실험 사용 | E5, E7 |
| `ziglang` (PyPI) | 0.17.0 | 2026-10-08 | | E5 |
| `cross-rs/cross` | v0.2.5 | 2023-02-04 | | E7 |
| `gh` CLI | v2.103.0 | 2026-10-21 | | E7 |

### 7.5 tu 워크플로 대비 변경점

| 항목 | tu 현황 | howtui 권고 | 근거 |
| --- | --- | --- | --- |
| 빌드 구조 | 단일 Ubuntu job, zigbuild로 Linux와 macOS 전체 | Linux zigbuild job, macOS 네이티브 job, Windows 네이티브 job 2종 | S17, 2.7절 |
| glibc | 접미사 없음 (실측 2.30, 2.34) | `.2.17` 접미사, 관문 검사 | E2, E5 |
| 도구 버전 | `pip install cargo-zigbuild ziglang` 미고정 | 버전 고정 | S17, 2.3절 |
| macOS 최소 버전 | 미지정 (실측 minos 15.0) | `MACOSX_DEPLOYMENT_TARGET=11.0` | E3, 4.4절 |
| 게시 순서 | 릴리스 생성 후 `gh release upload --clobber` | draft 생성, 업로드, 게시 (immutable 호환) | S17, S57 |
| 증명 | 없음 (조회 404) | `actions/attest@v4` | E1c, S58 |
| 액션 버전 | `actions/checkout@v4` | v7 계열, SHA 고정 | S17, E7 |
| 체크섬 생성 | `shasum -a 256` (`package-all.sh`) | Linux 게시 job `sha256sum` (출력 형식 동일 여부 미검증) | S17 |

## 8. SRS 반영 후보

| SRS 항목 | 현재 기재 | 반영 후보 (판단) | 근거 |
| --- | --- | --- | --- |
| SRS-CMP-001 | Linux glibc 하한 2.17 목표 | 2.17 유지, 의존성 사유 상향 상한 2.28 명시, zigbuild 접미사 필수와 CI 관문 명시 | 2.6절 |
| SRS-CMP-005 | macOS 최소 버전: Rust 타깃 기본값 기준 | x86_64 기본 10.12와 Xcode 16.4 하한 10.13 불일치. `MACOSX_DEPLOYMENT_TARGET=11.0` 명시 | 4.4절 |
| 13절 (플랫폼별 빌드와 릴리스) | 조사 반영 예정 | 2.7절 대상별 표, 7.2절 단계 표 | 본 문서 |
| 14절 (설치 스크립트 설계) | 조사 반영 예정 | 3.6절 판별 절차, 5.4-5.7절, 6.3-6.4절 | 본 문서 |

## 9. 미확인 사항과 후속 검증 항목

| 번호 | 항목 | 검증 수단 |
| --- | --- | --- |
| (1) | tu armv7 자산의 Bullseye armhf 실행 실패, x86_64 자산의 Buster 실행 실패 (판단 상태) | Debian 10, 11 컨테이너 실행 |
| (2) | armv7 산출물 `Tag_Advanced_SIMD_arch: NEONv1` 유래와 NEON 미지원 CPU 영향 | 산출물 역어셈블, Pi 2 v1.1 실기 |
| (3) | Pi 4, Pi 5 32비트 OS의 `uname -m`, ELF 판별 결과 | Pi 실기 |
| (4) | Pi 2 v1.2 64비트 OS 실행 가능 여부 (비공식) | 실기 |
| (5) | macOS arm64 러너 Rosetta 2 설치 여부 | 러너 `arch -x86_64 true` |
| (6) | `ubuntu-24.04-arm` 러너 AArch32 실행 가능 여부 | 러너 armv7 바이너리 실행 |
| (7) | Xcode 16.4, 26.x에서 배포 대상 하한 미만 지정 시 링커 동작 | `macos-15` 러너 빌드 |
| (8) | `vcruntime140.dll` 의존 제거 확인 | `dumpbin /dependents` |
| (9) | PS 5.1 `irm`의 `application/octet-stream` 응답 해석, `iex` 내 `exit` 동작 | `windows-2025` `shell: powershell` |
| (10) | HKLM `PROCESSOR_ARCHITECTURE` 값의 x64 에뮬레이션 프로세스 조회 결과 | `windows-11-arm` 러너 x64 PowerShell 7 |
| (11) | Smart App Control의 무서명 CLI 차단 범위 | Windows 11 enforcement 환경 |
| (12) | `macos-26-intel` 종료 일정 | GitHub changelog 추적 |
| (13) | rustls 암호 백엔드의 zigbuild `.2.17` 빌드 가능 여부 | linux-cross job |
| (14) | `gh release edit --draft=false` 게시 흐름과 immutable 조합 | 시험 저장소 |
| (15) | GitHub `digest` 필드의 자산 재업로드 시 동작 | 시험 저장소 |

## 10. 출처와 실행 기록

### 10.1 출처

- S1 GitHub-hosted runners reference: https://docs.github.com/en/actions/reference/runners/github-hosted-runners
- S2 actions/runner-images README: https://github.com/actions/runner-images
- S3 GitHub changelog, macOS 13 runner image closing down (2025-09-19): https://github.blog/changelog/2025-09-19-github-actions-macos-13-runner-image-is-closing-down/
- S4 runner-images issue #13518 (macOS 14 deprecation): https://github.com/actions/runner-images/issues/13518
- S5 Ubuntu 24.04 Arm64 image README: https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2404-Arm64-Readme.md
- S6 macOS 15 Arm64 image README: https://github.com/actions/runner-images/blob/main/images/macos/macos-15-arm64-Readme.md
- S7 Windows Server 2025 VS2026 image README: https://github.com/actions/runner-images/blob/main/images/windows/Windows2025-VS2026-Readme.md
- S8 Windows 11 Arm64 image README: https://github.com/actions/runner-images/blob/main/images/windows/Windows11-Arm64-Readme.md
- S9 Workflow syntax (`shell`): https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax
- S10 Rust platform support: https://doc.rust-lang.org/nightly/rustc/platform-support.html
- S11 Rust reference, linkage (static and dynamic C runtimes): https://doc.rust-lang.org/reference/linkage.html
- S12 cargo-zigbuild README: https://github.com/rust-cross/cargo-zigbuild
- S13 zig `lib/std/Target.zig`: 0.14.1 https://github.com/ziglang/zig/blob/0.14.1/lib/std/Target.zig , 0.15.2 https://codeberg.org/ziglang/zig/src/tag/0.15.2/lib/std/Target.zig , 0.16.0 https://codeberg.org/ziglang/zig/src/tag/0.16.0/lib/std/Target.zig , 0.17.0 https://codeberg.org/ziglang/zig/src/tag/0.17.0/lib/std/Target.zig
- S14 zig `lib/std/zig/target.zig` 0.17.0: https://codeberg.org/ziglang/zig/src/tag/0.17.0/lib/std/zig/target.zig
- S15 cross-rs README (main): https://github.com/cross-rs/cross , v0.2.5: https://github.com/cross-rs/cross/blob/v0.2.5/README.md
- S16 PyPI JSON: https://pypi.org/pypi/ziglang/json , https://pypi.org/pypi/cargo-zigbuild/json
- S17 hungryZoo/tu: https://github.com/hungryZoo/tu (`.github/workflows/ci.yml`, `.github/workflows/release.yml`, `scripts/build-all.sh`, `scripts/package-all.sh`, `scripts/meta.sh`, `install.sh`), 릴리스 https://github.com/hungryZoo/tu/releases/tag/v1.1.5
- S18 소유자 스킬: `$HOME/.claude/skills/cross-platform-cli-distribution/SKILL.md`, `reference/install.sh`
- S19 Raspberry Pi processors: https://www.raspberrypi.com/documentation/computers/processors.html
- S20 Raspberry Pi hardware (boot modes 절): https://www.raspberrypi.com/documentation/computers/raspberry-pi.html
- S21 Raspberry Pi OS 다운로드: https://www.raspberrypi.com/software/operating-systems/
- S22 config.txt: https://www.raspberrypi.com/documentation/computers/config_txt.html
- S23 Raspberry Pi OS (armhf) release notes: https://downloads.raspberrypi.com/raspios_armhf/release_notes.txt
- S24 Raspberry Pi 저장소 glibc: https://archive.raspberrypi.com/debian/pool/main/g/glibc/ , Raspbian 저장소: https://archive.raspbian.org/raspbian/pool/main/g/glibc/
- S25 Debian libc6 패키지 검색: https://packages.debian.org/search?keywords=libc6&searchon=names&exact=1&suite=all&section=all , Debian madison (openssl): https://api.ftp-master.debian.org/madison?package=openssl&text=on
- S26 Launchpad API: https://api.launchpad.net/1.0/ubuntu/+archive/primary?ws.op=getPublishedSources&source_name=glibc&exact_match=true
- S27 rustup-init.sh: https://sh.rustup.rs
- S28 starship install.sh: https://starship.rs/install.sh
- S29 uv 설치 스크립트 (0.13.4): https://astral.sh/uv/install.sh , https://astral.sh/uv/install.ps1
- S30 bun install.ps1: https://bun.sh/install.ps1
- S31 deno install.ps1: https://deno.land/install.ps1
- S32 atuin setup 스크립트: https://setup.atuin.sh , 릴리스 https://github.com/atuinsh/atuin/releases/tag/v18.24.1
- S33 uv `build-release-binaries.yml`: https://github.com/astral-sh/uv/blob/main/.github/workflows/build-release-binaries.yml , uv `.cargo/config.toml`: https://github.com/astral-sh/uv/blob/main/.cargo/config.toml , ripgrep `.cargo/config.toml`: https://github.com/BurntSushi/ripgrep/blob/master/.cargo/config.toml
- S34 Apple, macOS Big Sur 11.0.1 Universal Apps Release Notes: https://developer.apple.com/documentation/macos-release-notes/macos-big-sur-11_0_1-universal-apps-release-notes
- S35 Apple, Notarizing macOS software before distribution: https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution , Customizing the notarization workflow: https://developer.apple.com/documentation/security/customizing-the-notarization-workflow
- S36 Apple, LSFileQuarantineEnabled: https://developer.apple.com/documentation/bundleresources/information-property-list/lsfilequarantineenabled
- S37 Apple Developer Forums, Testing a Notarised Product: https://developer.apple.com/forums/thread/130560
- S38 Unit 42, Gatekeeper bypass (2차 자료): https://unit42.paloaltonetworks.com/gatekeeper-bypass-macos/
- S39 Apple, Xcode support: https://developer.apple.com/support/xcode/
- S40 Microsoft, Universal CRT deployment: https://learn.microsoft.com/en-us/cpp/windows/universal-crt-deployment
- S41 Microsoft, Invoke-WebRequest (5.1): https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/invoke-webrequest?view=powershell-5.1
- S42 Microsoft, KB5074596 (PowerShell 5.1 Invoke-WebRequest 보안 프롬프트): https://support.microsoft.com/KB/5074596
- S43 Microsoft, Get-FileHash (5.1): https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/get-filehash?view=powershell-5.1 , Expand-Archive (5.1): https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.archive/expand-archive?view=powershell-5.1
- S44 Microsoft, TLS best practices with .NET Framework: https://learn.microsoft.com/en-us/dotnet/framework/network-programming/tls , SecurityProtocolType: https://learn.microsoft.com/en-us/dotnet/api/system.net.securityprotocoltype
- S45 GitHub blog, weak cryptographic standards removal: https://github.blog/2018-02-01-crypto-removal-notice/ , https://github.blog/2018-02-23-weak-cryptographic-standards-removed/
- S46 Microsoft, RuntimeInformation.OSArchitecture: https://learn.microsoft.com/en-us/dotnet/api/system.runtime.interopservices.runtimeinformation.osarchitecture
- S47 Microsoft, WOW64 Implementation Details: https://learn.microsoft.com/en-us/windows/win32/winprog64/wow64-implementation-details
- S48 Linaro Windows on Arm wiki, Identify CPU architecture (2차 자료): https://linaro.atlassian.net/wiki/spaces/WOAR/pages/28869623809/Identify+CPU+architecture
- S49 Microsoft, about_Execution_Policies (5.1): https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_execution_policies?view=powershell-5.1
- S50 Microsoft, Smart App Control: https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/overview
- S51 Microsoft, setx: https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/setx
- S52 .NET Framework 참조 소스 `environment.cs`: https://github.com/microsoft/referencesource/blob/master/mscorlib/system/environment.cs , .NET `Environment.Windows.cs`: https://github.com/dotnet/runtime/blob/main/src/libraries/System.Private.CoreLib/src/System/Environment.Windows.cs
- S53 Microsoft, RegistryKey.SetValue: https://learn.microsoft.com/en-us/dotnet/api/microsoft.win32.registrykey.setvalue , RegistryKey.GetValue: https://learn.microsoft.com/en-us/dotnet/api/microsoft.win32.registrykey.getvalue
- S54 Microsoft, KNOWNFOLDERID: https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid
- S55 PowerShell issue #2138 (Progress bar can significantly impact cmdlet performance): https://github.com/PowerShell/PowerShell/issues/2138
- S56 GitHub changelog, release asset digests (2025-06-03): https://github.blog/changelog/2025-06-03-releases-now-expose-digests-for-release-assets/
- S57 GitHub, immutable releases: https://docs.github.com/en/code-security/supply-chain-security/understanding-your-software-supply-chain/immutable-releases , 검증: https://docs.github.com/en/code-security/supply-chain-security/understanding-your-software-supply-chain/verifying-the-integrity-of-a-release , GA changelog (2025-10-28): https://github.blog/changelog/2025-10-28-immutable-releases-are-now-generally-available/
- S58 GitHub, artifact attestations: https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations , actions/attest README: https://github.com/actions/attest , actions/attest-build-provenance README: https://github.com/actions/attest-build-provenance
- S59 GitHub REST API rate limits: https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api
- S60 Swatinem/rust-cache README: https://github.com/Swatinem/rust-cache
- S61 Phoronix, Rust 1.91 (2차 자료): https://phoronix.com/news/Rust-1.91-Released , RFC 3817: https://rust-lang.github.io/rfcs/3817-promote-aarch64-pc-windows-msvc-to-tier-1.html
- S62 Pi 5 페이지 크기 (2차 자료): https://pimylifeup.com/raspberry-pi-page-size/ , https://support.ipconfigure.com/hc/en-us/articles/32362653709709-Change-Kernel-Page-Size-for-Raspberry-Pi-5-Devices
- S63 Eric Lawrence, Downloads and the Mark-of-the-Web (2차 자료): https://textslashplain.com/2016/04/04/downloads-and-the-mark-of-the-web
- S64 프로젝트 문서: `docs/research/spikes.md` (SPIKE-02), `docs/SRS.md` (SRS-CMP-001, SRS-CMP-005, 13절, 14절)

### 10.2 실행 기록 (2026-10-22, 로컬 Ubuntu 20.04 x86_64)

- E0 `gh api repos/hungryZoo/howtui` → `visibility: public`, `default_branch: main`, `created_at: 2026-10-21T18:25:22Z`
- E1 `gh api repos/hungryZoo/tu/releases/latest` → v1.1.5 (2026-10-18), 자산 21종 전부 `digest: sha256:...` 노출, musl 자산 포함
- E1b `gh api repos/{atuinsh/atuin, starship/starship, astral-sh/uv}/releases/latest` → 자산 이름 목록 (버전 없는 `<app>-<target>` 형식, `.sha256`, `sha256.sum`, `atuin-installer.ps1`)
- E1c `gh api repos/hungryZoo/tu/releases/latest --jq '{immutable}'` → `false`. `gh api repos/hungryZoo/tu/attestations/sha256:<x86_64 gnu digest>` → 404
- E2 tu v1.1.5 Linux 자산 `file`, `objdump -T`, `readelf -V`, `readelf -A`, `readelf -p .comment` (새 빈 디렉터리 `scratchpad/clones/tu-release-1.1.5/`, 실행 없음)
- E3 tu v1.1.5 macOS 자산 Mach-O 적재 명령 해석 (`scratchpad/scripts/macho_info.py`, `uv run --no-project python -I`) → aarch64 `codesig True`, `minos 15.0.0`, `sdk 27.0.0`, CodeDirectory flags `0x20002`. x86_64 `codesig False`, `minos 15.0.0`
- E4 rustc 1.96.0: `rustc --print deployment-target --target {x86_64,aarch64}-apple-darwin` (기본값, `MACOSX_DEPLOYMENT_TARGET=10.15` 지정), `rustc --print cfg` (`-C target-feature=+crt-static` 포함), `RUSTC_BOOTSTRAP=1 rustc -Z unstable-options --print target-spec-json --target <7개 대상>`
- E5 `uv tool run --from 'cargo-zigbuild<0.24' --with 'ziglang==0.17.0' cargo-zigbuild zigbuild --release --target <접미사 변형>` (시험 crate `scratchpad/zigtest/hello`, 대상별 별도 `CARGO_TARGET_DIR`), 결과 `objdump -T`, `readelf -V`, `readelf -A`. `cargo-zigbuild==0.24.0` 지정 시 wheel 부재 오류. PyPI JSON 조회 (ziglang 0.17.0, cargo-zigbuild 0.24.0)
- E6 `curl -sSI` / `curl -D -`: raw.githubusercontent.com tu `install.sh` (200, `text/plain; charset=utf-8`, `cache-control: max-age=300`), `releases/latest/download/install.sh` (302 → `releases/download/v1.1.5/install.sh`), `releases/download/v1.1.5/install.sh` (302 → `release-assets.githubusercontent.com`, 200, `application/octet-stream`, `content-disposition: attachment; filename=install.sh`)
- E7 `gh api repos/<owner>/<repo>/releases/latest` (7.4절 표 전체), `gh api repos/dtolnay/rust-toolchain/branches`
- E8 tu v1.1.5 자산 3종 로컬 `sha256sum`과 API `digest` 비교 → 일치
- E9 Launchpad API (glibc, openssl: focal, jammy, noble, resolute), Debian madison (openssl)
- E10 archive.raspberrypi.com `pool/main/g/glibc/`, archive.raspbian.org `pool/main/g/glibc/` 목록, downloads.raspberrypi.com `raspios_armhf/release_notes.txt`, `raspios_arm64/images/` 목록
- E11 `getconf GNU_LIBC_VERSION` (`glibc 2.31`), `getconf LONG_BIT` (`64`)
