# SRS: howtui 소프트웨어 요구사항 명세

- 문서 ID: SRS
- 버전: 0.1 (Phase 0 초안)
- 작성일: 2026-10-22
- 상태: 사용자 검토 대기
- 상위 문서: [PRD](PRD.md)
- 설계 결정 기록: [adr/](adr/)
- 근거 자료: [research/](research/)

## 0. 표기 규칙

- 설계 요구사항 ID: `SRS-<영역>-<번호>`
- 표 열: ID, 설계 요구사항, 관련 PRD, 구현 위치 (계획 경로)
- 코드 블록의 Rust 정의는 인터페이스 계약 초안. 구현 시 이름과 시그니처 변경은 본 문서 갱신 후 수행.
- 검증 상태 표기: 실측(Phase 0 실험), 소스 확인, 문서 확인, 미검증

---

## 1. 시스템 개요

howtui는 단일 실행 파일. 기존 터미널 위에서 Inline TUI를 표시하고, 원격 LLM Provider에 명령어 생성을 요청한 뒤, 사용자가 확정한 명령어를 stdout 또는 쉘 어댑터 전달 경로로 반환.

외부 경계:

| 경계 | 대상 | 방향 |
| --- | --- | --- |
| 제어 터미널 | Unix `/dev/tty`, Windows `CONIN$`/`CONOUT$` | 입력 이벤트 수신, 화면 출력, 커서 질의 |
| stdout | 호출자 (사용자 터미널, 파이프, 쉘 어댑터) | 확정 명령어 1회 출력 |
| stderr | 호출자 | TUI 종료 후 오류 메시지, 비대화형 모드 설명 |
| LLM Provider | `https://api.openai.com/v1` (P1) | HTTPS 요청 |
| 인증 서버 | `https://auth.openai.com` | OAuth, JWKS |
| 브라우저 | 시스템 기본 브라우저 | 인가 URL 열기 |
| 파일 시스템 | 설정, 상태, 자격증명, 로그 파일 | 읽기와 쓰기 (12절 경로) |
| OS 보안 저장소 | Keychain, Credential Manager, Secret Service | 자격증명 저장 |

## 2. 전체 아키텍처

```text
            ┌──────────────────────── howtui 프로세스 ────────────────────────┐
 argv ────▶ │ cli (clap)                                                     │
            │   │                                                            │
            │   ├─ login/logout/auth ─▶ auth ──────────────▶ 인증 서버, 브라우저 │
            │   ├─ doctor ───────────▶ doctor ─(읽기 전용 점검)               │
            │   ├─ init ─────────────▶ shell::init (스크립트 출력, P2)        │
            │   └─ (기본) ──▶ runtime                                         │
            │                 │  ┌──────────── app (순수 코어) ───────────┐    │
 제어 터미널 ◀─▶ platform::tty ─▶ ui::keymap ─Action─▶ update(State) ─Effect─┐ │
            │        ▲        │  └────────────────────────────────────────┘ │ │
            │        │        │                 ▲ Action(결과)              │ │
            │   ui::inline / ui::fullscreen      │                          ▼ │
            │   (render(&State))          runtime ── llm::Provider ──▶ LLM API │
            │                                    └─ safety::analyze          │
            │                                                                │
 stdout ◀── │ output (확정 명령어 1회)                                        │
            └────────────────────────────────────────────────────────────────┘
```

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-ARC-001 | 단방향 상태 갱신 구조: keymap → update → Effect → runtime → Action 재주입 (ADR-0004) | PRD-UI-013, PRD-FS-002 | `src/app/`, `src/runtime.rs` |
| SRS-ARC-002 | `app` 코어는 터미널, 네트워크, 시계, 파일 시스템에 직접 접근 금지 | PRD-FS-002 | `src/app/` |
| SRS-ARC-003 | 화면 모드는 렌더러와 터미널 세션 설정으로만 구분 | PRD-FS-001, PRD-FS-002 | `src/ui/` |
| SRS-ARC-004 | OS별 차이는 `platform` 모듈과 쉘 어댑터 스크립트로 격리 | PRD-PLAT-001 | `src/platform/`, `src/shell/` |

## 3. crate 및 모듈 구조

### 3.1 모듈 구성

단일 바이너리 crate (ADR-0001).

| 모듈 | 책임 | 의존 허용 |
| --- | --- | --- |
| `main.rs` | 진입점, 런타임 생성, 종료 코드 매핑 | 전체 |
| `cli` | clap 정의, 하위 명령 분기 | 전체 |
| `runtime` | 이벤트 루프, Effect 실행, 작업 생성과 취소 | `app`, `ui`, `llm`, `auth`, `safety`, `platform`, `output` |
| `app` | `State`, `Action`, `Effect`, `update`, 편집 버퍼 | `safety`, `llm::types` |
| `ui` | keymap, inline 렌더러, fullscreen 렌더러(P3), 공통 위젯 | `app` |
| `llm` | Provider trait, 표준 타입, 프롬프트 구성, 응답 파싱, ChatGPT Provider, SSE, Mock(기능 플래그) | `auth`(토큰 조회 trait) |
| `auth` | OAuth, ID 토큰 검증, 자격증명 저장, 갱신, 잠금 | `config`, `platform` |
| `shell` | 쉘 판정, init 스크립트 생성(P2) | 없음 |
| `output` | 확정 명령어 전달 (stdout, 출력 파일) | 없음 |
| `config` | 설정 로드, 검증, 경로 | `platform` |
| `safety` | 위험 명령어 규칙 판정 | 없음 |
| `doctor` | 점검 항목 실행, 텍스트와 JSON 출력, 마스킹 | `config`, `auth`, `platform`, `shell` |
| `platform` | 제어 터미널 열기, fd 전환, 시그널, 경로, 파일 권한 | 없음 |

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-MOD-001 | 3.1절 모듈 구성과 의존 허용 범위 준수 | PRD-FS-002 | `src/` |
| SRS-MOD-002 | `app` 모듈에서 `crossterm`, `ratatui`, `reqwest`, `tokio::net`, `std::fs` 사용 금지 (리뷰 점검 항목, CI grep 검사) | PRD-FS-002 | `src/app/` |
| SRS-MOD-003 | Mock Provider는 cargo 기능 `mock-provider`로만 컴파일, 릴리스 빌드 제외 | PRD-LLM-012 | `src/llm/mock.rs` |

### 3.2 주요 의존성

조사 근거: [research/rust-crates.md](research/rust-crates.md). 버전은 Phase 1 착수 시 `Cargo.lock`으로 고정.

| 영역 | crate | 용도 | 비고 |
| --- | --- | --- | --- |
| TUI | ratatui 0.30 | Inline, Fullscreen 뷰포트, 위젯 | SPIKE-01 실측 0.30.2 |
| 터미널 | crossterm 0.29 | raw mode, 이벤트, bracketed paste | SPIKE-01 실측 0.29.0, 3.3절 조사 반영 |
| 비동기 | tokio 1.x | 런타임, 채널, 시그널, 타이머 | SPIKE-01 실측 1.54.0 |
| HTTP | reqwest | HTTPS, 스트리밍 | TLS 백엔드는 3.3절 |
| CLI | clap 4 (derive) | 인자 파싱 | |
| 직렬화 | serde, serde_json, toml | 설정, 상태, API | |
| 진단 | tracing, tracing-subscriber | 선택적 파일 로그 | 기본 비활성 |

3.3절에 조사 결과 기반 확정안 기록.

### 3.3 의존성 확정안

근거: [research/rust-crates.md](research/rust-crates.md) (2026-10-22 crates.io 최신 안정 버전, 교차 빌드 실행 결과 포함). `rust-version = "1.89"` (ratatui 0.30, keyring 4.2 MSRV 1.88과 `std::fs::File::lock` 안정화 1.89 중 높은 값), 개발 툴체인은 구현 착수 시점 최신 안정 버전 1.99.1 고정 (`rust-toolchain.toml`).

| crate | 버전 | 기능 플래그 | 용도 | 선정 근거 |
| --- | --- | --- | --- | --- |
| ratatui | 0.30.2 | 기본 | TUI | Inline 뷰포트, TestBackend |
| crossterm | 0.29.0 | 기본 + `bracketed-paste`, macOS만 `use-dev-tty` 추가 | 터미널 | 최신 릴리스. Linux는 stdin 비TTY 시 `/dev/tty` 자동 사용(SPIKE-01 stdin_pipe). macOS kqueue는 `/dev/tty` 폴링 미지원이라 select 기반 이벤트 소스 필요 (CI PTY 실측, 2026-10-22) |
| tokio | 1.54 | `rt-multi-thread`, `macros`, `net`, `time`, `sync`, `io-util`, `signal` | 네트워크 작업, 콜백 리스너, 시그널 | |
| reqwest | 0.13.5 | `default-features = false`, `rustls-no-provider`, `http2`, `charset`, `system-proxy`, `json`, `stream` | HTTPS, 스트리밍 | native-tls는 armv7, aarch64 교차 빌드 실패 (openssl-sys) |
| rustls | 0.23 | `default-features = false`, `ring`, `std`, `tls12`, `logging` | TLS | ring 백엔드가 aws-lc-rs 대비 바이너리 증가분 약 1.46 MB 대 3.47 MB (x86_64 strip, fat LTO 실측) |
| ring | 0.17 | 기본 | TLS 암호 백엔드, ID 토큰 RS256 검증 | 추가 crate 없이 RSA PKCS#1 v1.5 SHA-256 검증 |
| keyring | 4.2 | 기본 | OS 보안 저장소 | macOS Keychain, Windows Credential Manager, Linux Secret Service(zbus, 순수 Rust, libdbus 불필요). armv7, aarch64, Windows, macOS `cargo check` 통과 |
| clap | 4.6 | `derive` | CLI | |
| serde, serde_json | 1.0 | `derive` | 직렬화 | |
| toml | 1.1 | 기본 | 설정 | |
| sha2 | 0.11 | 기본 | PKCE challenge | |
| base64 | 0.23 | 기본 | base64url (`URL_SAFE_NO_PAD`) | |
| getrandom | 0.4 | 기본 | state, nonce, verifier, host ID 난수 | rand 불필요 |
| httparse | 1 | 기본 | 콜백 HTTP 요청 파싱 | hyper 경유 기존 의존 트리 포함 |
| url | 2 | 기본 | URL 구성, 질의 파싱 | reqwest 경유 기존 의존 트리 포함 |
| open | 5.4 | 기본 | 브라우저 실행 (`that_detached`) | webbrowser는 텍스트 브라우저에서 대기 발생 |
| futures-util | 0.3 | `std` | SSE 바이트 스트림 순회 | reqwest 의존 트리 포함 |
| tokio-util | 0.7 | 기본 | 취소 토큰 (`CancellationToken`) | h2 의존 트리 포함 |
| unicode-segmentation, unicode-width | ratatui 의존 버전 | 기본 | 편집 버퍼 grapheme과 표시 폭 | ratatui 의존 트리 포함 |
| tracing, tracing-subscriber | 0.1.44, 0.3.23 | `fmt` | 선택적 파일 로그 | |
| thiserror | 2.0 | 기본 | 오류 타입 | |
| libc | 0.2 | 기본 (`cfg(unix)`) | `dup`, `dup2` | |
| windows-sys | crossterm 의존 버전 | `Win32_System_Console` (`cfg(windows)`) | `CONOUT$` 쓰기, 콘솔 모드 | 6.1절 Windows 출력 경로 |

개발 의존성:

| crate | 용도 |
| --- | --- |
| assert_cmd, tempfile | 바이너리 통합 테스트 |
| wiremock | Mock HTTP 서버 (LLM, 인증, revoke) |
| portable-pty | Windows ConPTY E2E 테스트 (`cfg(windows)` 개발 의존성) |

미채택:

| crate | 사유 |
| --- | --- |
| tui-textarea 0.7 | ratatui 0.29, crossterm 0.28 의존 |
| ratatui-textarea 0.9.3 | 검토 결과 ADR-0015 (자체 편집 버퍼) |
| oauth2 5.0 | reqwest 0.12 중복 의존, sha2 0.10, rand 0.8 추가 |
| jsonwebtoken 11.1, openidconnect 4.0 | ring 백엔드 부재, rsa 0.9 권고(RUSTSEC-2023-0071), 의존 crate 99-165개 |
| reqwest-eventsource | reqwest 0.12 의존 |
| native-tls | 교차 빌드 실패, Linux OpenSSL 버전 의존 |
| rexpect | Windows 빌드 불가 |

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-MOD-004 | 3.3절 의존성 목록과 기능 플래그 고정, 추가 시 본 표 갱신 후 도입 | PRD-REL-001 | `Cargo.toml` |
| SRS-MOD-005 | TLS: rustls + ring. 프로세스 시작 시 `rustls::crypto::ring::default_provider().install_default()` 호출 후 HTTP 클라이언트 생성 (미호출 시 reqwest panic, reqwest `client.rs:2499-2511`) | PRD-AUTH-001, PRD-LLM-001 | `src/llm/http.rs` |
| SRS-MOD-006 | 자격증명 저장소 우선순위: (1) OS 보안 저장소(keyring, 응답 제한 시간 2초) (2) 데이터 디렉터리 `credentials.json`(0700 디렉터리, 0600 파일, 원자적 이름 변경). (2) 사용 시 1회 고지 | PRD-AUTH-008 | `src/auth/store.rs` |
| SRS-MOD-007 | Linux 빌드에 C 라이브러리 동적 의존 추가 금지 (glibc 외). `ldd` 또는 ELF NEEDED 검사로 확인 | PRD-PLAT-002 | CI |
| SRS-MOD-008 | `ratatui::init`, `init_with_options`, `restore`, ratatui 기본 panic hook 사용 금지 (stdout 고정 출력, `restore`의 `ESC[?1049l` stdout 기록) | PRD-OUT-002 | `src/ui/terminal.rs` |

