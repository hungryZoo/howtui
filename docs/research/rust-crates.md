# howtui Rust 크레이트 조사

- 조사일: 2026-10-22
- 조사 환경: Linux x86_64 (Ubuntu 20.04.6 LTS, glibc 2.31), rustc 1.96.0 (2026-05-25), cargo 1.96.0, cargo-zigbuild 0.22.3, ziglang 0.17.0 (uv 임시 환경)
- 실험 프로젝트: `<로컬 임시 디렉터리>/crates-probe/` (로그는 같은 경로의 `logs/`)
- 근거 표기
  - 확인(문서): crates.io API, docs.rs, README, 공식 문서 기준
  - 확인(소스): `$HOME/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/<crate>-<ver>/` 소스 파일과 줄 번호 기준
  - 확인(실행): 이번 조사에서 직접 빌드 또는 실행한 결과 기준
  - 미확인: 근거 부족 또는 실행 환경 부재

## 1. 핵심 결론 요약

(1) ratatui 0.30 계열 사용. 0.30부터 `ratatui-core`, `ratatui-widgets`, `ratatui-crossterm` 분리 구조. 애플리케이션은 `ratatui` 단일 의존성 권장.

(2) stdout 순수성 요구와 `Viewport::Inline` 사이의 충돌 확인. crossterm 0.29 Unix `cursor::position()`의 CPR 질의(`ESC[6n`)가 stdout으로 출력되는 구조. `CrosstermBackend::new(stderr())` 사용 시에도 `cmd=$(howtui)` 환경에서 질의 문자열이 캡처 결과에 섞이고 2초 후 오류 발생(확인(실행)). 해결책: CPR만 `/dev/tty`로 보내는 Backend 래퍼(약 60줄) 작성. 래퍼 적용 후 stdout 순수성 확인(확인(실행)).

(3) `ratatui::init_with_options`, `ratatui::restore`, `ratatui::run` 사용 금지. 세 함수 모두 stdout 고정이며 `restore`는 stdout에 `LeaveAlternateScreen` 시퀀스 출력(확인(소스)). `Terminal::with_options` 직접 구성과 자체 panic hook 필요.

(4) textarea 위젯: `ratatui-textarea` 0.9.3 권장. `tui-textarea` 0.7.0은 ratatui 0.29와 crossterm 0.28 의존으로 ratatui 0.30과 비호환.

(5) TLS: rustls와 ring 조합 권장(`reqwest`의 `rustls-no-provider` 기능과 `rustls/ring`). reqwest 0.13 기본값은 rustls와 aws-lc-rs 조합. 두 provider 모두 대상별 C 컴파일러 필요. cargo-zigbuild로 armv7 및 aarch64 Linux 빌드 성공(확인(실행)). native-tls는 Linux에서 OpenSSL 동적 링크(libssl.so.1.1)로 배포 호환성 문제.

(6) keyring 4.2.0 기본(`v1`) 기능: macOS Keychain, Windows Credential Manager, Linux Secret Service(zbus, 순수 Rust). 빌드 시 libdbus 불필요. Secret Service 부재 시 즉시 `NoDefaultStore` 오류(확인(실행)). 0600 권한 파일 fallback 설계 필요.

(7) OAuth와 OIDC: `oauth2`, `openidconnect`, `jsonwebtoken`의 `rust_crypto` 기능 모두 중복 의존성 유발. PKCE, state, nonce는 `getrandom` + `sha2` + `base64` 직접 구현, ID 토큰 RS256 검증은 TLS용으로 이미 링크된 ring의 `RsaPublicKeyComponents::verify` 직접 사용 권장. 콜백 수신은 `tokio::net::TcpListener` + `httparse`(reqwest 경유 기존 의존성) 권장.

(8) 테스트: ratatui `TestBackend` + `insta` 스냅샷, `assert_cmd`, `portable-pty` 기반 PTY 종단 테스트 조합 권장. `rexpect`는 Windows 빌드 실패(확인(실행)).

## 2. 버전 표

crates.io API(`https://crates.io/api/v1/crates/<name>`) 조회 결과. MSRV는 각 버전의 `rust_version` 필드. `null`은 crate 메타데이터에 미표기.

| 크레이트 | 최신 안정 버전 | 배포일 | MSRV | 비고 |
| --- | --- | --- | --- | --- |
| ratatui | 0.30.2 | 2026-06-19 | 1.88.0 | 0.30.0(2025-12-26)부터 분리 구조 |
| ratatui-core | 0.1.2 | 2026-06-19 | 1.88.0 | Terminal, Backend 트레이트, TestBackend |
| ratatui-widgets | 0.3.2 | 2026-06-19 | 1.88.0 | 기본 위젯 |
| ratatui-crossterm | 0.1.2 | 2026-06-19 | 1.88.0 | `crossterm_0_28`, `crossterm_0_29` 기능, 기본 0.29 |
| crossterm | 0.29.0 | 2025-04-05 | 1.63.0 | 0.29 이후 미배포 |
| tokio | 1.54.0 | 2026-10-19 | 1.85 | |
| reqwest | 0.13.5 | 2026-09-08 | 1.85.0 | 기본 TLS rustls + aws-lc-rs |
| clap | 4.6.7 | 2026-09-14 | 1.85 | |
| serde | 1.0.229 | 2026-07-18 | 1.56 | |
| serde_json | 1.0.154 | 2026-10-11 | 1.71 | reqwest `json` 기능 경유 기존 의존성 |
| toml | 1.1.8+spec-1.1.0 | 2026-10-09 | 1.85 | |
| tracing | 0.1.44 | 2025-12-18 | 1.65.0 | |
| tracing-subscriber | 0.3.23 | 2026-03-13 | 1.65.0 | |
| keyring | 4.2.0 | 2026-08-29 | 1.88.0 | keyring-core 1.0.1(2026-10-13, MSRV 1.85) 기반 |
| zbus-secret-service-keyring-store | 1.0.1 | 2026-08-15 | 1.88 | keyring `v1` 기능의 Linux 저장소 |
| apple-native-keyring-store | 1.0.2 | 2026-08-06 | 1.85 | |
| windows-native-keyring-store | 1.1.0 | 2026-05-24 | 1.88 | |
| dbus-secret-service-keyring-store | 1.0.1 | 2026-08-15 | 1.88 | libdbus 사용, keyring `cli` 기능 전용 |
| linux-keyutils-keyring-store | 1.0.0 | 2026-04-21 | 1.85 | keyring `cli` 기능 전용 |
| ratatui-textarea | 0.9.3 | 2026-10-05 | 1.86.0 | ratatui 조직 관리 fork |
| tui-textarea | 0.7.0 | 2024-10-22 | 1.56.1 | ratatui ^0.29, crossterm ^0.28 |
| oauth2 | 5.0.0 | 2025-01-21 | 1.65 | reqwest ^0.12, sha2 ^0.10, rand ^0.8 |
| openidconnect | 4.0.1 | 2025-07-06 | 1.65 | oauth2 5 기반 |
| jsonwebtoken | 11.1.0 | 2026-09-16 | 1.88.0 | `aws_lc_rs` 또는 `rust_crypto` 선택 필수 |
| dirs | 7.0.0 | 2026-09-05 | null | 7.0: Windows `preference_dir` 변경 |
| directories | 6.0.0 | 2025-01-12 | null | |
| etcetera | 0.11.0 | 2025-10-28 | 1.87.0 | macOS에서 XDG 경로 선택 가능 |
| open | 5.4.4 | 2026-09-10 | 1.62 | |
| webbrowser | 1.2.4 | 2026-08-05 | 1.85 | |
| sha2 | 0.11.1 | 2026-10-12 | 1.88 | |
| base64 | 0.23.1 | 2026-08-04 | 1.71.0 | reqwest, hyper-util 경유 기존 의존성 |
| getrandom | 0.4.3 | 2026-06-17 | 1.85 | |
| rand | 0.10.3 | 2026-09-20 | 1.85 | |
| anyhow | 1.0.104 | 2026-07-18 | 1.68 | |
| thiserror | 2.0.21 | 2026-09-23 | 1.77 | |
| insta | 1.49.0 | 2026-10-03 | 1.66.0 | |
| assert_cmd | 2.2.2 | 2026-05-11 | 1.85 | |
| tempfile | 3.27.0 | 2026-03-11 | 1.63 | |
| wiremock | 0.6.5 | 2025-08-24 | null | |
| httpmock | 0.9.0 | 2026-10-20 | 1.88.0 | |
| mockito | 1.7.2 | 2026-02-02 | 1.70 | |
| portable-pty | 0.9.0 | 2025-02-11 | null | wezterm 저장소 |
| expectrl | 0.9.0 | 2026-05-11 | null | Windows는 conpty 0.5.1 사용 |
| rexpect | 0.7.1 | 2026-05-14 | 1.85.0 | Unix 전용 |
| eventsource-stream | 0.2.3 | 2022-02-17 | null | nom 7 의존 |
| reqwest-eventsource | 0.6.0 | 2024-03-29 | null | reqwest ^0.12 의존 |
| sse-stream | 0.3.0 | 2026-09-18 | null | bytes, http-body 기반 |
| tiny_http | 0.12.0 | 2022-10-06 | null | |
| hyper | 1.12.0 | 2026-10-06 | 1.63 | reqwest 경유 기존 의존성(client 기능만) |
| axum | 0.8.9 | 2026-04-14 | 1.80 | |
| rustls | 0.23.46 | 2026-10-14 | 1.71 | 0.24.0-dev.2 별도 존재 |
| ring | 0.17.14 | 2025-03-11 | 1.66.0 | |
| aws-lc-rs | 1.18.2 | 2026-10-12 | 1.71.0 | aws-lc-sys 0.46.0 |
| native-tls | 0.2.18 | 2026-02-18 | 1.80 | |
| rustls-platform-verifier | 0.7.1 | 2026-09-24 | 1.85 | reqwest rustls 기능의 인증서 검증기 |

MSRV 종합: ratatui, keyring, sha2, jsonwebtoken 1.88 기준. 프로젝트 `rust-version = "1.88"` 하한 권장. `eventsource-client` 0.18.1은 MSRV 1.97.0으로 로컬 rustc 1.96.0보다 높아 제외 대상(확인(문서)).

## 3. 권장 Cargo.toml

```toml
[package]
name = "howtui"
edition = "2024"
rust-version = "1.88"

[features]
default = ["os-keyring"]
os-keyring = ["dep:keyring"]

[dependencies]
ratatui = "0.30.2"                     # 기본 기능에 crossterm 0.29 백엔드 포함
ratatui-textarea = "0.9.3"             # 기본 기능 crossterm
tokio = { version = "1.54", features = ["rt", "macros", "net", "time", "sync", "io-util", "signal"] }
reqwest = { version = "0.13.5", default-features = false, features = [
  "rustls-no-provider", "http2", "charset", "system-proxy", "json", "stream",
] }
rustls = { version = "0.23", default-features = false, features = ["ring", "std", "tls12", "logging"] }
ring = "0.17"                          # rustls와 동일 버전, ID 토큰 RS256 검증용
clap = { version = "4.6", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1"
thiserror = "2"
sha2 = "0.11"
base64 = "0.23"
getrandom = "0.4"
httparse = "1"                         # hyper 경유 기존 의존성 재사용
etcetera = "0.11"                      # 또는 dirs = "7"
open = "5.4"
futures-util = { version = "0.3", default-features = false }
keyring = { version = "4.2", optional = true }   # 기본 v1 기능

[target.'cfg(unix)'.dependencies]
libc = "0.2"                           # /dev/tty CPR 구현용 poll/select

[target.'cfg(target_os = "macos")'.dependencies]
crossterm = { version = "0.29", features = ["use-dev-tty"] }  # stdin 파이프 입력 지원 시에만

[dev-dependencies]
insta = "1.49"
assert_cmd = "2.2"
tempfile = "3"
portable-pty = "0.9"
wiremock = "0.6"

[profile.release]
strip = true
lto = "fat"
codegen-units = 1
panic = "abort"
```

- `rustls-no-provider` 사용 시 `reqwest::Client` 생성 전에 `rustls::crypto::ring::default_provider().install_default()` 호출 필수. 미호출 시 panic(reqwest 0.13.5 `src/async_impl/client.rs:2499-2511`, 확인(소스)).
- crossterm은 `ratatui::crossterm` 재수출 경로 사용 권장(ratatui-crossterm 0.1.2 `src/lib.rs:87-99`, 확인(소스)). 직접 의존성 추가는 기능 플래그 지정 목적에 한정.
- `panic = "abort"` 적용 시에도 panic hook은 실행. 터미널 복원 hook 등록 필수.
- 위 구성 그대로 x86_64 Linux `cargo check` 성공(`crates-probe/reco-probe/`, 확인(실행)). normal 의존성 중복: getrandom 0.2.17(ring 경유)과 0.4.3, syn 2와 3, hashbrown, toml_datetime, toml_parser. getrandom 중복 제거가 필요하면 `ring::rand::SystemRandom` 사용 후 getrandom 직접 의존성 제거.

## 4. ratatui Viewport::Inline 동작

대상: ratatui 0.30.2, ratatui-core 0.1.2, ratatui-crossterm 0.1.2. 모두 확인(소스).

### 4.1 높이 확보 방식

- `Terminal::with_options`에서 `Viewport::Inline(h)` 선택 시 `compute_inline_size(backend, h, size, 0)` 호출(`ratatui-core-0.1.2/src/terminal/init.rs:122-133`).
- `compute_inline_size` 순서(`src/terminal/inline.rs:390-423`)
  - (a) `backend.get_cursor_position()`로 현재 커서 행 조회(396행)
  - (b) 높이를 터미널 높이로 제한(`size.height.min(height)`, 399행)
  - (c) `backend.append_lines(h - 1)` 호출(405행). CrosstermBackend 구현은 `"\n"` 반복 출력(`ratatui-crossterm-0.1.2/src/lib.rs:330-335`)
  - (d) 커서가 화면 하단에 가까워 줄바꿈이 스크롤을 일으킨 경우 부족분만큼 시작 행을 위로 이동(407-412행)
- 결과: 화면 하단 근처에서 시작하면 기존 터미널 내용이 위로 스크롤되고 뷰포트는 화면 하단에 고정. 뷰포트는 항상 0열 시작, 전체 폭 사용(`src/terminal/viewport.rs:84-91`).

### 4.2 커서 위치 질의

- Inline 생성 시점, Inline `resize` 시점(`src/terminal/resize.rs:30`에서 `compute_inline_size` 재호출), `Terminal::clear()`(`src/terminal/buffers.rs:148`)에서 `get_cursor_position()` 호출.
- CrosstermBackend의 `get_cursor_position`은 writer와 무관한 전역 함수 `crossterm::cursor::position()` 호출(`ratatui-crossterm-0.1.2/src/lib.rs:302-305`).
- crossterm 0.29 Unix 구현(`crossterm-0.29.0/src/cursor/sys/unix.rs:17-56`)
  - raw 모드가 아니면 일시적으로 raw 모드 전환(25-30행)
  - `io::stdout()`에 `\x1B[6n` 기록(34-36행)
  - 내부 이벤트 리더에서 응답을 최대 2000 ms 대기(39행), 실패 시 `"The cursor position could not be read within a normal duration"` 오류(47-51행)
- stdout이 파이프 또는 파일이면 질의가 터미널에 도달하지 않음. 응답 부재로 2초 대기 후 오류, 질의 문자열 4바이트가 stdout 결과에 혼입(5.3절 실행 결과).
- stdin이 tty가 아니어도 응답 수신 경로는 `/dev/tty`로 유지(5.1절).
- 동시성 제약: crossterm 이벤트 리더는 전역 mutex(`crossterm-0.29.0/src/event.rs:156-163, 258-282`). 다른 스레드가 `event::read()`로 대기 중이면 `cursor::position()`의 `try_lock_for(2000ms)` 실패로 오류. ratatui 이슈 #2640(OPEN, 2026-07-08)에 EventStream 사용 중 Inline 생성 또는 `clear()` 시 CPR 경쟁 오류 보고.
- 대응: Inline 생성은 이벤트 루프 시작 전에 수행, 입력 처리는 단일 UI 스레드의 `event::poll(timeout)` + `event::read()` 구조 권장. ratatui PR #2807(OPEN)은 호출자가 커서 위치를 직접 넘기는 `Terminal::with_options_and_cursor_position` 제안 단계.

### 4.3 resize 처리와 알려진 이슈