### 3.4 CLI 정의

```text
howtui [OPTIONS] [QUESTION]...          기본: Inline TUI (QUESTION 지정 시 즉시 생성)
  --no-tui                              비대화형 1회 생성 (QUESTION 필수)
  --shell <bash|zsh|powershell>         대상 쉘 지정
  --height <N>                          Inline 높이 (3-40)
  --mode <inline|fullscreen>            화면 모드 (P3)
  --widget                              쉘 어댑터 전용 위젯 모드 (P2, 도움말 숨김)
  -V, --version / -h, --help
howtui login [--no-browser] [--port <PORT>]
howtui logout [--forget]
howtui auth status
howtui doctor [--json]
howtui init <bash|zsh|powershell> [--key <KEY>] [--no-bind]   (P2)
```

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-CLI-001 | 3.4절 명령 구조 (clap derive) | PRD-CLI-001, PRD-CLI-003, PRD-CLI-004 | `src/cli/mod.rs` |
| SRS-CLI-002 | `QUESTION` 여러 인자는 공백 1개로 결합 | PRD-CLI-001 | `src/cli/mod.rs` |
| SRS-CLI-003 | 옵션 충돌은 사용법 오류(2): `--no-tui`와 `--widget` 동시 지정, `--no-tui` 질문 누락, `--height` 범위 밖 | PRD-CLI-005, PRD-CLI-006, PRD-CLI-010 | `src/cli/mod.rs` |
| SRS-CLI-004 | `--help` 본문: 하위 명령, 옵션, 종료 코드 요약, 전송 데이터 범위 1줄, 위험 경고 한계 1줄 | PRD-CLI-011, PRD-PRIV-003, PRD-SAFE-004 | `src/cli/mod.rs` |
| SRS-CLI-005 | `--version` 출력 `howtui <CARGO_PKG_VERSION>` | PRD-CLI-002 | `src/cli/mod.rs` |
| SRS-CLI-006 | `--height` 우선순위: CLI > 설정 > 기본값 10 | PRD-CLI-006 | `src/cli/mod.rs` |
| SRS-CLI-007 | `--mode` 우선순위: CLI > 설정 > `inline` (P3) | PRD-CLI-009 | `src/cli/mod.rs` |

## 4. 데이터 모델

### 4.1 애플리케이션 상태

```rust
pub struct State {
    pub phase: Phase,
    pub focus: Focus,
    pub question: TextBuffer,          // 질문 입력 영역
    pub command: TextBuffer,           // 명령어 편집 영역
    pub explanation: Explanation,      // 설명 본문과 갱신 필요 여부
    pub findings: Vec<safety::Finding>,// 위험 판정 결과
    pub conversation: Conversation,    // 세션 내 이력 (디스크 미저장)
    pub pending: Option<Pending>,      // 진행 중 요청 (최대 1개)
    pub status: StatusLine,            // 상태 메시지, 오류 메시지
    pub env: EnvContext,               // OS, 아키텍처, 쉘
    pub provider_label: ProviderLabel, // 모델명, 플랜 사용 표시
    pub viewport: ViewportInfo,        // 크기, 최소 크기 미만 여부
    next_request_id: u64,
}

pub enum Phase { LoginRequired, LoggingIn, Ready, Editing, Requesting, ConfirmDanger, Accepted, Cancelled }
pub enum Focus { Question, Command }
pub struct Pending { pub id: RequestId, pub op: Operation, pub return_phase: Phase }
pub struct Explanation { pub text: String, pub stale: bool }
pub struct Conversation { pub turns: Vec<Turn> }
pub struct Turn { pub op: Operation, pub question: Option<String>, pub command_sent: Option<String>, pub response: LlmResponse }
```

### 4.2 Action과 Effect

```rust
pub enum Action {
    Key(KeyAction), Paste(String), Resize { width: u16, height: u16 }, Tick,
    LlmDone { id: RequestId, result: Result<LlmResponse, LlmError> },
    LoginDone(Result<AccountSummary, AuthError>), ProviderReady(ProviderLabel),
}
pub enum Effect {
    StartRequest(LlmRequest), CancelRequest(RequestId),
    StartLogin, CancelLogin, Exit(ExitOutcome),
}
pub enum ExitOutcome { Accepted(String), Cancelled }
```

### 4.3 편집 버퍼

```rust
pub struct TextBuffer {
    lines: Vec<String>,          // LF 기준 줄 분리, 내부 저장은 UTF-8
    cursor: (usize, usize),      // (줄, grapheme 인덱스)
    undo: Vec<Snapshot>, redo: Vec<Snapshot>,
    read_only: bool,
}
```

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-DATA-001 | 4.1절 `State` 구조. 진행 요청 최대 1개 | PRD-UI-013, PRD-LLM-004 | `src/app/state.rs` |
| SRS-DATA-002 | 편집 버퍼 커서는 grapheme cluster 단위, 화면 열 계산은 Unicode 표시 폭 기준 | PRD-EDIT-005 | `src/app/editor.rs` |
| SRS-DATA-003 | Undo 스냅샷 단위: 연속 문자 입력 묶음 1단계, 삭제와 붙여넣기 각 1단계, AI 응답 반영 1단계. 이력 최대 200단계 | PRD-EDIT-003, PRD-EDIT-006 | `src/app/editor.rs` |
| SRS-DATA-004 | 요청 중 명령어 버퍼 `read_only = true` (Explain 포함) | PRD-EDIT-006 | `src/app/update.rs` |
| SRS-DATA-005 | 대화 이력은 메모리 전용. 요청 시 최근 6개 Turn까지 컨텍스트 포함 (초과분 제외) | PRD-LLM-007 | `src/app/state.rs`, `src/llm/prompt.rs` |
| SRS-DATA-006 | 줄바꿈 정규화: 붙여넣기와 응답의 CRLF는 LF로 변환하여 저장, 출력도 LF | PRD-OUT-005 | `src/app/editor.rs` |

## 5. LLM Provider 인터페이스

### 5.1 공통 타입

```rust
pub enum Operation { Generate, Revise, Explain }

pub struct EnvContext {
    pub os: OsKind,                  // Linux, MacOs, Windows
    pub arch: &'static str,          // std::env::consts::ARCH
    pub shell: ShellKind,            // Bash, Zsh, PowerShell
    pub shell_version: Option<String>,
}

pub struct LlmRequest {
    pub id: RequestId,
    pub op: Operation,
    pub env: EnvContext,
    pub question: Option<String>,    // Generate, Revise
    pub command: Option<String>,     // Revise, Explain: 요청 시점 편집 내용
    pub history: Vec<Turn>,
}

pub struct LlmResponse {
    pub command: Option<String>,     // Explain은 None
    pub explanation: String,
    pub notes: Vec<String>,
    pub model: String,
}

pub enum LlmError {
    AuthRequired, AuthFailed { detail: String }, NotEligible, PermissionConfig { code: String },
    RegionOrPolicy { detail: String }, UsageLimit { manage_url: &'static str },
    Unsupported { param: Option<String> }, ModelNotFound, Transient { status: Option<u16> },
    Network { detail: String }, InvalidResponse { detail: String }, Timeout, Cancelled,
}

pub type LlmFuture<'a> = Pin<Box<dyn Future<Output = Result<LlmResponse, LlmError>> + Send + 'a>>;

// dyn 호환을 위해 표준 라이브러리 Box<dyn Future> 반환 (async-trait crate 미사용)
pub trait Provider: Send + Sync {
    fn label(&self) -> ProviderLabel;
    fn complete(&self, req: LlmRequest, cancel: CancelToken) -> LlmFuture<'_>;
}
```

### 5.2 프롬프트 구성

- `instructions`(Responses API) 또는 Provider별 시스템 지시에 다음 고정 규칙 포함:
  - (a) 대상 OS, 아키텍처, 쉘 종류와 버전
  - (b) 출력은 JSON 객체 1개 `{"command": string|null, "explanation": string, "notes": [string]}`
  - (c) `command`는 대상 쉘에서 그대로 입력 가능한 문자열, 마크다운 코드 블록 금지
  - (d) 설명 언어는 사용자 요청 언어
  - (e) 명령어 실행 주장 금지, 비밀값 생성 금지
- `input`은 이력 Turn과 현재 요청을 user, assistant 메시지 순으로 구성. Revise 요청에는 "사용자가 편집한 현재 명령어" 블록 포함.
- 지시문 원문은 `src/llm/prompt.rs` 상수로 관리, 변경 시 Mock 기반 스냅샷 테스트 갱신.

### 5.3 응답 파싱

4단계 폴백 파이프라인:

1. 응답 본문 전체를 JSON으로 파싱
2. 실패 시 첫 ```json 코드 블록 또는 첫 `{` - 마지막 `}` 구간 JSON 파싱
3. 실패 시 첫 코드 블록을 명령어, 나머지 본문을 설명으로 분리 (Generate, Revise 한정)
4. 실패 시 `LlmError::InvalidResponse`

- 공통 후처리: 명령어 앞뒤 공백 줄 제거, CRLF를 LF로 변환, 빈 명령어는 형식 오류 처리.

### 5.4 ChatGPT Provider (P1)

근거: [research/chatgpt-oauth.md](research/chatgpt-oauth.md) 5절, 8절.

| 항목 | 값 |
| --- | --- |
| 모델 목록 | `GET https://api.openai.com/v1/models`, 응답 `models[]`의 `visibility == "list"` 항목, 서버 순서 유지 |
| 추론 | `POST https://api.openai.com/v1/responses` |
| 필수 필드 | `model`, `instructions`, `input`(배열), `store: false`, `stream: true` |
| 미포함 필드 | `temperature`, `top_p`, `max_output_tokens`, `previous_response_id`, `metadata`, `user`, `truncation` 외 preview 미지원 목록 전체 |
| 구조화 출력 | `text.format` JSON schema 시도. `subscription_sharing_unsupported_capability`(`error.param`이 `text` 계열) 수신 시 지시문 방식 전환 후 상태 파일에 결과 기록 |
| 성공 판정 | SSE `response.completed` 수신 시에만 성공 |
| 실패 이벤트 | `response.failed`, `response.incomplete`, `error` |
| 요청 ID | 응답 헤더 `x-request-id` 보존 (오류 메시지와 로그에 기록, 토큰 제외) |

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-LLM-001 | 5.1절 Provider trait와 표준 타입 | PRD-LLM-005 | `src/llm/types.rs` |
| SRS-LLM-002 | 5.2절 프롬프트 규칙, 쉘별 문법 지시 | PRD-LLM-001-003, PRD-LLM-006, PRD-LLM-008 | `src/llm/prompt.rs` |
| SRS-LLM-003 | 5.3절 4단계 응답 파싱과 형식 오류 시 재시도 1회 | PRD-LLM-010 | `src/llm/parse.rs`, `src/runtime.rs` |
| SRS-LLM-004 | SSE 파서: `event:`/`data:` 줄 처리, `:` 주석 무시, 여러 줄 data 결합 | PRD-LLM-001 | `src/llm/sse.rs` |
| SRS-LLM-005 | 5.4절 ChatGPT Provider 요청 규칙 | PRD-LLM-009, PRD-AUTH-016 | `src/llm/chatgpt.rs` |
| SRS-LLM-006 | 모델 목록 24시간 캐시 (상태 파일), `ModelNotFound` 수신 시 캐시 무효화 | PRD-LLM-009 | `src/llm/chatgpt.rs` |
| SRS-LLM-007 | 제한 시간: 연결 10초, 첫 SSE 이벤트 60초, 전체 120초 | PRD-LLM-011 | `src/llm/chatgpt.rs` |
| SRS-LLM-008 | 오류 매핑: research/chatgpt-oauth.md 8.6절 표 기준 `LlmError` 변환 | PRD-AUTH-017, PRD-LLM-010 | `src/llm/chatgpt.rs` |
| SRS-LLM-009 | Mock Provider: 시나리오 파일(JSON 단계 목록: 응답, 지연, 오류 이름, 원문 응답)로 동작. `HOWTUI_MOCK_SCENARIO=<파일>` 또는 `echo`. 같은 기능 플래그에서만 테스트 재정의 `HOWTUI_TEST_API_BASE`, `HOWTUI_TEST_AUTH_BASE`, `HOWTUI_TEST_PANIC` 허용 (릴리스 빌드 문자열 부재 확인) | PRD-LLM-012 | `src/llm/mock.rs`, `src/auth/oauth.rs`, `src/runtime.rs` |
| SRS-LLM-010 | 취소 시 HTTP 스트림 즉시 drop, 결과 미반환 | PRD-LLM-004 | `src/llm/chatgpt.rs` |
| SRS-LLM-011 | P5 Provider 추가 시 trait 변경 없이 구현체 추가 (OpenAI, Gemini, Anthropic, OpenAI 호환) | PRD-LLM-013-017 | `src/llm/` |
| SRS-LLM-012 | P6 로컬 서버: OpenAI 호환 채팅 API 공통 구현 + 서버 종류별 탐지(Ollama `/api/version`, llama-server `/health`), 기본 주소 설정 가능 | PRD-LLM-018-020 | `src/llm/local.rs` |