- `Terminal::draw`와 `try_draw` 내부에서 `autoresize()` 자동 호출. Fullscreen과 Inline만 대상(`src/terminal/resize.rs:64-73`).
- `resize(area)`의 Inline 처리(`src/terminal/resize.rs:23-55`)
  - 이전 뷰포트 안 커서 상대 행을 유지하도록 `compute_inline_size` 재계산(25-37행). 이때 CPR 질의 재발생
  - 가로 폭 축소 시 `next_area.y = 0`과 `ClearType::All`로 화면 전체 삭제(41-45행). 0.30.2 기준 Inline 뷰포트가 화면 최상단으로 이동하고 위쪽 셸 출력이 화면에서 사라지는 동작
  - 이후 `clear_viewport()`로 뷰포트 원점부터 화면 끝까지 삭제(48행)
- 미배포 변경(main 브랜치, 2026-09-04 병합, ratatui-core 버전 표기 0.1.2 유지): PR #2670으로 Inline 뷰포트의 가로 축소 시 전체 화면 삭제 제거. 이슈 #2666(0.30.2에서 연속 draw와 `insert_before` 조합 시 resize마다 뷰포트 복제본이 scrollback에 누적)이 이 PR로 종료(확인(문서), GitHub).
- 기타 관련 이슈: #984(OPEN, Inline 높이 변경 API 요청), PR #1964(OPEN, `set_viewport_height` 제안), #2556(OPEN, `Buffer::resize`의 x, y 무시).
- howtui 영향: 0.30.2 기준 창 폭 축소 시 화면 전체 삭제 발생. Inline 높이는 생성 후 변경 불가. 높이 변경이 필요하면 Terminal 재생성(재생성 시 CPR 질의 재발생) 필요.

### 4.4 insert_before

- 뷰포트 위쪽에 줄 삽입. Inline 이외 뷰포트에서는 무동작(`src/terminal/inline.rs:109-120`).
- 기본 구현: 임시 Buffer에 그린 뒤 화면 스크롤과 직접 출력, 마지막에 `clear()` 호출로 다음 draw에서 뷰포트 전체 재그리기(130-215행). `clear()`가 CPR 질의를 포함하는 점 주의(4.2절).
- `scrolling-regions` 기능 활성화 시 스크롤 영역(DECSTBM)으로 뷰포트 재그리기 없이 삽입(228-292행).
- 열린 PR #2527, #2827: 넓은 글리프(한글 등) 뒤 셀 처리 결함 수정 진행 중. 이슈 #1332(OPEN): 이모지 폭 공백 문제.

### 4.5 종료 시 Inline 영역 정리

- `Terminal::clear()`: Inline에서 뷰포트 원점으로 이동 후 `ClearType::AfterCursor`(crossterm `FromCursorDown`)로 화면 끝까지 삭제, 원래 커서 위치 복원(`src/terminal/buffers.rs:147-174`). CPR 질의 포함.
- CPR 없는 정리 순서(확인(실행), 5.3절 실험 코드)
  - (1) `let area = terminal.get_frame().area();`
  - (2) `terminal.set_cursor_position(area.as_position())`
  - (3) `terminal.backend_mut().clear_region(ClearType::AfterCursor)`
  - (4) `terminal.show_cursor()`
  - (5) `crossterm::terminal::disable_raw_mode()`
  - (6) 결과 명령 문자열을 stdout에 출력
- `Terminal`의 `Drop`은 숨긴 커서 표시만 시도. raw 모드 해제와 화면 정리는 미수행(`ratatui-core-0.1.2/src/terminal.rs:473-487`).

### 4.6 init_with_options, TerminalOptions, restore, panic hook

- `TerminalOptions { viewport: Viewport }` 단일 필드(`ratatui-core-0.1.2/src/terminal.rs:460-471`).
- `ratatui::try_init_with_options`: panic hook 설치, `enable_raw_mode()`, `CrosstermBackend::new(stdout())` 고정(`ratatui-0.30.2/src/init.rs:492-497`). alternate screen 미진입.
- `DefaultTerminal = Terminal<CrosstermBackend<Stdout>>`(`src/init.rs:213`).
- `ratatui::try_restore`: `disable_raw_mode()` 후 `execute!(stdout(), LeaveAlternateScreen)`(`src/init.rs:554-559`). Inline 모드에서도 stdout에 `ESC[?1049l` 출력.
- panic hook: 기존 hook을 감싸 `restore()` 후 기존 hook 호출(`src/init.rs:566-572`). 역시 stdout 출력.
- `ratatui::run_with_options`: main 브랜치 병합(PR #2741, 2026-09-04), 0.30.2 미포함.
- howtui 결론: 위 편의 함수 미사용. `Terminal::with_options(TtyCprBackend::new(tty_writer), TerminalOptions { viewport: Viewport::Inline(h) })` 직접 구성. panic hook은 tty writer 대상 정리(커서 표시, 뷰포트 삭제, raw 모드 해제)만 수행하도록 자체 구현.

## 5. stderr 및 /dev/tty 렌더링

### 5.1 Unix 소스 분석 (crossterm 0.29.0)

| 기능 | 사용 fd | 근거 |
| --- | --- | --- |
| 화면 출력(ratatui draw) | Backend writer(`W: Write`)로 지정한 대상. stderr, `/dev/tty` 파일 모두 가능 | `ratatui-crossterm-0.1.2/src/lib.rs:185-187` |
| `terminal::size()` | `/dev/tty` 읽기 전용 open, 실패 시 stdout fd, 이후 `tput` | `src/terminal/sys/unix.rs:85-105` |
| `enable_raw_mode()` | `tty_fd()`: stdin이 tty면 stdin fd, 아니면 `/dev/tty` open | `src/terminal/sys/unix.rs:125-140`, `src/terminal/sys/file_descriptor.rs:141-154` |
| 이벤트 읽기(mio 기본) | `tty_fd()` 결과를 mio에 등록 | `src/event/source/unix/mio.rs:35-46` |
| 이벤트 읽기(`use-dev-tty`) | `tty_fd()` 결과를 `filedescriptor::poll`로 대기 | `src/event/source/unix/tty.rs:58-80` |
| `cursor::position()` 질의 기록 | 항상 `io::stdout()` | `src/cursor/sys/unix.rs:34-36` |
| `cursor::position()` 응답 수신 | 이벤트 리더(`tty_fd()`) | `src/cursor/sys/unix.rs:39-44` |
| 키보드 향상 플래그 질의 | `/dev/tty` 기록, 실패 시 stdout | `src/terminal/sys/unix.rs:232-241` |

- `libc` 암묵 기능(기본 비활성) 사용 시에도 같은 정책. 0.28부터 기본 백엔드 rustix(CHANGELOG 0.28 항목).
- crossterm 저장소 master(2026-10-20 커밋 기준)도 `cursor/sys/unix.rs`에서 stdout 기록 유지(확인(문서), GitHub API). 수정 PR #1062 "fix(cursor): write CSI 6n query to TTY when stdout is redirected"(OPEN, 2026-05-28). 관련 이슈 #652, #919(OPEN).

### 5.2 use-dev-tty 기능

- 정의: `use-dev-tty = ["filedescriptor", "rustix/process"]`(crossterm 0.29.0 `Cargo.toml`).
- 변경 범위: 이벤트 소스만 mio 구현에서 poll 기반 tty 구현으로 교체(`src/event/source/unix.rs:1-11`, `src/event/sys/unix/waker.rs:1-8`). `size()`, `enable_raw_mode()`, `cursor::position()`의 fd 선택은 무변경.
- macOS 처리: `filedescriptor` 0.8.3은 macOS `poll(2)` 결함 회피용 `select()` 구현 사용(`filedescriptor-0.8.3/src/unix.rs:463-584`). crossterm 이슈 #828(OPEN): macOS에서 stdin 파이프 상태로 `cursor::position()` 호출 시 멈춤. stdin 파이프 입력을 지원하려면 macOS 한정 `use-dev-tty` 활성화 권장(macOS 실행 미확인).
- 알려진 결함: 이슈 #839(OPEN) `use-dev-tty`에서 `event::poll(Duration::ZERO)` 오판. 0이 아닌 timeout 사용 권장.

### 5.3 실행 실험 결과 (Linux x86_64)

실험 방법: `src/bin/inline_probe.rs`(Viewport::Inline(3), 키 입력 1회 후 정리, stdout에 `echo hello` 출력)와 Python `pty` 하네스(`scratchpad/ptyharness/harness.py`, uv 실행, 80x24 PTY, `ESC[6n` 수신 시 `ESC[10;1R` 응답, 화면 출력 확인 후 키 전송). 실제 터미널 에뮬레이터 실행은 미확인.

| 사례 | Backend | 실행 형태 | 결과 |
| --- | --- | --- | --- |
| A | `CrosstermBackend<Stdout>` | stdout tty | 성공 |
| B | `CrosstermBackend<Stderr>` | `howtui > file` | 실패. stderr에 "The cursor position could not be read within a normal duration", 파일 내용 `ESC [ 6 n` 4바이트 |
| F | `CrosstermBackend<Stderr>` | `x=$(howtui)` | 실패. 캡처 값 `[ESC[6n]` |
| C | `TtyCprBackend<Stderr>` | `howtui > file` | 성공. 파일 내용 `echo hello\n`만 존재 |
| D | `TtyCprBackend<Stderr>` | `echo piped \| howtui > file` | 성공. 이벤트 입력은 `/dev/tty` 경유 |
| E | `TtyCprBackend<Stderr>` | `x=$(howtui)` | 성공. 캡처 값 `[echo hello]` |
| G | `TtyCprBackend<Stderr>` | `howtui < /dev/null > file` | 성공 |
| H | `TtyCprBackend<Stderr>` | `setsid -w`, 제어 터미널 없음 | `enable_raw_mode` 실패, `No such device or address (os error 6)` |

- 사례 C의 tty 출력: `ESC[6n`, 줄바꿈 2개(높이 3 확보), `ESC[10;1H`부터 UI 출력, 종료 시 `ESC[10;1H ESC[J ESC[?25h`. 뷰포트가 CPR 응답 행(10행)에 고정된 동작 확인.
- 같은 시나리오의 Rust 종단 테스트(`tests/pty_e2e.rs`, portable-pty 사용) 통과(확인(실행)).

### 5.4 TtyCprBackend 설계

- 구조: `CrosstermBackend<W>`를 감싼 Backend 트레이트 구현. `get_cursor_position`만 교체, 나머지 메서드(`draw`, `append_lines`, `hide_cursor`, `show_cursor`, `set_cursor_position`, `clear`, `clear_region`, `size`, `window_size`, `flush`)는 위임. `scrolling-regions` 기능 사용 시 `scroll_region_up`, `scroll_region_down` 위임 추가(`ratatui-core-0.1.2/src/backend.rs:159-395`).
- Unix CPR: `/dev/tty`를 읽기 쓰기로 열고 `ESC[6n` 기록, 2초 제한 대기 후 `ESC[<row>;<col>R` 파싱. 실험 코드는 `libc::poll` 사용. macOS는 `/dev/tty`에 대한 `poll(2)` 결함(5.2절)으로 `select()` 또는 별도 읽기 스레드 필요(미확인).
- 응답 대기 중 들어온 키 입력 바이트 처리: 실험 코드는 폐기. 실제 구현은 버퍼링 후 재주입 또는 CPR 질의 시점 최소화(생성 시 1회, resize 시) 필요.
- writer 선택: Unix는 `/dev/tty` 쓰기 핸들 우선, stderr가 tty인 경우 stderr 사용 가능. `howtui 2>/dev/null` 실행 시에도 UI 표시를 유지하려면 `/dev/tty` 우선.
- 비대화형 fallback: `/dev/tty` open 실패(사례 H) 시 TUI 미사용, 질의 인자 기반 단일 출력 모드 또는 오류 종료.
- 참고 구현: atuin 셸 통합은 `atuin search ... 3>&1 1>&2 2>&3 3>&-` 형태로 stdout과 stderr를 교환해 TUI를 stdout(tty)에 출력하고 결과를 stderr로 받는 방식("swap stderr and stdout, so that the tui stuff works", atuinsh/atuin `crates/atuin/src/shell/atuin.zsh`, 2026-10-15 커밋 기준). stdout 순수성 계약과 충돌하므로 howtui에는 TtyCprBackend 방식 권장.

### 5.5 Windows (crossterm 0.29.0, crossterm_winapi 0.9.1)

| 기능 | 사용 핸들 | 근거 |
| --- | --- | --- |
| raw 모드 설정과 해제 | `CONIN$` | `src/terminal/sys/windows.rs:31-52` |
| `terminal::size()` | `ScreenBuffer::current()` = `CONOUT$`, `srWindow` 기준 | `src/terminal/sys/windows.rs:55-62`, `crossterm_winapi-0.9.1/src/screen_buffer.rs:35-39`, `src/csbi.rs:49-54` |
| `cursor::position()` | `ScreenBufferCursor::output()` = `CONOUT$`, 콘솔 API 조회, 출력 없음 | `src/cursor/sys/windows.rs:38-45, 121-125` |
| 이벤트 읽기 | `CONIN$` | `src/event/source/windows.rs:28` |
| VT 처리 활성화 | `CONOUT$`에 `ENABLE_VIRTUAL_TERMINAL_PROCESSING` 설정 | `src/ansi_support.rs:16-27` |
| `CONIN$`, `CONOUT$` 생성 | `CreateFileW("CONIN$" / "CONOUT$")` | `crossterm_winapi-0.9.1/src/handle.rs:112-160` |

- Windows는 CPR 질의를 쓰지 않으므로 stdout 리디렉션 상태에서도 위치 조회 가능(확인(소스)). TtyCprBackend의 Windows 분기는 crossterm 위임.
- stderr VT 출력: Microsoft 문서 기준 `STDOUT`, `STDERR` 초기 핸들은 콘솔의 활성 화면 버퍼를 가리키고, `CONOUT$`도 활성 화면 버퍼를 연다. 출력 모드는 화면 버퍼 단위 설정(SetConsoleMode Remarks). 따라서 `CONOUT$`에 설정한 VT 모드가 같은 화면 버퍼를 가리키는 stderr 쓰기에도 적용되는 구조(확인(문서), Windows 실행 미확인).
- 알려진 결함: 이슈 #1095(OPEN, 2026-07-31) conhost의 큰 화면 버퍼(9001행)에서 `cursor::position()`이 창 기준이 아닌 버퍼 절대 행 반환(`src/cursor/sys/windows.rs:22-33` `parse_relative_y`). Windows Terminal(ConPTY) 환경 영향 여부 미확인.
- Windows 키 이벤트는 Press와 Release 모두 발생. ratatui-textarea는 Release 무시(`ratatui-textarea-0.9.3/src/input/crossterm.rs:44`). 자체 키 처리도 `KeyEventKind::Press` 필터 필요.
- writer 선택: stderr가 콘솔이면 stderr, 아니면 `CONOUT$` 파일 open 방식 후보(미확인).

## 6. Textarea 위젯

| 항목 | ratatui-textarea 0.9.3 | tui-textarea 0.7.0 |
| --- | --- | --- |
| 관리 | ratatui 조직, tui-textarea fork(README 9행) | rhysd 개인, 2024-10 이후 미배포 |
| ratatui 호환 | ratatui-core 0.1, ratatui-widgets 0.3, ratatui-crossterm 0.1 의존(cargo tree, 확인(실행)) | ratatui ^0.29, crossterm ^0.28(crates.io 의존성 API) |
| 여러 줄 편집 | 지원. `insert_newline`, `lines()`, `into_lines()`(`src/textarea.rs:1088, 2192, 2208`) | 지원 |
| undo, redo | `undo()`, `redo()`(`src/textarea.rs:1603, 1629`), 기본 50단계, `set_max_histories`(README 361-373행), 기본 단축키 Ctrl+U, Ctrl+R(README 230-231행) | 지원 |
| 키 처리 사용자화 | `input()`(기본 단축키), `input_without_shortcuts()`(`src/textarea.rs:283, 706`), `Input { key, ctrl, alt, shift }` 직접 구성 가능(`src/input/mod.rs:105-114`) | 동일 계열 |
| 넓은 문자 처리 | unicode-width 기반 표시 폭 계산(`src/screen_map.rs:25-33`, `src/wrap.rs:233`), 소프트 줄바꿈 `WrapMode::{None, Word, Glyph, WordOrGlyph}`(`src/wrap.rs:12-21`) | unicode-width 0.2 |
| 화면 커서 좌표 | `screen_cursor()`, `scroll_offset()`(`src/textarea.rs:2229, 2261`) | 없음 |
| 미해결 이슈 | #35(OPEN): 가로 스크롤 모드에서 짝수 폭일 때 CJK 입력 중 커서 소실. #36, #37 수정 PR 진행 중 | crossterm 0.29 대응 이슈 #117(OPEN) |

- 권장: ratatui-textarea 0.9.3.
- 한글 관련 권고
  - (a) `set_wrap_mode(WrapMode::WordOrGlyph)` 사용으로 가로 스크롤 경로(#35) 회피.
  - (b) 위젯 커서는 셀 스타일로 그리는 방식. IME 조합 위치를 맞추려면 `screen_cursor()`와 `scroll_offset()`, 블록 테두리 여백으로 실제 좌표를 계산해 `Frame::set_cursor_position` 호출 필요(설계 근거 확인(소스), IME 실동작 미확인).
  - (c) `Event::Paste`는 `Input` 변환 대상 아님(`src/input/crossterm.rs:6-15`). bracketed paste 활성화 후 `insert_str` 직접 호출.
- 실행 확인: TestBackend 24x6, Inline(2)에서 "명령: ls -la" 렌더링 스냅샷 생성 성공. 넓은 문자 뒤 셀은 "Hidden by multi-width symbols"로 표기(`tests/snapshots/inline_snapshot__inline_viewport_snapshot_with_korean.snap`).

## 7. reqwest TLS, 스트리밍, 프록시

### 7.1 기능 이름과 기본값 (reqwest 0.13.5, crates.io 기능 목록)

- `default = ["default-tls", "charset", "http2", "system-proxy"]`
- `default-tls = ["rustls"]`
- `rustls = ["__rustls-aws-lc-rs", "dep:rustls-platform-verifier", "__rustls"]`: rustls + aws-lc-rs + 플랫폼 인증서 검증기
- `rustls-no-provider`: provider 미지정. `CryptoProvider::get_default()` 미설치 시 panic(`src/async_impl/client.rs:719-722, 2499-2511`)
- `native-tls`, `native-tls-vendored`, `native-tls-no-alpn`, `native-tls-vendored-no-alpn`
- `stream`: `Response::bytes_stream()` 제공(`src/async_impl/response.rs:348-351`). `chunk()`는 기능 없이 사용 가능(294행)
- 인증서 검증: rustls 경로는 `rustls_platform_verifier::Verifier` 사용(`client.rs:759-767`). Linux는 `rustls_native_certs::load_native_certs()`로 시스템 CA 로드(`rustls-platform-verifier-0.7.1/src/verification/others.rs:92`). CA 번들이 없는 최소 이미지에서는 TLS 실패 가능(미확인).

### 7.2 대상별 cargo check 결과

로컬에 대상별 C 교차 컴파일러 없음(`arm-linux-gnueabihf-gcc`, `aarch64-linux-gnu-gcc`, clang, MSVC 도구 부재). Windows와 macOS 대상은 rustup 대상 미설치 상태라 static.rust-lang.org의 `rust-std-1.96.0-<target>.tar.xz`(SHA-256 일치 확인)를 scratchpad 사용자 sysroot에 풀고 `RUSTFLAGS=--sysroot=...`로 검사. 모두 확인(실행).

| 대상 | TLS 없음 + keyring | rustls + aws-lc-rs | rustls + ring | native-tls |
| --- | --- | --- | --- | --- |
| x86_64-unknown-linux-gnu | 성공 | 성공(전체 기능 포함) | 성공 | 성공(시스템 libssl-dev 사용) |
| aarch64-unknown-linux-gnu | 성공 | 실패: `aarch64-linux-gnu-gcc` 없음 | 실패: cc 도구 없음 | 실패: openssl-sys, 교차 pkg-config 미구성 |
| armv7-unknown-linux-gnueabihf | 성공 | 실패: `arm-linux-gnueabihf-gcc` 없음 | 실패: 같은 원인 | 실패: openssl-sys |
| x86_64-pc-windows-msvc | 성공 | 실패: 호스트 cc 사용 오류, NASM 부재 경고 | 실패: `lib.exe` 없음 | 성공 |
| aarch64-pc-windows-msvc | 성공 | 실패: "requires clang-cl" 경고 | 실패: `clang` 없음 | 성공 |
| x86_64-apple-darwin | 성공 | 실패: 호스트 cc의 `-arch` 미지원 | 실패: 같은 원인 | 성공 |
| aarch64-apple-darwin | 성공 | 실패: 같은 원인 | 실패: 같은 원인 | 성공 |

cargo-zigbuild 결과(`uv run --with ziglang cargo zigbuild`, 링크까지 수행한 debug 빌드)

| 대상 | rustls + aws-lc-rs | rustls + ring | native-tls |
| --- | --- | --- | --- |
| armv7-unknown-linux-gnueabihf | 성공 | 성공 | 실패: openssl-sys |
| aarch64-unknown-linux-gnu | 성공 | 성공 | 미실행 |

- ring은 Windows AArch64에서 Clang 필수(`ring-0.17.14/build.rs:562-563` "On Windows AArch64 we currently must use Clang"). x86_64 Windows는 사전 생성 NASM 오브젝트 사용(`build.rs:396-402`, `pregenerated/*-nasm.o`).
- aws-lc-sys 0.46.0 aarch64-pc-windows-msvc 경고: "Windows ARM64 (aarch64-pc-windows-msvc) requires clang-cl"(실행 로그).
- 네이티브 Windows와 macOS 러너에서의 빌드 성공 여부 미확인.

### 7.3 TLS 권고

- 권고: rustls + ring 단일 경로(`rustls-no-provider` + `rustls/ring`).
  - (a) 바이너리 증가량: ring +1.46 MB, aws-lc-rs +3.47 MB(10절)
  - (b) armv7, aarch64 Linux는 cargo-zigbuild 또는 cross 컨테이너로 빌드 가능(확인(실행))
  - (c) ID 토큰 RS256 검증에 같은 ring 재사용 가능(9.1절)
- Windows ARM64 빌드 조건: Clang 필요(ring, aws-lc-rs 공통). 대안은 Windows 한정 `native-tls`(SChannel) 대상별 의존성 선언. Cargo resolver 2 기준 비대상 플랫폼 의존성 기능은 통합되지 않는 구조. 두 TLS 경로의 동작 차이 관리 비용 존재.
- native-tls Linux 비권장: 산출물이 `libssl.so.1.1`, `libcrypto.so.1.1` 동적 링크(`ldd`, 확인(실행)). OpenSSL 3만 제공하는 배포판(Raspberry Pi OS bookworm 등)에서 실행 불가 위험(배포판별 실행 미확인).

### 7.4 SSE 스트리밍

- `bytes_stream()`과 `futures_util::StreamExt::next()`로 청크 수신 확인(크기 측정용 main.rs 컴파일 성공).
- `reqwest-eventsource` 0.6.0: reqwest ^0.12 의존으로 reqwest 0.13과 중복. 비권장.
- `eventsource-stream` 0.2.3: reqwest 비의존 Stream 어댑터, 2022-02 이후 미배포, nom 7 의존.
- `sse-stream` 0.3.0: bytes, futures-util, http-body, http-body-util, pin-project-lite, thiserror 2 의존. 대부분 reqwest 트리에 기존 존재.
- 권고: SSE 파서 직접 구현(줄 단위 `data:`, `event:`, 빈 줄 경계, `\r\n` 처리) 또는 `sse-stream`. 별도 crate 필수 아님.

### 7.5 시스템 프록시

- 환경 변수 `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`(소문자 포함) 기본 적용, `ClientBuilder::no_proxy()`로 해제(reqwest `src/lib.rs:142-162`).
- `system-proxy` 기능: hyper-util `client-proxy-system`(`hyper-util-0.1.21/Cargo.toml:77-80`). macOS는 system-configuration, Windows는 레지스트리(`windows-registry`) 조회(`src/client/proxy/matcher.rs:238-247, 571, 658-696`).
- SOCKS 프록시는 `socks` 기능 필요.

## 8. 자격 증명 저장 (keyring)

### 8.1 구조와 백엔드

- keyring 4.x: API는 keyring-core, 저장소는 별도 crate 구조(README History 절).
- `v1` 기능(기본) 저장소 선택(`keyring-4.2.0/src/v1.rs:109-129`)
  - macOS: `apple_native_keyring_store::keychain::Store`
  - Windows: `windows_native_keyring_store::Store`(Credential Manager)
  - 그 외 Unix: `zbus_secret_service_keyring_store::Store`(Secret Service, zbus 5.19.0, `crypto-rust`)
- 저장소 초기화는 첫 `Entry::new` 또는 `Entry::store_status()` 시점 1회. 실패 시 `Entry::new`가 `Error::NoDefaultStore` 반환(`src/v1.rs:47-53, 67-69, 107`).
- zbus 저장소 초기화는 즉시 D-Bus 연결과 `SecretService::connect(EncryptionType::Dh)` 수행(`zbus-secret-service-keyring-store-1.0.1/src/service.rs:23-27`).
- `cli` 기능은 dbus-secret-service(libdbus `vendored`), linux-keyutils, db-keystore 등 전체 저장소 포함. 애플리케이션 사용 비권장(keyring `src/lib.rs:17-27`).

### 8.2 헤드리스 Linux 실행 결과 (확인(실행))

| 조건 | 결과 | 소요 |
| --- | --- | --- |
| 존재하지 않는 세션 버스 경로 | `store_status: Err(PlatformFailure(Zbus(Connection(... NotFound ...))))`, `Entry::new`는 `NoDefaultStore` | 1 ms 미만 |
| 서비스 디렉터리 없는 사설 dbus-daemon(Secret Service 부재) | `PlatformFailure(... org.freedesktop.DBus.Error.ServiceUnknown ... "The name org.freedesktop.secrets was not provided by any .service files")`, `Entry::new`는 `NoDefaultStore` | 28 ms |
| `DBUS_SESSION_BUS_ADDRESS`, `XDG_RUNTIME_DIR` 미설정 | zbus가 `/run/user/<uid>/bus`로 연결 성공, 실행 중인 gnome-keyring 응답(`NoEntry`) | 28 ms |

- 실험은 존재하지 않는 항목 조회만 수행, 사용자 키링 쓰기 없음.
- 잠긴 컬렉션과 GUI 프롬프터 부재 조합(데스크톱 Raspberry Pi OS에 SSH 접속 등)에서 잠금 해제 요청 대기 가능성 미확인. keyring 호출에 시간 제한(별도 스레드 + 3초 timeout) 적용 권장.

### 8.3 빌드 의존성

- `v1` 기능 Linux 경로: zbus 순수 Rust. libdbus 불필요. armv7, aarch64 `cargo check` 성공(TLS 제외 구성, 확인(실행)).
- macOS, Windows 저장소 포함 `cargo check` 성공(사용자 sysroot 방식, 확인(실행)).
- 의존성 비용: Linux 고유 crate 수 +60(187에서 247), Windows +7(187에서 194), macOS +6(192에서 198). x86_64 Linux small 프로필 기준 바이너리 +1.89 MB(10절).
- zbus 저장소는 zbus 기본 기능(`async-io`, `async-executor`) 사용. tokio와 별개 실행기 포함(cargo tree 기능 그래프, 확인(실행)). 내부 스레드 생성 방식은 미확인.

### 8.4 파일 fallback 설계

- 저장소 선택 순서
  - (1) 환경 변수(`HOWTUI_API_KEY` 등) 존재 시 최우선, 저장 없음
  - (2) 설정 `credential_store = "keyring" | "file" | "auto"`, 기본 `auto`
  - (3) `auto`: keyring `store_status()`를 timeout과 함께 확인, `Ok`면 keyring, 실패면 파일 저장소 사용과 stderr 1회 경고
- 파일 저장소
  - 경로: `$XDG_CONFIG_HOME/howtui/credentials.toml`(etcetera `choose_base_strategy` 기준), Windows `%APPDATA%\howtui\credentials.toml`
  - 디렉터리 0700, 파일 0600. `std::os::unix::fs::OpenOptionsExt::mode(0o600)`로 생성 시점 권한 지정, 임시 파일 기록 후 `rename`으로 원자적 교체
  - 읽기 시 권한 검사: group 또는 other 권한 비트 존재 시 경고 또는 거부
  - Windows: 사용자 프로필 기본 ACL 의존. DPAPI 암호화는 추가 과제(미조사)
  - 저장 항목: access token, refresh token, 만료 시각, 발급자. 평문 저장 사실을 `howtui auth status` 출력에 표시
- 테스트 용도: keyring-core의 `sample` 기능(메모리 저장소) 존재(keyring `cli` 기능 정의, 확인(문서)). 단위 테스트는 저장소 트레이트를 howtui 내부에 정의해 파일 저장소와 메모리 구현으로 검증하는 구조 권장.

## 9. OAuth, OIDC, 브라우저, 디렉터리

### 9.1 OIDC ID 토큰 검증 (RS256, JWKS)

| 후보 | 실험 프로젝트 고유 crate 수 | armv7 `cargo check` | 비고 |
| --- | --- | --- | --- |
| jsonwebtoken 11.1.0 `rust_crypto` | 99 | 성공(순수 Rust) | rsa 0.9.10, p256, p384, ed25519-dalek, hmac 0.12, rand 0.8, sha2 0.10, base64 0.22 추가. sha2 0.11, base64 0.23과 중복 |
| jsonwebtoken 11.1.0 `aws_lc_rs` | 38 | 실패(aws-lc-sys C 컴파일러 부재) | TLS가 aws-lc-rs일 때 추가 암호 코드 없음 |
| openidconnect 4.0.1(기본 기능 해제) | 165 | 성공 | oauth2 5 기반, base64 0.21과 0.22, sha2 0.10, getrandom 0.2 중복 |
| ring 0.17 직접 사용 | 22 | 실패(ring C 컴파일러 부재), zigbuild 성공 | TLS가 ring일 때 추가 비용 없음 |

- jsonwebtoken 11: 백엔드 `aws_lc_rs`, `rust_crypto` 중 최대 1개 선택, 미선택 시 사용자 `CryptoProvider` 필요(README 11-18행). ring 백엔드 없음. `DecodingKey::from_jwk`, `from_rsa_components`, `JwkSet::find`, `Validation::set_issuer`, `set_audience` 제공(`src/decoding.rs:121, 213`, `src/jwk.rs:672-678`, `src/validation.rs:141-147`). ID 토큰 `nonce` 클레임 검증 기능 없음, 별도 확인 필요.
- rsa crate: RUSTSEC-2023-0071(Marvin, 비밀키 연산 타이밍 누출, `patched = []`, 2026-09-12 기준 0.9.10과 0.10.0-rc.18 영향) 존재(확인(문서), rustsec/advisory-db). 공개키 검증만 수행하는 howtui에 직접 영향은 낮으나 `cargo audit`, `cargo deny` 경고 발생 요인.
- 권고: ring 직접 검증(TLS 권고와 일치)
  - (1) JWKS 조회: `https://auth.openai.com/.well-known/jwks.json`을 reqwest로 수신, serde_json 파싱, `kid` 기준 캐시
  - (2) 토큰 분해: `header.payload.signature`, base64 0.23 `URL_SAFE_NO_PAD` 디코딩
  - (3) 헤더 검사: `alg == "RS256"` 고정, `kid` 일치 키 선택, JWK `kty == "RSA"`
  - (4) 서명 검증: `ring::signature::RsaPublicKeyComponents { n, e }.verify(&RSA_PKCS1_2048_8192_SHA256, signing_input, sig)`(`ring-0.17.14/src/rsa/verification.rs:179`). aws-lc-rs도 같은 이름 API 제공(`aws-lc-rs-1.18.2/src/signature.rs:290, 779`, `src/rsa/key.rs:635`)
  - (5) 클레임 검사: `iss` 정확 일치, `aud`(문자열 또는 배열) 포함 여부, `exp`와 `iat`(허용 오차 60초 내외), 저장한 `nonce` 일치
  - TLS를 aws-lc-rs로 정하는 경우 jsonwebtoken `aws_lc_rs` 사용도 동등한 선택지

### 9.2 PKCE, state, nonce 생성

- 최소 구성: getrandom 0.4.3(`getrandom::fill`, `src/lib.rs:87`), sha2 0.11.1(`Sha256::digest`), base64 0.23.1(`engine::general_purpose::URL_SAFE_NO_PAD`, `src/engine/general_purpose/mod.rs:472`).
- code_verifier: 난수 32바이트의 base64url(43자, RFC 7636 허용 길이 43-128자). code_challenge: `base64url(SHA256(verifier))`, `code_challenge_method=S256`.
- state, nonce: 난수 16-32바이트 base64url.
- 버전 정합: base64 0.23.1은 reqwest와 hyper-util 경유 기존 의존성, getrandom 0.4.3은 Linux에서 secret-service 경유 기존 의존성(cargo tree, 확인(실행)). rand 불필요. ring 채택 시 `ring::rand::SystemRandom`(`ring-0.17.14/src/rand.rs:109`)으로 getrandom 대체 가능.
- oauth2 5.0.0: 기본 기능이 reqwest 0.12와 rustls-tls 활성화. 실험 결과 reqwest 0.12.28 추가와 ring, aws-lc-rs 동시 포함 발생(cargo tree, 확인(실행)). 기본 기능 해제 시에도 sha2 0.10, rand 0.8, getrandom 0.2, base64 0.22, thiserror 1, chrono 의존(crates.io 의존성 API). `AsyncHttpClient`는 클로저 구현으로 reqwest 0.13 연결 가능(`oauth2-5.0.0/src/endpoint.rs:23-47`). 중복 비용 대비 이점 부족으로 직접 구현 권장.

### 9.3 루프백 콜백 수신

- 권고: `tokio::net::TcpListener::bind("127.0.0.1:0")` 후 `local_addr().port()`로 redirect URI 구성, 단일 요청 수신.
- 요청 파싱: `httparse` 1.10.1(hyper, hyper-util 경유 기존 의존성), 쿼리 파싱은 `url::form_urlencoded`(url 2.5.8 경유 기존 의존성 form_urlencoded 1.2.2). 추가 crate 없음.
- 처리 범위: `GET /callback?code=...&state=...` 1건, state 불일치 거부, 응답은 짧은 HTML과 `Connection: close`, 시간 제한(예: 5분), 수신 후 listener 종료.
- 비교
  - tiny_http 0.12.0: 2022-10 이후 미배포, 동기 스레드 모델, ascii, chunked_transfer, httpdate, log 추가
  - hyper 서버: reqwest 트리의 hyper는 client 기능만 활성. 서버 사용 시 `server`, `http1` 기능 추가 컴파일
  - axum: 라우터와 tower 계층 추가, 단일 콜백 용도 대비 과도
- 헤드리스 환경 대비: 브라우저 실행 실패 시 URL을 stderr(tty)에 출력. SSH 환경은 원격 루프백 포트 접근 불가로 수동 코드 붙여넣기 또는 디바이스 코드 흐름 제공 필요(제공자 지원 여부 미확인).

### 9.4 브라우저 열기

- open 5.4.4: Linux 의존 crate 4개(is-wsl, libc 등), macOS 1개, Windows 0개(cargo tree). `xdg-open`, `gio`, `gnome-open`, `kde-open` 순차 시도(`open-5.4.4/src/unix.rs:34-46`), 자식 프로세스 stdio를 null로 연결(`src/lib.rs:369-385`). `that()`은 실행기 종료 대기, 차단 회피용 `that_detached()` 제공(`src/lib.rs:181, 285`).
- webbrowser 1.2.4: `$BROWSER` 우선, WSL 감지, GUI 브라우저 출력 억제. 텍스트 브라우저(lynx 등)는 차단 실행(README Consistent Behaviour 절). 의존 crate는 url 포함 46개(Linux), macOS는 objc2 계열 추가.
- 권고: `open::that_detached`. 텍스트 브라우저가 TUI 화면을 점유하는 경로 회피.

### 9.5 설정 디렉터리

- dirs 7.0.0: macOS `config_dir`는 `$HOME/Library/Application Support`(README 표). 7.0 변경점은 Windows `preference_dir`의 RoamingAppData 전환.
- directories 6.0.0: `ProjectDirs` 제공, macOS 경로는 dirs와 같은 Apple 규칙.
- etcetera 0.11.0: `choose_base_strategy()`가 Windows 이외(macOS 포함) XDG 전략 선택(`etcetera-0.11.0/src/base_strategy.rs:45-59`). `choose_native_strategy()`는 macOS Apple 전략.
- 권고: CLI 관례(`$HOME/.config/howtui`)를 macOS에도 적용하려면 etcetera `choose_base_strategy`. Apple 규칙을 따르려면 dirs 7.

## 10. 바이너리 크기와 시작 시간

측정 프로그램: `crates-probe/src/main.rs`(clap, toml, sha2, tracing-subscriber, tokio current_thread, reqwest 스트리밍, ratatui Inline, ratatui-textarea, keyring, open, webbrowser를 모두 실제 호출 경로에 포함). 프로필 small = `strip = true`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, opt-level 3. tiny = small + opt-level "z". 모두 확인(실행).

| 구성 | 대상 | 프로필 | 크기(바이트) |
| --- | --- | --- | --- |
| TLS 없음, keyring 없음 | x86_64 Linux | small | 3,287,560 |
| TLS 없음, keyring | x86_64 Linux | small | 5,179,040 |
| rustls + ring, keyring 없음 | x86_64 Linux | small | 4,752,376 |
| rustls + aws-lc-rs, keyring 없음 | x86_64 Linux | small | 6,754,944 |
| rustls + ring, keyring | x86_64 Linux | small | 6,637,888 |
| rustls + aws-lc-rs, keyring | x86_64 Linux | small | 8,649,000 |
| native-tls(OpenSSL 동적), keyring | x86_64 Linux | small | 5,280,872 |
| rustls + aws-lc-rs, keyring | x86_64 Linux | tiny | 5,438,400 |
| rustls + aws-lc-rs, keyring | x86_64 Linux | release 기본(strip 없음) | 14,567,504 |
| rustls + ring, keyring | armv7 Linux(zigbuild) | small | 5,470,504 |
| rustls + aws-lc-rs, keyring | armv7 Linux(zigbuild) | small | 6,085,532 |

- 고유 crate 수(호스트, normal 의존성): TLS와 keyring 제외 187, rustls + aws-lc-rs 추가 200, keyring 추가 247, 둘 다 260.
- 시작 시간: `--version` 500회 반복 평균 1.3-2.6 ms(구성별 2회 측정). 구성 간 차이는 측정 잡음 범위. 네트워크 미사용 경로 기준 체감 영향 없음.
- 의존성 최소화 권고
  - (a) `oauth2` 기본 기능, `openidconnect`, `reqwest-eventsource` 제외: reqwest 0.12 중복과 ring, aws-lc-rs 동시 포함 유발
  - (b) `jsonwebtoken`의 `rust_crypto` 제외: rsa와 구버전 RustCrypto 계열 중복
  - (c) `tui-textarea` 제외: ratatui 0.29 중복
  - (d) TLS provider 1개로 고정(ring 권장), Linux native-tls 제외
  - (e) keyring은 기본 활성 Cargo 기능으로 분리, 최소 빌드(`--no-default-features`) 경로 제공
  - (f) tokio는 `rt`(current_thread) 사용, `full`과 `rt-multi-thread` 제외
  - (g) `webbrowser` 대신 `open`
  - (h) tracing-subscriber `env-filter`는 regex-automata, matchers 추가. 필요 시 `RUST_LOG` 대신 단순 레벨 설정으로 대체 가능

## 11. 테스트 도구

| 도구 | 용도 | 플랫폼 확인 |
| --- | --- | --- |
| ratatui `TestBackend` | 위젯과 Inline 뷰포트 렌더링 검증. `Display` 구현으로 스냅샷 출력(`ratatui-core-0.1.2/src/backend/test.rs:239`), `assert_buffer_lines`, `assert_scrollback_lines`, `assert_cursor_position`(169-233행) | 순수 Rust |
| insta 1.49.0 | `assert_snapshot!(terminal.backend())` 스냅샷. Inline(2) + 한글 스냅샷 생성 확인 | Linux 실행 확인 |
| assert_cmd 2.2.2 | 비대화형 경로(stdout 순수성, 종료 코드, `--help`) 검증 | Windows 포함 `cargo check --tests` 성공 |
| portable-pty 0.9.0 | PTY 종단 테스트. Unix openpty, Windows ConPTY. CPR 응답 하네스 포함 테스트 통과(`tests/pty_e2e.rs`) | Linux 실행 확인, Windows와 macOS는 check 성공 |
| expectrl 0.9.0 | expect 방식 PTY 테스트, Windows conpty 0.5.1 사용 | Windows check 성공 |
| rexpect 0.7.1 | expect 방식 PTY 테스트 | Windows check 실패(`nix::fcntl` 미해결), Unix 전용 |
| wiremock 0.6.5 | LLM API와 OAuth 엔드포인트 HTTP 모의 서버(async, tokio) | Windows check 성공 |
| httpmock 0.9.0, mockito 1.7.2 | 대체 모의 서버 | 미실행 |
| tempfile 3.27.0 | 설정과 자격 증명 파일 테스트 | Windows check 성공 |

- 권고 구성: 단위 테스트(TestBackend + insta), CLI 테스트(assert_cmd), PTY 종단 테스트(portable-pty, Linux와 macOS CI 우선), HTTP 모의(wiremock).
- PTY 테스트 주의
  - (a) 하네스가 터미널 역할로 `ESC[6n`에 `ESC[<row>;<col>R` 응답 필요(5.3절)
  - (b) portable-pty Windows 구현은 `PSEUDOCONSOLE_INHERIT_CURSOR` 플래그 사용(`portable-pty-0.9.0/src/win/psuedocon.rs:27, 87-89`). Microsoft 문서 기준 이 플래그 사용 시 호출 측이 커서 상태 요청에 비동기 응답 필요, 미응답 시 정지 가능(CreatePseudoConsole Remarks). Windows PTY 테스트 하네스에도 CPR 응답 스레드 필요(Windows 실행 미확인)
  - (c) PTY 크기 미설정 시 0x0 크기 보고로 뷰포트 높이 0 계산 발생(실험 중 확인). `PtySize` 명시 필요

## 12. 대상별 cargo check 종합

| 대상 | 본 의존성(TLS 없음 + keyring) | dev 의존성(`--tests`) | 비고 |
| --- | --- | --- | --- |
| x86_64-unknown-linux-gnu | 성공 | 성공 | 전체 TLS 3종 성공 |
| aarch64-unknown-linux-gnu | 성공 | 성공 | TLS C 의존성은 zigbuild로 성공 |
| armv7-unknown-linux-gnueabihf | 성공 | 성공 | 같음 |
| x86_64-pc-windows-msvc | 성공 | rexpect 제외 시 성공 | native-tls 성공, ring과 aws-lc-rs는 MSVC 도구 필요 |
| aarch64-pc-windows-msvc | 성공 | rexpect 제외 시 성공 | ring과 aws-lc-rs 모두 Clang 필요 |
| x86_64-apple-darwin | 성공 | 미실행 | native-tls 성공, ring과 aws-lc-rs는 Apple 교차 도구 필요 |
| aarch64-apple-darwin | 성공 | 성공 | 같음 |

- Windows와 macOS 결과는 사용자 sysroot 기반 `cargo check`. 링크 단계와 실제 실행 미확인.
- 로그 위치: `crates-probe/logs/check_<target>_<features>.log`, `crates-probe/logs/zig_<target>_<features>.log`.

## 13. 미확인 항목

- (1) macOS, Windows 실제 터미널(Terminal.app, iTerm2, Windows Terminal, conhost)에서 Inline 뷰포트와 TtyCprBackend 동작
- (2) macOS `/dev/tty` CPR 구현에서 `select()` 사용 시 정상 동작 여부
- (3) Windows에서 stdout 리디렉션 상태의 stderr VT 출력 실동작
- (4) 잠긴 Secret Service 컬렉션과 GUI 프롬프터 부재 조합의 keyring 대기 동작
- (5) 네이티브 Windows ARM64 러너의 ring 빌드용 Clang 제공 여부
- (6) Raspberry Pi 실기기에서 aws-lc-rs, ring 실행 성능과 정상 동작
- (7) 한글 IME 조합 위치와 `Frame::set_cursor_position` 연동 결과
- (8) ChatGPT 로그인 제공자의 디바이스 코드 흐름 지원 여부

## 14. 출처

- crates.io API: `https://crates.io/api/v1/crates/<name>` 및 `/<version>/dependencies` (ratatui, ratatui-core, ratatui-widgets, ratatui-crossterm, crossterm, tokio, reqwest, clap, serde, serde_json, toml, tracing, tracing-subscriber, keyring, keyring-core, 저장소 crate 5종, ratatui-textarea, tui-textarea, oauth2, openidconnect, jsonwebtoken, rsa, dirs, directories, etcetera, open, webbrowser, sha2, base64, getrandom, rand, anyhow, thiserror, insta, assert_cmd, tempfile, wiremock, httpmock, mockito, portable-pty, expectrl, rexpect, eventsource-stream, reqwest-eventsource, sse-stream, eventsource-client, tiny_http, hyper, hyper-util, axum, rustls, ring, aws-lc-rs, native-tls, rustls-platform-verifier, zbus, secret-service), 2026-10-22 조회
- 소스(`$HOME/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`): ratatui-0.30.2, ratatui-core-0.1.2, ratatui-crossterm-0.1.2, crossterm-0.29.0, crossterm_winapi-0.9.1, filedescriptor-0.8.3, ratatui-textarea-0.9.3, reqwest-0.13.5, hyper-util-0.1.21, rustls-platform-verifier-0.7.1, keyring-4.2.0, zbus-secret-service-keyring-store-1.0.1, oauth2-5.0.0, jsonwebtoken-11.1.0, ring-0.17.14, aws-lc-rs-1.18.2, base64-0.23.1, getrandom-0.4.3, open-5.4.4, webbrowser-1.2.4, dirs-7.0.0, etcetera-0.11.0, portable-pty-0.9.0, expectrl-0.9.0
- docs.rs: https://docs.rs/ratatui/0.30.2/ratatui/enum.Viewport.html, https://docs.rs/ratatui/0.30.2/ratatui/fn.init_with_options.html, https://docs.rs/crossterm/0.29.0/crossterm/cursor/fn.position.html, https://docs.rs/reqwest/0.13.5/reqwest/, https://docs.rs/keyring/4.2.0/keyring/, https://docs.rs/ratatui-textarea/0.9.3/ratatui_textarea/, https://docs.rs/jsonwebtoken/11.1.0/jsonwebtoken/
- ratatui GitHub: 이슈 #984, #1332, #2086, #2500, #2556, #2640, #2666, PR #1964, #2355, #2485, #2527, #2670, #2694, #2723, #2731, #2741, #2807, #2827, 릴리스 노트 ratatui-v0.30.2(https://github.com/ratatui/ratatui/releases/tag/ratatui-v0.30.2), main 브랜치 `ratatui-core/src/terminal/resize.rs`
- crossterm GitHub: 이슈 #652, #672, #828, #839, #919, #963, #1095, PR #1062, master 브랜치 `src/cursor/sys/unix.rs`
- ratatui-textarea GitHub: 이슈 #35, PR #10, #23, #30, #36, #37. tui-textarea GitHub: 이슈 #105, #117
- atuin 셸 통합: https://github.com/atuinsh/atuin/blob/main/crates/atuin/src/shell/atuin.zsh
- RustSec: https://github.com/rustsec/advisory-db/blob/main/crates/rsa/RUSTSEC-2023-0071.md
- Microsoft 문서: https://learn.microsoft.com/en-us/windows/console/console-handles, https://learn.microsoft.com/en-us/windows/console/setconsolemode, https://learn.microsoft.com/en-us/windows/console/createpseudoconsole
- Rust 표준 라이브러리 배포물: https://static.rust-lang.org/dist/rust-std-1.96.0-{x86_64-pc-windows-msvc, aarch64-pc-windows-msvc, x86_64-apple-darwin, aarch64-apple-darwin}.tar.xz (SHA-256 일치 확인)
- 실험 코드와 로그: `<로컬 임시 디렉터리>/crates-probe/` (`src/bin/inline_probe.rs`, `src/bin/keyring_probe.rs`, `tests/inline_snapshot.rs`, `tests/pty_e2e.rs`, `jwt-probe/`, `logs/`), `<로컬 임시 디렉터리>/ptyharness/harness.py`