## 6. 터미널 세션과 Output Layer

### 6.1 터미널 세션 수명 주기

근거: SPIKE-01 실측, ADR-0002.

```text
(1) 제어 터미널 열기   Unix: /dev/tty (O_RDWR)      Windows: 출력 핸들 선택 (아래 표), 입력은 crossterm 콘솔 입력(CONIN$)
(2) fd 1 보관과 전환   Unix: saved = dup(1); dup2(tty, 1)   Windows: 해당 없음
(3) raw mode 진입, bracketed paste 활성화 (키보드 확장 플래그는 P1 미적용, PRD-KEY-002 이월)
(4) 시작 커서 위치 질의, 열 > 0 이면 CRLF 출력 후 Inline 뷰포트 생성 (이벤트 루프 시작 전)
(5) 이벤트 루프 (10절)
(6) 종료 처리: 뷰포트 첫 행 이동 → 커서 뒤 영역 삭제(ESC[J, 커서 질의 없음) → 커서 원위치
             → 키보드 확장 해제 → bracketed paste 이전 상태 복원 → 커서 표시 → raw mode 해제 → fd 1 복구
(7) 확정 명령어 전달 (output 모듈)
```

Windows 출력 핸들 선택 순서 (Phase 1 실측 대상):

| 순서 | 조건 | 출력 대상 | 근거 |
| --- | --- | --- | --- |
| (1) | stdout이 콘솔 | stdout (Rust 표준 라이브러리의 콘솔 UTF-16 출력 경로) | 위젯 모드, stdout 미리디렉션 직접 실행 |
| (2) | stdout 리디렉션, stderr가 콘솔 | stderr | Microsoft 문서: stderr와 `CONOUT$`는 활성 화면 버퍼 공유 (research/rust-crates.md 4절) |
| (3) | stdout, stderr 모두 리디렉션 | `CONOUT$` 핸들 + `WriteConsoleW` 래퍼 (UTF-8을 UTF-16으로 변환) | 파일 핸들 `WriteFile` 경로는 콘솔 코드 페이지 의존 (research/shell-integration.md 4.5절) |
| (4) | 콘솔 없음 | TUI 미시작, 종료 코드 3 | |

bracketed paste 복원 규칙: 직접 실행 모드는 종료 시 해제 (쉘이 다음 입력 줄에서 다시 활성화). 위젯 모드(`--widget`)는 종료 시 활성 상태 유지: 쉘 입력 편집 도중이므로 해제하면 여러 줄 붙여넣기 즉시 실행 위험. 이전 상태 질의(DECRQM)는 응답 읽기가 crossterm 입력 처리와 경합하여 미사용 (구현 시 결정, 2026-10-22).

### 6.2 복구 경로

| 상황 | 처리 |
| --- | --- |
| 정상 확정, 취소 | 6.1 (6) 순서 수행 |
| LLM 오류 | 종료 사유 아님, 화면 표시 |
| panic | 전역 panic hook에서 (6) 중 화면 삭제 제외 단계 수행 후 기본 hook 호출, 종료 코드 101 |
| SIGTERM, SIGHUP (Unix) | tokio 시그널 수신 → (6) 수행 → 143, 129 종료 |
| 콘솔 닫힘, 로그오프 (Windows) | 미검증, Phase 1 실측 후 기록 |
| SIGKILL, 전원 차단 | 복구 불가. 문서에 `reset` 또는 `stty sane` 안내 |
| Ctrl+Z (raw mode) | ISIG 비활성 상태로 키 입력 처리 (Undo) |

### 6.3 Output Layer

```rust
pub enum OutputTarget { Stdout, File(PathBuf) }
pub fn emit(target: &OutputTarget, command: &str) -> io::Result<()>;
```

| 대상 | 기록 내용 | 사용 |
| --- | --- | --- |
| `Stdout` | 명령어 + LF | 직접 출력 모드 (`howtui`, `cmd=$(howtui)`, `--no-tui`) |
| `File(path)` | 명령어 UTF-8 (BOM 없음, LF 줄바꿈, 끝 줄바꿈 없음). 쉘이 만든 파일을 덮어쓰기, 파일 부재 시 0600으로 생성 | 위젯 모드 3개 쉘 공통 (9절) |

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-TERM-001 | 6.1절 수명 주기 순서 준수 | PRD-UI-010, PRD-OUT-002 | `src/platform/tty.rs`, `src/ui/terminal.rs` |
| SRS-TERM-002 | Unix fd 1 전환과 복구 (`dup`, `dup2`), 복구 실패 시 stderr 오류와 종료 코드 1 | PRD-OUT-002, PRD-OUT-006 | `src/platform/tty.rs` |
| SRS-TERM-003 | Windows `CONOUT$` 백엔드, stdout 무변경, VT 처리 활성화 (Phase 1 실측 필요) | PRD-OUT-002 | `src/platform/tty.rs` |
| SRS-TERM-004 | 제어 터미널 열기 실패 시 TUI 미시작, 종료 코드 3, 제어 시퀀스 없는 stderr 메시지 | PRD-OUT-007 | `src/platform/tty.rs`, `src/main.rs` |
| SRS-TERM-005 | 시작 커서 열 > 0 처리와 종료 시 원위치 (SPIKE-04 보완 사항) | PRD-UI-011 | `src/ui/terminal.rs` |
| SRS-TERM-006 | 6.2절 복구 경로 (panic hook, 시그널) | PRD-UI-010, PRD-ERR-004, PRD-ERR-005 | `src/ui/terminal.rs`, `src/runtime.rs` |
| SRS-TERM-007 | Inline 높이 = min(설정 높이, 터미널 높이). 최소 크기(30x3) 미만 시 크기 부족 화면 | PRD-UI-002, PRD-UI-009 | `src/ui/inline.rs` |
| SRS-TERM-008 | 리사이즈: crossterm Resize 이벤트 수신 후 다음 draw에서 ratatui autoresize 사용. 가로 축소 시 전체 화면 정리 동작은 ratatui 기본 동작 유지, 문서에 명시 | PRD-UI-008 | `src/ui/inline.rs` |
| SRS-OUT-001 | 6.3절 Output Layer. TUI 종료와 fd 복구 이후에만 호출 | PRD-OUT-001, PRD-OUT-005 | `src/output/mod.rs` |
| SRS-OUT-002 | 취소, 오류 경로에서 Output Layer 미호출 | PRD-OUT-003, PRD-OUT-004 | `src/main.rs` |
| SRS-OUT-003 | 비대화형 모드: 제어 터미널 미사용, stdout 명령어, stderr 설명과 경고 (ANSI 없음) | PRD-OUT-008, PRD-CLI-005 | `src/cli/no_tui.rs` |
| SRS-OUT-004 | 로그는 파일 전용 (`HOWTUI_LOG` 설정 시), stdout과 stderr 미사용 | PRD-OUT-010, PRD-PRIV-005 | `src/main.rs` |
| SRS-OUT-005 | 출력 파일 대상: 확정 시에만 기록, 취소와 오류 시 미기록. 기존 파일 덮어쓰기, 부재 시 0600 생성 | PRD-OUT-009 | `src/output/mod.rs` |

## 7. TUI 상태 관리와 화면 모드

### 7.1 상태 관리

- PRD 3절 상태 전이 표를 `update` 함수 분기로 구현. 표의 각 행은 단위 테스트 1개 이상 대응 (TC-UI 계열).
- keymap은 `(KeyEvent, Phase, Focus)` → `Option<KeyAction>` 순수 함수. PRD 4절 표와 1:1 대응 테스트.

### 7.2 Inline 레이아웃

높이 H, 폭 W 기준 영역 배분 (우선순위 순):

| 영역 | 줄 수 |
| --- | --- |
| 상태줄 | 1 (항상) |
| 질문 | 1-2 (W 기준 줄바꿈, 최대 2줄, 초과 시 앞부분 생략 표시) |
| 명령어 | 1 이상, 내용 줄 수와 남은 줄 수 중 작은 값 (최대 H-3), 초과 시 스크롤 |
| 경고 | 위험 판정 존재 시 1 |
| 설명 | 남은 줄 전체 (0 가능) |

### 7.3 Fullscreen (P3)

- 같은 `State` 사용. 레이아웃: 상단 대화 이력 스크롤 영역, 하단 질문, 명령어, 경고, 상태줄.
- Alternate Screen 진입과 해제는 터미널 세션 설정에서 처리. 뷰포트 `Viewport::Fullscreen`.
- 모드 결정 순서: CLI `--mode` > 설정 `ui.mode` > 기본 `inline`.

### 7.4 편집기 구현

ADR-0015: `app::editor::TextBuffer` 자체 구현 (ratatui-textarea 미사용).

- 기능 범위: 4.3절 구조, PRD 4절 키 표의 편집 동작, 여러 줄, Undo/Redo, 붙여넣기, 읽기 전용 전환
- 표시: ui 계층이 `TextBuffer`의 줄과 커서를 읽어 줄바꿈(`WordOrGlyph` 방식과 동등한 문자 단위 줄바꿈)과 세로 스크롤 계산
- 실제 터미널 커서를 편집 위치에 배치 (`Frame::set_cursor_position`). 한국어 IME 조합 위치 표시 목적, 실측은 Phase 1
- 근거: (a) `app` 순수 코어 규칙(SRS-MOD-002) 유지 (b) ratatui-textarea의 CJK 가로 스크롤 커서 소실 결함(#35) 회피 (c) Undo 키(Ctrl+Z, Ctrl+Y)와 AI 응답 단일 Undo 단계 제어

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-TUI-001 | 상태 전이 표 구현과 행 단위 테스트 | PRD-UI-013 | `src/app/update.rs` |
| SRS-TUI-002 | keymap 순수 함수와 PRD 4절 표 대응 테스트 | PRD-KEY-001 | `src/ui/keymap.rs` |
| SRS-TUI-003 | 7.2절 Inline 레이아웃 규칙 | PRD-UI-002-006 | `src/ui/inline.rs` |
| SRS-TUI-004 | 렌더링 스냅샷 테스트 (ratatui TestBackend) | PRD-UI-003-006 | `tests/`, `src/ui/` |
| SRS-TUI-005 | 응답 반영 시 포커스 질문 영역 유지와 설명 갱신 필요 표시 해제. 질문 영역 빈 입력 Enter는 명령어 확정(위험 높음 재확인 동일), ↓는 명령어 영역 이동, 명령어 첫 줄 ↑는 질문 영역 이동, 빈 질문 영역의 Undo와 Redo는 명령어 대상 (CR-001) | PRD-UI-012, PRD-EDIT-007, PRD-KEY-001 | `src/app/update.rs` |
| SRS-TUI-006 | 위험 확인 상태: 높음 판정 명령어 확정 시 진입, Enter 외 키 입력 시 해제 | PRD-SAFE-003 | `src/app/update.rs` |
| SRS-TUI-007 | 7.3절 Fullscreen 렌더러와 Alternate Screen 처리 | PRD-FS-001-005 | `src/ui/fullscreen.rs` |
| SRS-TUI-008 | Shift+Enter: crossterm 키보드 확장 플래그 지원 판정 후 활성화 | PRD-KEY-002 | `src/ui/terminal.rs` |
| SRS-TUI-009 | 미로그인 시작 시 로그인 자동 시작, 높이 최소 14행. 로그인 진행 화면: 진행 표시, 단계 안내 2개((1) 브라우저 로그인과 허용, (2) 오류 화면 시 주소 붙여넣기), 인가 URL 전체, 붙여넣기 입력란. Esc 취소 후 로그인 필요 화면, 다시 Esc 종료 (CR-002) | PRD-AUTH-013 | `src/ui/inline.rs`, `src/runtime.rs` |
| SRS-TUI-010 | 상태줄 플랜 사용 표시와 최초 1회 안내 | PRD-AUTH-014 | `src/ui/inline.rs`, `src/auth/state.rs` |
| SRS-TUI-011 | 7.4절 자체 편집 버퍼와 ui 계층 줄바꿈, 스크롤, 실제 커서 배치 | PRD-EDIT-001, PRD-EDIT-002, PRD-EDIT-004, PRD-EDIT-005 | `src/app/editor.rs`, `src/ui/widgets.rs` |
| SRS-TUI-012 | 키 바인딩 재정의 설정 미제공 (설정 스키마에 키 항목 부재) | PRD-KEY-003 | `src/config/mod.rs` |
| SRS-TUI-013 | 화면 문구에 팝업 오버레이 동등 표현 미사용 | PRD-UI-014 | `src/ui/`, `README.md` |

### 7.5 다국어 메시지

ADR-0016 적용.

```rust
pub enum Lang { En, Ko }
pub enum Msg { StatusSubmit, LoginRequired, /* ... */ }
pub fn tr(lang: Lang, msg: Msg) -> &'static str;          // 언어별 match 전체 열거
pub fn trf(lang: Lang, msg: Msg, args: &[(&str, &str)]) -> String; // {name} 자리 채움
```

- 언어 결정 순서: `HOWTUI_LANG` > 설정 `ui.language` > `auto` > 영어
- `auto` 판정: Unix `LC_ALL` > `LC_MESSAGES` > `LANG` 값의 앞부분이 `ko`이면 한국어. Windows 사용자 기본 UI 언어의 주 언어 ID가 한국어(0x12)이면 한국어. 그 외 영어
- 언어 추가: `Lang` 항목과 해당 언어 match 함수 추가. match 누락은 컴파일 오류로 검출

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-I18N-001 | `Lang`, `Msg`, `tr` 구조. 언어별 메시지 함수의 전체 열거 match (와일드카드 금지) | PRD-I18N-001, PRD-I18N-003 | `src/i18n/` |
| SRS-I18N-002 | 언어 결정 순서와 `auto` 판정 규칙 | PRD-I18N-002 | `src/i18n/detect.rs` |
| SRS-I18N-003 | 자리 채움 메시지: 언어 간 자리 이름 집합 일치 테스트 | PRD-I18N-003 | `src/i18n/` |
| SRS-I18N-004 | clap 도움말 문자열을 선택 언어로 구성 (builder 방식 문자열 주입) | PRD-I18N-004 | `src/cli/mod.rs` |
| SRS-I18N-005 | JSON 키, 상태 값, 종료 코드, stdout 출력은 언어 무관 | PRD-I18N-005 | `src/doctor/json.rs`, `src/output/` |

## 8. 오류 코드와 실패 처리

### 8.1 종료 코드

ADR-0003 표 적용.

| 코드 | 의미 | 발생 경로 |
| --- | --- | --- |
| 0 | 성공 | 확정 출력, 하위 명령 성공 |
| 1 | 내부 오류 | 분류 불가 오류, fd 복구 실패 |
| 2 | 사용법 오류 | clap |
| 3 | 사용 가능한 터미널 없음 | 제어 터미널 열기 실패, 커서 위치 질의 무응답 |
| 4 | 인증 필요, 인증 실패 | `--no-tui`, `login`, `auth status` 일부 |
| 5 | 플랜 비대상, 사용량 한도 | `--no-tui` |
| 6 | 네트워크, 일시 장애 | `--no-tui`, `login`, `logout` |
| 7 | Provider 오류 | `--no-tui` |
| 8 | 설정 오류 | 모든 경로 (설정 로드 단계) |
| 101 | panic | 모든 경로 |
| 129, 143 | SIGHUP, SIGTERM | TUI |
| 130 | 사용자 취소 | TUI |

### 8.2 오류 분류와 사용자 표시

| `LlmError` | TUI 표시 (원인 / 조치) | 비대화형 종료 코드 | 자격증명 처리 |
| --- | --- | --- | --- |
| AuthRequired | 로그인 필요 / Enter로 로그인 | 4 | 해당 없음 |
| AuthFailed | 인증 실패 / 다시 로그인 | 4 | 갱신 종결 오류일 때만 토큰 삭제 |
| NotEligible | ChatGPT 플랜 사용 불가 계정 / 플랜과 워크스페이스 확인 | 5 | 보존 |
| PlanNotGranted | 로그인 시 플랜 사용 권한 미허용 / 다시 로그인해 허용 (요청 미발생) | 5 | 보존 |
| ClientNotEnabled | howtui 플랜 사용 미활성화 / 활성화 전까지 사용 불가 (E-01) | 5 | 보존 |
| UsageLimit | 사용량 한도 도달 / 사용량 관리 URL | 5 | 보존 |
| RegionOrPolicy | 정책 또는 지역 제한 / 상세 메시지 | 7 | 보존 |
| PermissionConfig | 권한 구성 오류 / 다시 로그인 | 7 | 보존 |
| Unsupported, ModelNotFound | 모델 또는 기능 미지원 / 모델 설정 확인 | 7 | 보존 |
| Transient, Network, Timeout | 일시 장애 또는 연결 실패 / 잠시 후 재시도 | 6 | 보존 |
| InvalidResponse | 응답 형식 오류 / 재시도 | 7 | 보존 |
| Cancelled | 요청 취소됨 | 130 | 보존 |

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-ERR-001 | 8.1절 종료 코드 매핑 (`LlmError::exit_code`, CLI 결과 분기) | PRD-CLI-010 | `src/llm/types.rs`, `src/cli/` |
| SRS-ERR-002 | 8.2절 오류 분류별 메시지 (원인 1줄 + 조치 1줄) | PRD-ERR-001, PRD-ERR-002 | `src/app/update.rs`, `src/llm/error.rs` |
| SRS-ERR-003 | 사용량 한도 메시지에 리셋 시각 미표시, URL `https://chatgpt.com/settings/usage` 표시 | PRD-ERR-003 | `src/llm/error.rs` |
| SRS-ERR-004 | 일시 장애 재시도: 비대화형 모드와 토큰 갱신에 한해 최대 2회, 지수 백오프(1초, 2초) | PRD-AUTH-018 | `src/llm/chatgpt.rs`, `src/auth/session.rs` |

## 9. Shell Adapter 인터페이스

근거: [research/shell-integration.md](research/shell-integration.md), SPIKE-03, SPIKE-04, SPIKE-05 (ADR-0009).

### 9.1 구성

- 쉘 쪽 어댑터: `howtui init <bash|zsh|powershell>`가 출력하는 쉘 스크립트 (바이너리에 내장, `src/shell/scripts/`). 구성 2개: 단축키 위젯(`#@bind` 구간)과 `howtui` 쉘 함수(`#@wrap` 구간, 9.6절).
- 바이너리 쪽: 위젯 모드 `howtui --widget`. 입력은 환경변수, 결과는 출력 파일. 쉘 함수 경유 직접 실행은 `HOWTUI_NEXT_INPUT_FILE` (9.6절).
- 사용자 등록 방식: Bash `eval "$(howtui init bash)"`, Zsh `eval "$(howtui init zsh)"`, PowerShell `howtui init powershell | Out-String | Invoke-Expression` (rc 파일과 Profile 직접 편집은 사용자 수행).

### 9.2 위젯 프로토콜 (4단계 구조)

| 단계 | 내용 |
| --- | --- |
| (1) 입력 전달 | 환경변수 `HOWTUI_SHELL`(`bash`, `zsh`, `powershell`), `HOWTUI_BUFFER`(현재 입력란 전체), `HOWTUI_LBUFFER`(커서 앞 문자열, Bash 4.x 생략), `HOWTUI_OUTPUT_FILE`(쉘이 만든 임시 파일), `HOWTUI_SHELL_VERSION`(선택). argv에 사용자 문자열 미전달 (PowerShell 5.1과 7.3 이상의 인자 인용 차이) |
| (2) TUI 실행 | 표준 입출력 전부 터미널 연결. Bash, Zsh: `</dev/tty >/dev/tty 2>/dev/tty`. PowerShell: 리디렉션 없는 `System.Diagnostics.Process` 실행 (`& howtui.exe`는 stdout 리디렉션 발생) |
| (3) 결과 기록 | 확정 시에만 출력 파일 기록 (6.3절 `File`) |
| (4) 판정과 반영 | 종료 코드 0이고 파일이 비어 있지 않을 때만 입력란 교체, 커서 끝. 그 외 종료 코드는 입력란 무변경 |

- 바이너리 동작: `HOWTUI_OUTPUT_FILE` 미설정 시 종료 코드 2. 명령어 영역 초기값 = `HOWTUI_BUFFER`, 커서 = `HOWTUI_LBUFFER` 길이 위치 (없으면 끝).
- 종료 코드는 ADR-0003 표 그대로 사용 (취소 130). 쉘 스크립트는 0 외 모든 값을 "무변경"으로 처리.

### 9.3 쉘별 규칙

| 항목 | Bash | Zsh | PowerShell |
| --- | --- | --- | --- |
| 최소 버전 | 4.0 (`READLINE_LINE`), 미만은 바인딩 생략과 경고. `HOWTUI_LBUFFER`는 5.0 이상 (`READLINE_POINT` 문자 단위 실측) | 5.1 이상 (검증 5.9) | Windows PowerShell 5.1 (PSReadLine 2.0.0), PowerShell 7.x (검증 Linux 7.4, PSReadLine 2.3.5) |
| 바인딩 | `bind -m <keymap> -x` (emacs, vi-insert, vi-command). Bash 5.1은 `bind -r` 선행 (기존 바인딩 덮어쓰기 결함) | `zle -N howtui-widget`, `bindkey -M emacs/viins/vicmd` | `Set-PSReadLineKeyHandler -Chord` (Vi 모드는 Insert, Command 각각) |
| 버퍼 교체 | `IFS= read -r -d '' READLINE_LINE <tmp` (Bash 5.2는 `bind -x` 안의 `$(<tmp)` 치환 후 다음 명령 줄 절단, 실측), `READLINE_POINT=${#READLINE_LINE}` (5.0 이상 문자 단위, 4.x는 `LC_ALL=C` 바이트 단위. 범위 밖 값은 5.2에서 미보정) | `BUFFER=$(<tmp)`, `CURSOR=$#BUFFER`, `zle reset-prompt` | `Replace(0, $line.Length, $new)` (빈 입력은 `Insert`), `SetCursorPosition`, CRLF를 LF로 정규화 (PSReadLine 2.0.0 대응) |
| 화면 재그림 | 자동 (4.4 이상 마지막 프롬프트 줄) | `zle reset-prompt` (바이너리의 정확한 커서 복원 전제) | `InvokePrompt($null, $y)`, `$y = CursorTop - ExtraPromptLineCount`를 TUI 종료 직후, 입력 교체 전에 계산 (화면 스크롤과 여러 줄 교체 대응), 0과 `BufferHeight - 1` 사이 보정 |
| 바이너리 조회 | `command howtui` | `command howtui` | `Get-Command howtui -CommandType Application` 실행 시점 조회 (경로 내장 금지, 한글 사용자 폴더 대응) |
| 스크립트 문자 집합 | 제한 없음 | 제한 없음 | ASCII 전용 (5.1의 BOM 없는 스크립트 ANSI 해석) |
| bracketed paste | `bind -v`의 `enable-bracketed-paste` 값을 `HOWTUI_BRACKETED_PASTE`로 전달 | `zle_bracketed_paste` 설정 시 `HOWTUI_BRACKETED_PASTE=1` | 미전달 (TUI 종료 시 해제) |
| 바이너리 없음 | 쉘 오류 메시지, 입력란 무변경 | `zle -M` 메시지, 입력란 무변경 | `Ding`, 입력란 무변경 |
| 다음 입력 전달 (9.6절) | `history -s`(현재 `howtui` 줄 교체), 4.0 이상은 `PROMPT_COMMAND`에서 `ESC[5n` 요청, 응답 `ESC[0n` 바인딩(`bind -x`, 3개 키맵)이 빈 입력란에 삽입 | `print -rz` (버퍼 스택) | `AddToHistory`, `PowerShell.OnIdle` 1회 이벤트에서 빈 입력란에 `Insert` |

### 9.4 단축키

- 기본 키: Ctrl+G (Bash `\C-g`, Zsh `^G`, PSReadLine `Ctrl+g`).
- 대체되는 기본 동작: Bash emacs `abort`, Zsh `send-break`, PSReadLine Emacs와 Vi 모드 `Abort`. PSReadLine Windows 모드는 미할당.
- 알려진 충돌: VS Code 통합 터미널(Windows, Linux)의 "Go to Recent Directory". 문서에 `terminal.integrated.commandsToSkipShell` 설정 안내 (적용 실측 필요).
- 옵션: `howtui init <쉘> --key <키>`, `--no-bind` (바인딩 생략), `--no-wrap` (`howtui` 쉘 함수 생략).
- 키 형식: `ctrl+<영문자>`, `alt+<영문자>` (`-` 구분자, 대소문자 무관). 쉘 입력에 필요한 Ctrl+C, D, H, I, J, M, Q, S, Z는 거부 (종료 코드 2).

### 9.5 쉘 판정 (직접 실행 모드)

우선순위: (1) `--shell` 옵션 (2) `HOWTUI_SHELL` 환경변수 (init 스크립트 설정) (3) OS 기본 조합 (Linux Bash, macOS Zsh, Windows PowerShell). `$SHELL`은 로그인 쉘이므로 판정 근거로 사용하지 않고 doctor 정보로만 표시.

### 9.6 직접 실행 결과 전달 (CR-003, 4단계 구조)

| 단계 | 내용 |
| --- | --- |
| (1) 함수 호출 | init 스크립트의 `howtui` 쉘 함수가 임시 파일 생성 후 `HOWTUI_SHELL`, `HOWTUI_SHELL_VERSION`, `HOWTUI_SHELL_INTEGRATION`, `HOWTUI_NEXT_INPUT_FILE`를 해당 호출에만 지정해 바이너리 실행 (전역 export 없음). 인자와 하위 명령은 그대로 전달 |
| (2) 경로 판정 | 바이너리는 TUI 시작 전 stdout이 터미널이고 `HOWTUI_NEXT_INPUT_FILE`가 비어 있지 않을 때만 확정 명령어를 파일에 기록 (6.3절 `File` 형식). 그 외(캡처, 파이프, 리디렉션, `--no-tui`, 하위 명령)는 기존 동작 |
| (3) 파일 기록 실패 | stdout 전달로 대체 |
| (4) 쉘 반영 | 종료 코드 0이고 파일이 비어 있지 않을 때만 9.3절 "다음 입력 전달" 방식 적용. 종료 코드는 그대로 반환 (취소 130) |

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-SH-001 | `howtui init <쉘> [--key K] [--no-bind] [--no-wrap]`: 내장 스크립트 출력, 알 수 없는 쉘과 잘못된 키는 종료 코드 2. 설정 파일 오류와 무관하게 동작 (rc 파일에서 매번 실행) | PRD-CLI-008, PRD-SH-009 | `src/shell/init.rs`, `src/shell/scripts/`, `src/cli/mod.rs` |
| SRS-SH-002 | Bash 스크립트: 9.3절 규칙, 대화형 쉘 한정, 서브쉘 안에서만 환경변수 export | PRD-SH-001, PRD-SH-006 | `src/shell/scripts/howtui.bash` |
| SRS-SH-003 | Zsh 스크립트: 9.3절 규칙, `emulate -L zsh`, `</dev/tty` 필수 (위젯 stdin은 /dev/null) | PRD-SH-002, PRD-SH-006 | `src/shell/scripts/howtui.zsh` |
| SRS-SH-004 | PowerShell 스크립트: 9.3절 규칙, PSReadLine 2.0.0 미만 또는 미로드 시 바인딩 생략과 경고, UTF-8(BOM 없음) 명시 읽기 | PRD-SH-003, PRD-SH-006 | `src/shell/scripts/howtui.ps1` |
| SRS-SH-005 | 9.2절 위젯 프로토콜 (`--widget`, 환경변수, 출력 파일, 종료 코드 판정) | PRD-SH-005, PRD-SH-007, PRD-OUT-009, PRD-EDIT-008 | `src/cli/widget.rs` |
| SRS-SH-006 | 9.4절 기본 키와 `--key`, `--no-bind`, `--no-wrap`, 키 형식 검증 | PRD-SH-004 | `src/shell/init.rs` |
| SRS-SH-007 | 9.5절 쉘 판정 우선순위 | PRD-CLI-007 | `src/shell/mod.rs` |
| SRS-SH-008 | 통합 표식: init 스크립트가 `HOWTUI_SHELL_INTEGRATION=<쉘>` 설정, doctor가 판독 | PRD-SH-011, PRD-DOC-006 | `src/shell/scripts/`, `src/doctor/` |
| SRS-SH-009 | 실패 대체 경로: 위젯 모드에서 출력 파일 기록 실패 시 줄바꿈 후 원인 1줄과 명령어를 터미널에 표시, 종료 코드 1 (입력란 무변경) | PRD-SH-007, PRD-SH-008 | `src/cli/widget.rs` |
| SRS-SH-010 | 스크립트 머리 주석에 등록 방법과 제거 방법 기재. 설치 스크립트와 바이너리는 rc 파일과 Profile 미수정 | PRD-SH-009, PRD-SH-010 | `src/shell/scripts/`, `scripts/` |
| SRS-SH-011 | PowerShell 5.1과 7 공통 동작: 실행 정책 `Restricted`에서 Profile 미실행 안내, Profile 경로 차이 안내 | PRD-SH-003 | `src/shell/scripts/howtui.ps1`, 문서 |
| SRS-SH-012 | 쉘별 지원 상태 표기: 검증 쉘 버전과 터미널을 문서 표로 관리 | PRD-SH-012 | `README.md` 6절 |
| SRS-SH-013 | 9.6절 직접 실행 결과 전달: 쉘 함수, `HOWTUI_NEXT_INPUT_FILE` 경로 판정(stdout 터미널 조건), 기록 실패 시 stdout 대체 | PRD-SH-013 | `src/cli/widget.rs`, `src/shell/scripts/` |
| SRS-SH-014 | 쉘별 다음 입력 전달 방식 (9.3절 표). Bash는 터미널 상태 보고 요청을 프롬프트 직전에만 송신(뒤따르는 명령의 입력 오염 방지), 입력란이 비어 있을 때만 삽입. PowerShell은 입력란이 비어 있을 때만 삽입 | PRD-SH-013 | `src/shell/scripts/` |

## 10. 비동기 작업과 취소

ADR-0005 적용. 터미널 입력과 렌더링은 UI 스레드(주 스레드) 동기 루프, 네트워크 작업만 tokio 작업 스레드.

```text
UI 스레드:
loop {
  timeout = 요청 중이면 120ms, 아니면 500ms
  if crossterm::event::poll(timeout) { keymap(read()) → update → effects }
  while let Ok(msg) = results.try_recv() { update(msg) → effects }   // LLM 결과, 로그인 결과, 시그널
  effects 실행 (StartRequest → runtime.spawn, CancelRequest → token.cancel() + abort)
  상태 변경 또는 진행 표시 갱신 시 draw (리사이즈 시 커서 질의도 UI 스레드에서 수행)
}
tokio 작업 스레드: LLM 요청, 토큰 갱신, 콜백 리스너, 시그널 수신 → results 채널 전송
```

- 터미널 입력을 UI 스레드에서만 읽는 이유: ratatui Inline의 커서 위치 질의와 다른 이벤트 읽기 주체 간 경합 (ratatui #2640, research/rust-crates.md 2절). 질의와 입력 읽기를 같은 스레드에 두어 경합 제거.

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-ASYNC-001 | tokio 멀티스레드 런타임, 작업 스레드 2개 (UI 스레드와 별도) | PRD-LLM-004 | `src/main.rs` |
| SRS-ASYNC-002 | UI 스레드 단일 동기 루프 (`poll`, `read`, 결과 채널 `try_recv`). 터미널 입력 읽기 주체는 UI 스레드 1개 | PRD-UI-007 | `src/runtime.rs` |
| SRS-ASYNC-003 | `RequestId` 단조 증가, 결과 ID 불일치 시 폐기 | PRD-LLM-004 | `src/app/update.rs` |
| SRS-ASYNC-004 | 취소 토큰과 작업 abort 병행, 취소 후 100ms 이내 상태 복귀 | PRD-LLM-004, PRD-UI-007 | `src/runtime.rs` |
| SRS-ASYNC-005 | 진행 표시 갱신 주기 120ms, 요청 없을 때 poll 대기 500ms (유휴 CPU 사용 최소화) | PRD-UI-007 | `src/runtime.rs` |
| SRS-ASYNC-006 | 시작 시 인증 확인, 토큰 갱신, 모델 목록 조회는 백그라운드 작업. 첫 화면 표시를 지연시키지 않음 | PRD-UI-001 | `src/runtime.rs` |

## 11. OAuth 인증과 저장 구조

근거: [research/chatgpt-oauth.md](research/chatgpt-oauth.md), ADR-0007.

### 11.1 로그인 절차 (8단계 파이프라인)

1. host ID 확인: 상태 파일에 없으면 `urn:uuid:<UUIDv4>` 생성 후 저장
2. state, nonce, PKCE verifier 생성 (각 32바이트 난수, base64url 패딩 없음), challenge = base64url(SHA-256(verifier))
3. 루프백 리스너 기동: `127.0.0.1:<port>` (기본 OS 자동 할당, `--port` 지정 가능), 경로 `/auth/callback`
4. 인가 URL 구성: 최초 등록은 `client_id=dynamic_agent_client`, `agent_name_hint=howtui`, 재인가는 저장된 발급 `client_id` (+ `id_token_hint` 선택). 공통 `ext_agent_host_id`, `response_type=code`, `redirect_uri`, `scope`, `resource`, `state`, `nonce`, `code_challenge_method=S256`, `code_challenge`
5. 브라우저 실행 (실패 또는 `--no-browser` 시 URL 표시만), 제한 시간 10분
6. 콜백 검증: 경로, state 상수 시간 비교, `error=access_denied` 처리, 발급 `client_id` 수신 확인 (재인가 시 다른 `client_id`는 거부). 발급 `client_id`를 코드 교환 전에 상태 파일에 저장
7. 코드 교환: `POST https://auth.openai.com/api/accounts/oauth/token` (form, `grant_type=authorization_code`, `client_id`, `code`, `code_verifier`, `redirect_uri`, `resource`)
8. ID 토큰 검증 (RS256, JWKS `https://auth.openai.com/.well-known/jwks.json`, `iss`, `aud`, `exp`, `iat`, `nonce`, 시계 오차 5초), 응답 scope의 `chatgpt.tokens.use.direct` 확인, 자격증명 저장

구현 방식 (research/rust-crates.md 13절):

| 항목 | 방식 |
| --- | --- |
| 난수 | `getrandom` 32바이트 |
| PKCE challenge | `sha2::Sha256` + `base64` `URL_SAFE_NO_PAD` |
| 콜백 리스너 | `tokio::net::TcpListener` (`127.0.0.1:0` 또는 지정 포트), `httparse` 요청 줄과 헤더 파싱, `url::form_urlencoded` 질의 파싱 |
| ID 토큰 서명 | JWT 3부분 base64url 해석, 헤더 `alg == "RS256"`과 `kid` 확인, JWKS의 `n`, `e`로 `ring::signature::RsaPublicKeyComponents::verify(&RSA_PKCS1_2048_8192_SHA256, ...)` |
| 클레임 검증 | `iss == "https://auth.openai.com"`, `aud`에 발급 `client_id` 포함, `exp`, `iat`, `nonce`, `azp` 존재 시 `client_id` 일치 (자체 구현) |
| 브라우저 실행 | `open::that_detached` |

### 11.2 저장 구조

| 파일 또는 항목 | 위치 (12절) | 내용 | 비밀 여부 |
| --- | --- | --- | --- |
| 상태 파일 `state.json` | 데이터 디렉터리 | `version`, `host_id`, 등록 정보(`subject`, 마스킹 전 이메일, `client_id`, 등록 시각), 플랜 안내 표시 여부, 모델 목록 캐시, 구조화 출력 지원 판정 | 비밀 아님 (0600 적용) |
| 자격증명 | OS 보안 저장소 서비스 `howtui`, 계정 `chatgpt:<client_id>:{access,refresh,id,meta}` 분할 저장 (Windows 항목 크기 2560바이트 제한 대응). 보안 저장소 쓰기 실패 또는 사용 불가 시 `credentials.json` (0600)으로 전환하고 상태 파일에 기록. `HOWTUI_CREDENTIAL_STORE`로 강제 지정 | `access_token`, `refresh_token`, `id_token`, `token_type`, `scope`, `expires_at`, `earliest_refresh_at`, `saved_at` | 비밀 |
| 잠금 파일 `auth.lock` | 데이터 디렉터리 | 갱신 직렬화용 (`std::fs::File::lock`) | 비밀 아님 |

### 11.3 갱신, 로그아웃

- 갱신 조건: `now >= expires_at - 60초` 그리고 `now >= earliest_refresh_at`(값 존재 시)
- 갱신 절차: 잠금 획득 → 저장소 재조회(다른 프로세스 갱신 여부 확인) → 필요 시 `grant_type=refresh_token` 요청(`client_id`, `refresh_token`, `resource`, scope 생략) → 새 토큰 일괄 저장 → 잠금 해제
- 갱신 종결 오류(`invalid_grant`, `invalid_refresh_token`, `token_expired`, `refresh_token_expired`, `refresh_token_invalidated`, `refresh_token_reused`): 토큰 삭제, 재로그인 안내. `invalid_client`: 구성 오류 안내
- 로그아웃: `POST https://auth.openai.com/api/accounts/oauth/revoke` (`token=<refresh>`, `token_type_hint=refresh_token`, `client_id`), 빈 200 성공. 네트워크 오류와 5xx는 백오프 재시도 2회 후 로컬 삭제와 "원격 폐기 미확인" 고지. client 매핑과 host ID 보존
- `logout --forget`: 등록 정보(`client_id`)까지 삭제, 다른 계정 신규 등록 허용. host ID는 유지

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-AUTH-001 | 11.1절 로그인 8단계 | PRD-AUTH-001, PRD-AUTH-002 | `src/auth/oauth.rs` |
| SRS-AUTH-002 | host ID 생성과 영구 유지 | PRD-AUTH-003 | `src/auth/state.rs` |
| SRS-AUTH-003 | 루프백 리스너: `127.0.0.1` 바인딩, Host 헤더와 경로 검사, 응답 HTML 1회 후 종료, 비정상 요청 404 | PRD-AUTH-001 | `src/auth/callback.rs` |
| SRS-AUTH-004 | ID 토큰 검증과 JWKS 조회 (`kid` 기준, 상태 파일 캐시 24시간) | PRD-AUTH-004 | `src/auth/idtoken.rs` |
| SRS-AUTH-005 | `--no-browser`, `--port`, SSH 포트 포워딩 안내 문구. TUI 로그인은 `HOWTUI_NO_BROWSER=1` 또는 SSH 세션(`SSH_CONNECTION`, `SSH_TTY`)에서 브라우저 미실행 | PRD-AUTH-005 | `src/auth/mod.rs`, `src/cli/provider.rs` |
| SRS-AUTH-006 | 리다이렉트 URL 붙여넣기 입력: `--no-browser` 또는 브라우저 실행 실패 시 루프백 대기와 터미널 입력 대기를 병행. 붙여넣은 URL은 콜백과 같은 검증(스킴, 호스트 `127.0.0.1`, 포트, 경로 `/auth/callback`, state) 후 코드 교환. 공백과 줄바꿈 제거, `127.0.0.1:`로 시작하는 주소는 `http://` 보완, TUI에서 `/auth/callback` 포함 붙여넣기는 Enter 없이 즉시 제출 (CR-002) | PRD-AUTH-006 | `src/auth/callback.rs`, `src/app/update.rs` |
| SRS-AUTH-007 | 11.2절 저장 구조, OS 보안 저장소 우선, 파일 대체 시 고지 | PRD-AUTH-008, PRD-PRIV-006 | `src/auth/store.rs` |
| SRS-AUTH-008 | 11.3절 갱신 조건과 잠금 기반 직렬화 | PRD-AUTH-009 | `src/auth/session.rs` |
| SRS-AUTH-009 | 11.3절 로그아웃, `--forget` | PRD-AUTH-010 | `src/auth/session.rs` |
| SRS-AUTH-010 | `auth status` 출력: 이메일 마스킹(첫 글자와 도메인만), 만료 시각, 플랜 사용 여부. 토큰 미출력 | PRD-AUTH-011 | `src/auth/status.rs` |
| SRS-AUTH-011 | 플랜 scope 미부여 시 `prompt=consent` 재인가 안내 | PRD-AUTH-012 | `src/auth/oauth.rs` |
| SRS-AUTH-012 | 토큰 문자열 타입은 `Debug`, `Display`에서 마스킹, 로그 기록 금지. `id_token_hint` 포함 URL 로그 금지 | PRD-PRIV-005 | `src/auth/secret.rs` |
| SRS-AUTH-013 | 로그인 전 개인정보 처리 고지 문구 표시 (CLI와 TUI) | PRD-AUTH-015, PRD-PRIV-003 | `src/auth/notice.rs` |
| SRS-AUTH-014 | 비공식 경로 미사용: `~/.codex` 경로 접근 코드 부재, `backend-api` 문자열 부재 (CI grep 검사) | PRD-AUTH-016 | 저장소 전체 |
| SRS-AUTH-015 | P4 자격증명 내보내기와 가져오기: 가져오기 대상 장치 host ID 유지, 원본 장치 사본 사용 중지 안내 | PRD-AUTH-007 | `src/auth/transfer.rs` |
| SRS-AUTH-016 | 일시 장애와 네트워크 오류에서 자격증명 보존 | PRD-AUTH-018 | `src/auth/session.rs` |
| SRS-AUTH-017 | 플랜 사용 불가 판정(scope 미부여, `subscription_sharing_user_not_eligible`, `subscription_sharing_v2_client_not_enabled`, 지역 403) 시 원인별 메시지 후 요청 중단, 대체 Provider 전환 없음 | PRD-AUTH-019 | `src/llm/chatgpt.rs`, `src/app/update.rs` |

## 12. 설정과 경로

### 12.1 경로

| 용도 | Linux, macOS | Windows | 재정의 |
| --- | --- | --- | --- |
| 설정 파일 | `$XDG_CONFIG_HOME/howtui/config.toml` (기본 `~/.config/howtui/config.toml`) | `%APPDATA%\howtui\config.toml` | `HOWTUI_CONFIG` (파일 경로) |
| 데이터 (상태, 자격증명 파일, 잠금) | `$XDG_DATA_HOME/howtui/` (기본 `~/.local/share/howtui/`) | `%LOCALAPPDATA%\howtui\` | `HOWTUI_DATA_DIR` |
| 캐시 (업데이트 확인, P4) | `$XDG_CACHE_HOME/howtui/` (기본 `~/.cache/howtui/`) | `%LOCALAPPDATA%\howtui\cache\` | 없음 |
| 로그 (선택) | 데이터 디렉터리 `logs/howtui.log` | 동일 | `HOWTUI_LOG` (수준), `HOWTUI_LOG_FILE` (경로) |

- macOS도 XDG 형식 경로 사용 (CLI 도구 관례, ADR-0014).

### 12.2 설정 스키마 (P1)

```toml
[ui]
mode = "inline"          # "inline" (P1), "fullscreen" (P3)
inline_height = 10       # 3-40
language = "auto"        # "auto", "en", "ko"

[provider]
default = "chatgpt"      # P1: "chatgpt"만 허용

[provider.chatgpt]
# model = "<모델 slug>"  # 선택. 미지정 시 계정 모델 목록 첫 항목
```

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-CFG-001 | 12.1절 경로 규칙과 재정의 환경변수 | PRD-CFG-001 | `src/config/paths.rs` |
| SRS-CFG-002 | 12.2절 스키마, 값 범위 검증 (`inline_height` 3-40, `language` 허용 값) | PRD-CFG-003 | `src/config/mod.rs` |
| SRS-CFG-003 | 파일 부재 시 기본값, 파일 생성 없음 | PRD-CFG-002 | `src/config/mod.rs` |
| SRS-CFG-004 | 파싱 오류 메시지에 경로, 행, 열, 키 포함, 종료 코드 8 | PRD-CFG-005 | `src/config/mod.rs` |
| SRS-CFG-005 | 알 수 없는 키 목록 수집 (실행 계속, doctor 경고) | PRD-CFG-006 | `src/config/mod.rs` |
| SRS-CFG-006 | 설정 스키마에 비밀값 필드 부재. P5 API 키는 OS 보안 저장소 또는 환경변수 이름 참조(`api_key_env = "OPENAI_API_KEY"`) | PRD-CFG-007 | `src/config/mod.rs` |
| SRS-CFG-007 | `ui.mode = "fullscreen"` 허용 (P3) | PRD-CFG-004 | `src/config/mod.rs` |
| SRS-CFG-008 | 데이터 디렉터리 생성 시 0700, 파일 0600 (Unix) | PRD-PRIV-006 | `src/platform/fs.rs` |

## 13. 플랫폼별 빌드와 릴리스

근거: [research/platform-build.md](research/platform-build.md), SPIKE-02 (ADR-0010, ADR-0011).

### 13.1 타깃별 빌드 방식

| 타깃 | 러너 | 빌드 방식 | 실행 검사 | 단계 |
| --- | --- | --- | --- | --- |
| x86_64-unknown-linux-gnu | ubuntu-24.04 | `cargo zigbuild --target x86_64-unknown-linux-gnu.2.17` | ubuntu-24.04 네이티브 | P1 |
| aarch64-unknown-linux-gnu | ubuntu-24.04 | `cargo zigbuild --target aarch64-unknown-linux-gnu.2.17` | ubuntu-24.04-arm 네이티브 | P1 |
| armv7-unknown-linux-gnueabihf | ubuntu-24.04 | `cargo zigbuild --target armv7-unknown-linux-gnueabihf.2.17` | qemu-user 10.x (4.2 사용 금지, SPIKE-02) | P4 (P1 권장) |
| aarch64-apple-darwin | macos-15 | Apple 툴체인, `MACOSX_DEPLOYMENT_TARGET=11.0` | macos-15 | P1 |
| x86_64-apple-darwin | macos-15 | 같은 러너에서 교차 빌드, `MACOSX_DEPLOYMENT_TARGET=11.0` | macos-15-intel | P4 (P1 권장) |
| x86_64-pc-windows-msvc | windows-2025 | 네이티브, `+crt-static` | windows-2025 | P1 |
| aarch64-pc-windows-msvc | windows-11-arm | 네이티브, `+crt-static` (ring 빌드에 clang 필요) | windows-11-arm | P4 (P1 권장) |

- cargo-zigbuild와 zig 버전 고정 (조사 시점 cargo-zigbuild 0.23.4, zig 0.17.0 조합 실측). 접미사 없는 zig 빌드는 glibc 2.30(x86_64, aarch64), 2.34(armv7) 요구로 Raspberry Pi OS Buster, Bullseye armhf 미지원 (research/platform-build.md 2절).
- macOS arm64는 링커 ad-hoc 서명. 공증(notarization)은 curl 설치 경로에서 불필요 판단, 브라우저 다운로드 사용자 안내만 제공.
- Windows 코드 서명은 1.x 범위 밖 결정 항목 (Smart App Control 강제 모드 차단 가능성 기록).

### 13.2 릴리스 프로필

```toml
[profile.release]
lto = "fat"
codegen-units = 1
strip = true
opt-level = 3
panic = "unwind"   # panic hook 이후 Drop 기반 터미널 복구 경로 유지
```

```toml
# .cargo/config.toml
[target.'cfg(all(target_env = "msvc", target_os = "windows"))']
rustflags = ["-C", "target-feature=+crt-static"]
```

### 13.3 릴리스 자산

| 자산 | 이름 규칙 | 내용 |
| --- | --- | --- |
| Unix 압축 파일 | `howtui-<version>-<target>.tar.gz` | 루트에 `howtui`, `README.md`, `LICENSE` |
| Windows 압축 파일 | `howtui-<version>-<target>.zip` | 루트에 `howtui.exe`, `README.md`, `LICENSE` |
| 체크섬 | `SHA256SUMS` | 모든 압축 파일과 설치 스크립트의 SHA-256 (`sha256sum` 형식) |
| 설치 스크립트 | `install.sh`, `install.ps1` | 해당 릴리스 버전 고정 사본 |

- 태그: `vX.Y.Z` (SemVer). 압축 파일명의 `<version>`은 `v` 없는 값.
- 릴리스 절차 (5단계 파이프라인): (1) 태그 푸시 (2) 타깃별 빌드와 실행 검사 (3) 바이너리 검사(ELF 아키텍처, GLIBC 심볼 최대 버전, macOS minos와 서명, PE 머신 타입) (4) 초안 릴리스에 전체 자산 업로드 (5) 사용자 승인 후 공개.
- 저장소 설정: 첫 릴리스 전 immutable releases 활성화 (공개 후 자산 교체 불가). 공개 후 `--clobber` 업로드 절차 미사용.
- 출처 증명 (P4): `actions/attest`로 `SHA256SUMS` 기준 증명 생성, 사용자 검증 명령 `gh attestation verify`, `gh release verify-asset` 문서화.

### 13.4 CI 구성

| 워크플로 | 시점 | 작업 |
| --- | --- | --- |
| `ci.yml` | PR, main 푸시 | fmt, clippy(`-D warnings`) ubuntu-24.04. `cargo test --locked --features mock-provider` ubuntu-24.04, ubuntu-24.04-arm, macos-15, windows-2025, windows-11-arm. armv7 `.2.17` 교차 빌드와 GLIBC 검사. 추적성 검사, grep 규칙 검사, 의존성 점검. 설치 스크립트 검사 (`sh -n`, shellcheck, PowerShell 5.1 `shell: powershell`과 7 `shell: pwsh` 구문 검사) |
| `release.yml` | `v*` 태그 | 13.1절 빌드, 13.3절 자산과 초안 릴리스 |

- 액션 버전은 커밋 SHA로 고정. 캐시는 `Swatinem/rust-cache` (main 브랜치에서만 저장).

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-BLD-001 | 13.1절 타깃별 러너와 빌드 방식 | PRD-REL-003, PRD-REL-004, PRD-PLAT-002 | `.github/workflows/release.yml` |
| SRS-BLD-002 | Linux GNU 타깃 glibc 2.17 접미사 빌드, cargo-zigbuild와 zig 버전 고정 | PRD-REL-004, PRD-PLAT-003 | `.github/workflows/` |
| SRS-BLD-003 | Windows MSVC `+crt-static` | PRD-PLAT-002 | `.cargo/config.toml` |
| SRS-BLD-004 | macOS `MACOSX_DEPLOYMENT_TARGET=11.0`, `codesign -v` 검사 | PRD-PLAT-002 | `.github/workflows/release.yml` |
| SRS-BLD-005 | 13.2절 릴리스 프로필 | PRD-REL-001 | `Cargo.toml` |
| SRS-BLD-006 | 13.3절 자산 명명과 `SHA256SUMS` | PRD-REL-002 | `.github/workflows/release.yml` |
| SRS-BLD-007 | 13.4절 CI 구성 | PRD-REL-001 | `.github/workflows/ci.yml` |
| SRS-BLD-008 | 태그 기반 초안 릴리스, 공개는 사용자 승인 후 수동 | PRD-REL-001, PRD-REL-007 | `.github/workflows/release.yml` |
| SRS-BLD-009 | 공개 전 바이너리 검사 단계 (ELF, GLIBC, minos, 서명, PE) | PRD-REL-003, PRD-PLAT-004 | `.github/workflows/release.yml` |
| SRS-BLD-010 | 출처 증명 생성과 검증 문서 (P4) | PRD-REL-006 | `.github/workflows/release.yml`, 문서 |
| SRS-BLD-011 | 릴리스 노트: 변경 사항, 설치 명령, 체크섬 검증 방법 (`release-notes` 스킬 형식) | PRD-REL-005 | `.github/release-notes/` |
| SRS-BLD-012 | 지원 상태 표: 타깃별 실기기 검증, 에뮬레이션 검증, 빌드만, 미검증 구분 | PRD-PLAT-004, PRD-PLAT-003 | `README.md`, `docs/` |

## 14. 설치 스크립트 설계

근거: [research/platform-build.md](research/platform-build.md) 3절, 5절, `cross-platform-cli-distribution` 스킬 기준 원고.

### 14.1 install.sh (Linux, macOS)

| 단계 | 처리 |
| --- | --- |
| 실행 환경 | POSIX `sh`, `set -eu`, bash 전용 문법 금지. 필수 명령 `curl`, `tar`, `uname`, SHA-256 도구(`sha256sum` 또는 `shasum -a 256`) |
| 옵션 | `--version <x.y.z>`, `--bin-dir <dir>`, `-h`. 환경변수 `HOWTUI_VERSION`, `HOWTUI_INSTALL_DIR` |
| 플랫폼 판별 | 아래 14.2절 표 |
| 버전 결정 | 지정 버전 우선. 미지정 시 `https://github.com/hungryZoo/howtui/releases/latest` 리다이렉트의 태그 추출, 실패 시 GitHub API |
| 다운로드 | 임시 디렉터리(`mktemp -d`)에 압축 파일과 `SHA256SUMS` 다운로드 (HTTPS만) |
| 검증 | `SHA256SUMS`에서 해당 파일 행 조회, 불일치 또는 행 부재 시 설치 중단 |
| 설치 | `~/.local/bin` 기본, `install -m 0755`로 교체 (재설치는 덮어쓰기) |
| 설치 후 확인 | 설치 경로의 `howtui --version` 출력과 버전 일치 확인 |
| PATH | 설치 디렉터리가 PATH에 없으면 추가 방법 출력 (rc 파일 미수정) |
| 정리 | `trap`으로 임시 디렉터리 삭제, 실패 시 기존 바이너리 유지 |

### 14.2 플랫폼 판별 표 (install.sh)

| 조건 | 선택 타깃 |
| --- | --- |
| `uname -s` = Darwin, 하드웨어 arm64 (`sysctl -n hw.optional.arm64` = 1, Rosetta 쉘 포함) | aarch64-apple-darwin |
| Darwin, 그 외 x86_64 | x86_64-apple-darwin |
| Linux, `uname -m` = x86_64, 사용자 공간 64비트 | x86_64-unknown-linux-gnu |
| Linux, `uname -m` = aarch64 또는 arm64, 사용자 공간 64비트 | aarch64-unknown-linux-gnu |
| Linux, `uname -m` = aarch64, 사용자 공간 32비트 (64비트 커널 + 32비트 OS, Pi 4, 400, CM4, 5 기본 구성) | armv7-unknown-linux-gnueabihf |
| Linux, `uname -m` = armv7l 또는 armv8l | armv7-unknown-linux-gnueabihf |
| Linux, `uname -m` = armv6l | 미지원 종료 (Pi 1, Zero, Zero W) |
| Linux, musl 사용자 공간 (`ldd --version` 출력에 musl) | 미지원 종료 (GNU 타깃 전용) |
| 그 외 | 미지원 종료 |

- 사용자 공간 비트 판별: `/proc/self/exe` (실패 시 `/bin/sh`) ELF 헤더 5번째 바이트(EI_CLASS, 1 = 32비트, 2 = 64비트). rustup 방식.
- 보조 확인: 동적 로더 존재 (`/lib/ld-linux-armhf.so.3`, `/lib/ld-linux-aarch64.so.1`), `getconf GNU_LIBC_VERSION` 2.17 미만 시 경고.

### 14.3 install.ps1 (Windows)

| 단계 | 처리 |
| --- | --- |
| 호환성 | Windows PowerShell 5.1과 PowerShell 7 공통 문법, ASCII 전용 파일, 오류는 `throw` (iex 실행 중 `exit` 미사용) |
| TLS | `[Net.ServicePointManager]::SecurityProtocol`이 SystemDefault(0)가 아니면 Tls12 비트 추가 (TLS 1.3 유지) |
| 다운로드 | `Invoke-WebRequest -UseBasicParsing` 고정 (2025-12-09 보안 업데이트 이후 5.1 대화 상자 대기 방지), `$ProgressPreference = 'SilentlyContinue'` |
| 아키텍처 | `HOWTUI_ARCH` 재정의 → HKLM `SYSTEM\CurrentControlSet\Control\Session Manager\Environment`의 `PROCESSOR_ARCHITECTURE` → `PROCESSOR_ARCHITEW6432` → `PROCESSOR_ARCHITECTURE`. ARM64 → aarch64, AMD64 → x86_64, 그 외 미지원 |
| 버전 결정 | `HOWTUI_VERSION` 우선, 미지정 시 `releases/latest/download/SHA256SUMS`의 파일명에서 버전 추출 |
| 검증 | `Get-FileHash -Algorithm SHA256` 결과(대문자)와 `SHA256SUMS` 값 대소문자 무시 비교, 불일치 시 중단 |
| 설치 | `%LOCALAPPDATA%\Programs\howtui\bin` 기본, `HOWTUI_INSTALL_DIR` 재정의. `Expand-Archive -Force` |
| PATH | HKCU `Environment`의 `Path`를 확장 없이 읽어 `ExpandString` 형식으로 기록, 환경 변경 브로드캐스트, 현재 세션 `$env:Path` 갱신. `setx` 미사용 (1024자 절단). `HOWTUI_NO_MODIFY_PATH=1` 시 안내만 출력 |
| 설치 후 확인 | `howtui.exe --version` 출력 확인 |
| 정리 | 임시 디렉터리 삭제 (`finally`) |

### 14.4 대안 설치와 업데이트, 제거

- 문서화 대상: (a) 스크립트 다운로드 후 내용 검토 뒤 실행 (b) 릴리스 페이지에서 압축 파일 직접 다운로드 (c) `sha256sum -c SHA256SUMS --ignore-missing` 또는 `Get-FileHash` 독립 검증 (d) 버전 고정 URL `https://raw.githubusercontent.com/hungryZoo/howtui/v<version>/scripts/install.sh` 및 릴리스 자산 `install.sh` (e) 브라우저 다운로드 macOS 사용자의 `xattr -d com.apple.quarantine` 안내
- 업데이트 (P4): 같은 설치 명령 재실행, 기존 설치 경로 유지 (`--bin-dir` 또는 `HOWTUI_INSTALL_DIR` 동일 값)
- 제거 (P4): 바이너리 삭제, 설정 디렉터리와 데이터 디렉터리 삭제, OS 보안 저장소 항목 삭제(`howtui logout` 선행 권장), rc 파일과 Profile의 init 줄 삭제

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-INS-001 | 14.1절 install.sh 구조와 POSIX 호환 | PRD-INS-001 | `scripts/install.sh` |
| SRS-INS-002 | 14.2절 플랫폼 판별 (사용자 공간 ELF 클래스 기준, armv6l과 musl 미지원 종료) | PRD-INS-003, PRD-INS-004 | `scripts/install.sh` |
| SRS-INS-003 | 버전 결정 (지정 우선, latest 리다이렉트, API 대체) | PRD-INS-001 | `scripts/install.sh` |
| SRS-INS-004 | `SHA256SUMS` 검증 필수, 다운로드 대상은 릴리스 자산 2개뿐, 다운로드 내용 실행 금지 | PRD-INS-001, PRD-INS-009 | `scripts/install.sh`, `scripts/install.ps1` |
| SRS-INS-005 | 사용자 범위 설치, sudo 미사용, rc 파일 미수정, PATH 안내 | PRD-INS-001, PRD-INS-005, PRD-SH-010 | `scripts/install.sh` |
| SRS-INS-006 | 설치 후 `--version` 확인 | PRD-INS-001 | `scripts/install.sh`, `scripts/install.ps1` |
| SRS-INS-007 | 임시 파일 정리와 실패 시 기존 바이너리 유지 | PRD-INS-001 | `scripts/install.sh`, `scripts/install.ps1` |
| SRS-INS-008 | 14.3절 install.ps1 구조 (5.1과 7 호환, TLS, 다운로드 규칙) | PRD-INS-002 | `scripts/install.ps1` |
| SRS-INS-009 | install.ps1 아키텍처 판별 순서 | PRD-INS-002, PRD-INS-004 | `scripts/install.ps1` |
| SRS-INS-010 | install.ps1 사용자 PATH 등록 규칙과 해제 옵션 | PRD-INS-002 | `scripts/install.ps1` |
| SRS-INS-011 | 14.4절 대안 설치 경로 문서 | PRD-INS-006 | `README.md` |
| SRS-INS-012 | 업데이트 절차 문서와 재실행 시 설치 경로 유지 | PRD-INS-007 | `README.md`, `scripts/` |
| SRS-INS-013 | 제거 절차 문서 | PRD-INS-008 | `README.md` |

## 15. doctor 설계

### 15.1 점검 항목

| 점검 ID | 항목 | 판정 | 단계 |
| --- | --- | --- | --- |
| version | howtui 버전, 빌드 타깃 | 정보 | P1 |
| os | OS 종류, 버전, CPU 아키텍처 | 정보 | P1 |
| terminal | `TERM`, `TERM_PROGRAM` 이름(값 공개 허용 목록), 크기 | 정보 | P1 |
| shell | 대상 쉘 판정 결과와 근거 | 정보, 경고(판정 실패) | P1 |
| tty | 제어 터미널 열기 가능 여부, stdin, stdout, stderr TTY 여부 | 통과, 실패 | P1 |
| locale | UTF-8 로캘 여부 (Unix `LC_ALL`, `LC_CTYPE`, `LANG`), Windows 콘솔 코드 페이지 | 통과, 경고 | P1 |
| render | 커서 위치 질의 응답 수신 (제한 시간 1초) | 통과, 실패 | P1 |
| config | 설정 파일 경로, 파싱 결과, 알 수 없는 키 | 통과, 경고, 실패 | P1 |
| auth | 로그인 여부, 플랜 사용 scope, 만료 시각, 저장소 종류 | 통과, 경고, 실패 | P1 |
| provider | 모델 목록 조회 성공 여부와 모델 수 (추론 요청 없음) | 통과, 실패 | P1 |
| integration | 쉘 통합 표식 환경변수 존재 여부 | 정보 | P2 |
| update | 최신 릴리스 버전 비교 | 정보 | P4 |
| local_llm | Ollama, llama-server 연결과 모델 상태 | 정보, 실패 | P6 |

### 15.2 JSON 스키마

```json
{
  "schema_version": 1,
  "howtui_version": "0.1.0",
  "checks": [
    { "id": "tty", "status": "pass", "summary": "...", "details": { } }
  ]
}
```

- `status`: `pass`, `warn`, `fail`, `info`, `skip`

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-DOC-001 | 15.1절 P1 점검 항목 | PRD-DOC-001, PRD-DOC-002, PRD-DOC-003 | `src/doctor/` |
| SRS-DOC-002 | 15.2절 JSON 스키마, `schema_version` 증가 규칙 (필드 삭제 시 증가) | PRD-DOC-004 | `src/doctor/json.rs` |
| SRS-DOC-003 | 마스킹 함수 단일화: 토큰 패턴 제거, 이메일 마스킹, 홈 경로 `~`, 환경변수 값 미출력 | PRD-DOC-005 | `src/doctor/mask.rs` |
| SRS-DOC-004 | 종료 코드 0 또는 1 | PRD-DOC-009 | `src/doctor/mod.rs` |
| SRS-DOC-005 | 쉘 통합 점검 (P2), 업데이트 점검 (P4), 로컬 LLM 점검 (P6) | PRD-DOC-006-008 | `src/doctor/` |
| SRS-DOC-006 | 점검 실행 중 네트워크 요청은 모델 목록 조회 1회와 (P4) 릴리스 확인 1회로 한정 | PRD-DOC-003 | `src/doctor/` |

## 16. 안전 검사

### 16.1 규칙 표 (초안)

| 범주 | 수준 | Bash, Zsh 패턴 | PowerShell 패턴 |
| --- | --- | --- | --- |
| 재귀 삭제 | 높음 | `rm` + (`-r`, `-R`, `--recursive`) 또는 `-f` 조합, `find ... -delete`, `find ... -exec rm` | `Remove-Item` + `-Recurse`, `rm`/`del`/`rd`/`rmdir` + `/s` |
| 디스크 초기화, 직접 쓰기 | 높음 | `mkfs*`, `dd ... of=/dev/`, `wipefs`, `shred`, `> /dev/sd*` | `Format-Volume`, `Clear-Disk`, `Initialize-Disk` |
| 권한 변경 (재귀) | 높음 | `chmod -R`, `chown -R`, `chmod 777 /` 계열 | `Set-Acl`, `icacls ... /grant` + `/t` |
| 원격 스크립트 실행 | 높음 | `curl`/`wget` 출력의 `sh`, `bash`, `zsh` 파이프 | `iex`/`Invoke-Expression` + `irm`/`iwr`/`Invoke-WebRequest` |
| 대량 프로세스 종료 | 높음 | `kill -9 -1`, `killall`, `pkill` | `Stop-Process` + 와일드카드 또는 `-Name *` |
| 시스템 전원, 서비스 | 주의 | `shutdown`, `reboot`, `systemctl (stop\|disable\|mask)` | `Stop-Computer`, `Restart-Computer`, `Stop-Service` |
| 권한 상승 | 주의 | `sudo`, `su`, `doas` | `Start-Process -Verb RunAs` |
| 덮어쓰기 | 주의 | `>` 리디렉션(`>>`, `/dev/null` 제외), `mv -f`, `cp -f`, `truncate` | `Set-Content`, `Out-File` (`-Append` 제외), `>` |
| 이력 손실 (git) | 주의 | `git reset --hard`, `git clean -fd`, `git push --force`, `git push -f` | 동일 |
| 시스템 설정 | 주의 | `/etc/` 경로 쓰기, `crontab -r`, `iptables -F` | `Set-ExecutionPolicy`, `reg delete`, `Remove-ItemProperty HKLM:` |
| fork bomb | 높음 | `:(){ :\|:& };:` 형태 | 해당 없음 |

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-SAFE-001 | 16.1절 규칙 표 구현, 판정 결과 `Finding { level, category, reason, span }` | PRD-SAFE-002, PRD-SAFE-003 | `src/safety/` |
| SRS-SAFE-002 | 토큰화: 따옴표 내부 문자열은 명령 인자로 취급, 연결 연산자 기준 명령 단위 분리, 정규식 엔진 미사용 (문자열 비교) | PRD-SAFE-002 | `src/safety/lexer.rs` |
| SRS-SAFE-003 | 판정 시점: 응답 반영 직후, 직접 편집 후 300ms 유휴 시, 확정 직전 | PRD-SAFE-002, PRD-SAFE-003 | `src/app/update.rs` |
| SRS-SAFE-004 | `src/`의 직접 프로세스 실행 API(`std::process::Command`) 사용 0곳. 브라우저 실행은 `open::that_detached` 1곳 (인가 URL 인자만 전달). 쉘 버전은 쉘 통합 스크립트가 넘기는 환경변수로만 수집. AI 생성 문자열을 프로세스 인자로 전달 금지 (`scripts/ci/check_rules.sh`) | PRD-SAFE-001, PRD-SAFE-006 | 저장소 전체 |
| SRS-SAFE-005 | 규칙 테스트는 문자열 입력만 사용, 실제 실행 금지 | PRD-SAFE-007 | `src/safety/tests.rs` |
| SRS-SAFE-006 | 도움말과 README에 한계 고지 문구 | PRD-SAFE-004 | `src/cli/`, `README.md` |
| SRS-SAFE-007 | 모델 `notes`는 설명 영역 하단 표시, `Finding` 수준 계산 미반영 | PRD-SAFE-005 | `src/ui/inline.rs` |

## 17. 보안 요구사항

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-SEC-001 | TLS 인증서 검증 비활성화 옵션 미제공 | PRD-AUTH-001 | `src/llm/`, `src/auth/` |
| SRS-SEC-002 | 시스템 프록시 환경변수(`HTTPS_PROXY`, `NO_PROXY`) 존중 | PRD-LLM-001 | `src/llm/http.rs` |
| SRS-SEC-003 | 자격증명과 상태 파일 쓰기는 임시 파일 + 이름 변경 원자적 처리, Unix 0600 | PRD-PRIV-006 | `src/platform/fs.rs` |
| SRS-SEC-004 | LLM 전송 페이로드 구성 함수 단일화, 허용 필드 외 컨텍스트 추가 금지 (단위 테스트로 필드 목록 고정) | PRD-PRIV-001, PRD-PRIV-002 | `src/llm/prompt.rs` |
| SRS-SEC-005 | 의존성 취약점 점검 (`cargo audit` 또는 `cargo deny`) CI 실행 | PRD-REL-001 | `.github/workflows/` |
| SRS-SEC-006 | 설치 스크립트는 HTTPS URL만 사용, 체크섬 불일치 시 설치 중단 | PRD-INS-001, PRD-INS-009 | `scripts/` |
| SRS-SEC-007 | 쉘 입력란 내용과 사용자 편집 명령어는 사용자 요청 제출 시에만 페이로드 포함. 시작 시점 자동 전송 없음, 로그인 전 고지와 `--help`에 명시 | PRD-PRIV-004 | `src/app/update.rs`, `src/llm/prompt.rs` |

## 18. 성능 요구사항

잠정 목표. Phase 1 측정 후 확정 (MRD-KPI-004).

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-PERF-001 | 실행부터 첫 화면 표시까지 x86_64 기준 100ms 이하 (네트워크 대기 없음) | PRD-UI-001 | 측정 스크립트 |
| SRS-PERF-002 | 유휴 상태 CPU 사용 거의 없음 (요청 없을 때 타이머 정지) | PRD-UI-007 | `src/runtime.rs` |
| SRS-PERF-003 | 릴리스 바이너리 크기 기록 (목표치 Phase 1 측정 후 확정) | PRD-REL-001 | CI |
| SRS-PERF-004 | Raspberry Pi 2 (ARMv7) 기준 첫 화면 표시 시간 측정과 기록 | PRD-PLAT-003 | Phase 4 |

## 19. 호환성 제약

| ID | 제약 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-CMP-001 | Linux glibc 하한 2.17 (SPIKE-02 실측, 최종 의존성 기준 CI 재측정). C 의존성 링크 실패 시에만 2.28까지 상향 허용 (Raspberry Pi OS Buster 2.28) | PRD-PLAT-003, PRD-REL-004 | CI |
| SRS-CMP-002 | 쉘 하한: Bash 4.0, PSReadLine 2.0.0 (Windows PowerShell 5.1 기본 탑재), Zsh 하한은 Phase 2 실측 후 확정 | PRD-SH-001, PRD-SH-002, PRD-SH-003 | `src/shell/scripts/` |
| SRS-CMP-003 | 터미널: VT100 계열 제어 시퀀스와 커서 위치 질의 응답 지원 필요. 응답 제한 시간 2초 (crossterm 기본값, 고지연 SSH에서 초과 가능). 무응답 시 TUI 미시작, 터미널 복구 후 종료 코드 3 | PRD-UI-001, PRD-OUT-007 | `src/ui/terminal.rs` |
| SRS-CMP-004 | Windows: VT 처리 지원 콘솔 (Windows 10 이상) | PRD-PLAT-001 | `src/platform/tty.rs` |
| SRS-CMP-005 | macOS 최소 버전 11.0 (두 타깃 공통, Xcode 26의 지원 하한 기준) | PRD-PLAT-002 | `.github/workflows/release.yml` |
| SRS-CMP-006 | 지원 범위 외 명시: Fish, Nushell 등 기타 쉘, ble.sh 사용 Bash, musl 기반 Linux, ARMv6 | PRD-PLAT-001, PRD-SH-012 | `README.md` |

## 20. 테스트 전략

| 계층 | 도구 | 대상 | CI |
| --- | --- | --- | --- |
| 단위 | `cargo test` | `app::update`, keymap, 편집 버퍼, 파서, SSE, 안전 규칙, 설정, 마스킹, 오류 매핑 | 모든 OS |
| 렌더링 | ratatui `TestBackend` 버퍼 행 단위 검사 | Inline 화면 (Fullscreen은 P3) | 모든 OS |
| 통합 (프로세스) | `assert_cmd` 바이너리 실행 (Mock Provider, wiremock) | 비대화형 모드, 제어 터미널 없음, 종료 코드, stdout 순수성, ChatGPT 경로 | 모든 OS |
| PTY E2E | Unix: `tests/pty/run.sh`(tmux), `tests/pty/no_dsr.py`(무응답 PTY). Windows: `tests/conpty.rs`(portable-pty ConPTY) | TUI 입력, 취소, 리사이즈, 복구, stdout 분리 | Linux, macOS, Windows |
| 실연동 | 실제 ChatGPT 계정 | OAuth 전체 흐름, 추론, 갱신, 폐기 | 수동, CI 제외 |
| 실기기 | Raspberry Pi, Windows ARM64 | 설치, 실행 | 수동 |

| ID | 설계 요구사항 | 관련 PRD | 구현 위치 |
| --- | --- | --- | --- |
| SRS-TEST-001 | 외부 인증 필요 테스트는 기본 `cargo test`에서 제외 (`#[ignore]` + 환경변수 게이트) | PRD-LLM-012 | `tests/` |
| SRS-TEST-002 | 네트워크 단위 테스트는 로컬 Mock HTTP 서버 사용 | PRD-LLM-012 | `tests/` |
| SRS-TEST-003 | 위험 명령어 테스트는 문자열 판정만 수행 | PRD-SAFE-007 | `src/safety/` |
| SRS-TEST-004 | 테스트 결과는 TC 문서 결과 열에 기록, 미실행은 "미실행"으로 표기. 쉘과 플랫폼 지원 표기는 TC 결과 기준 | PRD-SH-012, PRD-PLAT-004 | `docs/TC.md` |
| SRS-TEST-005 | 추적성 검사 스크립트(`scripts/trace_check.py`)로 MRD, PRD, SRS, TC 연결 검사, CI 실행 | PRD-REL-001 | `scripts/trace_check.py` |
| SRS-TEST-006 | PTY E2E 하네스: Unix tmux 스크립트(CI Linux, macOS), Windows portable-pty ConPTY 테스트(`ESC[6n` 질의 자동 응답) | PRD-UI-010, PRD-OUT-006 | `tests/pty/`, `tests/conpty.rs` |

## 21. ADR 목록

| ADR | 제목 | 상태 |
| --- | --- | --- |
| [0001](adr/0001-single-crate-with-module-boundaries.md) | 단일 crate 구조와 모듈 경계 | 제안 |
| [0002](adr/0002-tui-output-to-controlling-terminal.md) | TUI 출력 대상: 제어 터미널과 fd 1 임시 전환 | 제안 (Unix 실측) |
| [0003](adr/0003-stdout-contract-and-exit-codes.md) | stdout 계약과 종료 코드 | 제안 |
| [0004](adr/0004-state-update-effect-and-renderer-separation.md) | 상태 갱신 구조와 렌더러 분리 | 제안 |
| [0005](adr/0005-async-runtime-and-cancellation.md) | 비동기 실행과 요청 취소 | 제안 |
| [0006](adr/0006-provider-interface-and-standard-response.md) | LLM Provider 인터페이스와 표준 응답 | 제안 |
| [0007](adr/0007-chatgpt-sign-in-provider.md) | 최초 Provider: Sign in with ChatGPT | 제안 (조건부) |
| [0008](adr/0008-credential-storage.md) | 자격증명 저장: OS 보안 저장소 우선, 0600 파일 대체 | 제안 |
| [0009](adr/0009-shell-integration-protocol.md) | 쉘 통합 프로토콜: 환경변수 입력, 출력 파일 결과 | 제안 (부분 실측) |
| [0010](adr/0010-linux-gnu-targets-with-glibc-floor.md) | Linux GNU 타깃과 glibc 2.17 하한 | 제안 (실측) |
| [0011](adr/0011-release-assets-and-checksums.md) | 릴리스 자산 명명과 체크섬 검증 | 제안 |
| [0012](adr/0012-rule-based-danger-warnings.md) | 규칙 기반 위험 명령어 경고 | 제안 |
| [0013](adr/0013-tls-rustls-ring.md) | TLS 백엔드: rustls + ring | 제안 (교차 빌드 실측) |
| [0014](adr/0014-config-and-data-paths.md) | 설정과 데이터 경로 | 제안 |
| [0015](adr/0015-own-text-buffer.md) | 자체 편집 버퍼 구현 | 제안 |
| [0016](adr/0016-i18n-message-catalog.md) | 다국어 메시지 카탈로그 | 승인 (E-03) |

## 22. 미확정 설계 항목

| 항목 | 확정 시점 | 관련 |
| --- | --- | --- |
| ChatGPT 경로 구조화 출력 지원 여부 | 해소: 실계정 `state.json`의 `structured_output = true` (2026-10-22) | SRS-LLM-005 |
| 리다이렉트 URL 붙여넣기 허용 여부 | 해소: 실계정 붙여넣기 로그인 성공 (사용자 확인, 2026-10-22) | SRS-AUTH-006 |
| 신규 동적 등록 앱 활성화 여부, 한국 지역 허용 여부 | 해소: 한국 지역 실계정 로그인과 명령어 생성 성공 (사용자 확인, 2026-10-22) | ADR-0007 |
| Windows 콘솔 출력 핸들 선택과 stdout 리디렉션 병행 | 부분 해소: ConPTY 자동 E2E 3개 경로 통과 (CI windows-2025, windows-11-arm). Windows Terminal, conhost 대화형 확인과 한국어 코드 페이지 환경은 수동 확인 대기 | SRS-TERM-003 |
| macOS `/dev/tty`와 fd 1 전환 동작 | 해소: CI macos-15 PTY E2E 18건 통과 (2026-10-22) | SRS-TERM-002 |
| 한국어 IME 조합 입력과 raw mode 상호작용 | Phase 1 실측 | SRS-TUI-011 |
| ratatui 가로 축소 시 전체 화면 정리 동작 (0.30.2, main 브랜치 수정 미릴리스) | ratatui 차기 릴리스 | SRS-TERM-008 |
| Zsh 지원 하한 버전 | 부분 해소: 5.9 PTY E2E 통과 (Debian 12 컨테이너, CI). 5.1-5.8 미검증 | SRS-CMP-002 |
| PowerShell 한국어 코드 페이지 환경 실제 값과 동작 | Phase 2 Windows 실측 | SRS-SH-004 |
