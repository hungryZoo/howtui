# 경쟁 도구 조사 보고서

## 0. 조사 개요

### 0.1 조사 정보

- 조사일: 2026-10-22
- 조사 목적: howtui(Rust 기반 AI 터미널 명령 도우미) 계획 기능 대비 경쟁 도구 현황 파악
- 조사 방법
  - 각 저장소 기본 브랜치 shallow clone 후 소스 열람(grep, cat). 대상 코드의 빌드와 실행 없음.
  - 릴리스와 자산 목록: `gh release view`, `gh api` 기준
  - 공식 문서: 저장소 내 문서 원본과 공식 문서 사이트(WebFetch) 대조
- 근거 표기
  - 확인(소스): 클론 소스의 파일 경로와 행 번호 기준
  - 확인(문서): README, 공식 문서 사이트, 릴리스 메타데이터, 이슈 기록 기준
  - 미확인: 실행 검증 없음 또는 공개 근거 부재. 소스 판독 기반 추정은 "미확인(추정)" 표기
- 행 번호 기준: 아래 표의 조사 기준 커밋

### 0.2 조사 기준 커밋과 태그

| 프로젝트 | 조사 기준 커밋(기본 브랜치 HEAD) | 최신 릴리스 또는 태그 |
|---|---|---|
| antonmedv/howto | `d190fa1d3d18b13a28f9c874c113b221359b3e68` (2026-01-29) | GitHub Release `0.0.0` (2025-02-12), 태그 커밋 `7acdaf90d7ea97829cb67d30e884128829b56c80` |
| noahhefner/how-tui | `395805eadcc936c0a23c85c213cd718401a50d70` (2026-09-25) | PyPI `1.0.3` (2026-09-25). GitHub Release 없음 |
| atuinsh/atuin | `92a8b9ae1d0bf5fff3e7a2378175b7774001fc0a` (2026-10-21) | `v18.24.1` (2026-10-16), 태그 커밋 `c58d2e993d05be423315f6ae3739e9ab3e45eea5` |
| atuinsh/eye-declare (Atuin AI TUI 의존 라이브러리, 보조 확인) | `e14258bc0965e61e76fa94e39455699bd676eeb5` (2026-08-10) | crates.io `0.7.1` |

- Atuin 열람 기준: HEAD. `v18.24.1` 대비 `crates/atuin-ai` 변경 13개 파일(추가 192행, 삭제 117행).
  - 셸 통합(`crates/atuin-ai/src/shell.rs`) 변경 범위: 종료 코드 오류 처리 추가, 결과 반환 방식 동일. 확인(소스): `git diff v18.24.1 HEAD`

---

## 1. antonmedv/howto

### 1.1 기본 정보

| 항목 | 내용 | 근거 |
|---|---|---|
| 언어, 스택 | Go 1.23, 외부 의존성 `golang.org/x/term` 1종 | 확인(소스): `go.mod:3-7` |
| 소스 규모 | Go 파일 7개(main, prompt, providers, security, utils, ioctl_unix, ioctl_windows) | 확인(소스) |
| 라이선스 | MIT | 확인(소스): `LICENSE:1-3` |
| 최신 릴리스 | `0.0.0` (2025-02-12), 유일한 릴리스 | 확인(문서): `gh release list -R antonmedv/howto` |
| 활동성 | 전체 커밋 10건. 2025-02-23 이후 변경은 2026-01-29 PR #4(다중 제공자 추가) 1건. 별 121, 포크 5. 열린 이슈와 PR 4건(#1, #2, #3, #5) | 확인(문서): `gh api repos/antonmedv/howto/commits`, `issues` |

### 1.2 UI 형태와 편집, 후속 수정, 설명 기능

- UI 형태: TUI 없음. 셸 입력 줄 자체를 자연어 입력란으로 사용하는 방식.
  - Zsh `bindkey -s "\C-g" "\C-ahowto \C-j"` 문자열 매크로: 줄 맨 앞 이동, `howto ` 삽입, 개행 입력으로 `howto <입력 줄>` 명령 실행. 확인(문서): `README.md:25-39`
  - 프로세스 흐름: 명령행 인자 결합, LLM 질의 1회, 결과 입력 주입. 확인(소스): `main.go:19, 34, 40`
- 편집 기능: howto 자체 편집 UI 없음. 주입된 명령을 셸 라인 에디터에서 편집하는 구조. 확인(문서): `README.md:64-66`
- 후속 수정 기능: 대화 문맥 없음. 셸 히스토리에서 이전 `howto ...` 줄을 다시 불러 질의를 고쳐 재실행하는 방식. 확인(문서): `README.md:68`, `.github/images/demo.tape`. 매 호출 단일 user 메시지 전송. 확인(소스): `providers.go:63-67`
- 설명 기능: 없음. 프롬프트에 "Output only the command" 지시. 확인(소스): `prompt.go:16-17`
- 출력 정리: 앞뒤 공백과 백틱 제거, 개행을 공백으로 치환. 확인(소스): `security.go:7-11`. 다중 행 명령의 한 줄 평탄화 결과.

### 1.3 LLM 제공자와 인증 방식

- HEAD 기준 제공자 3종: OpenAI(`gpt-4o`), Gemini(`gemini-2.0-flash`, OpenAI 호환 엔드포인트), DeepSeek(`deepseek-chat`). 확인(소스): `providers.go:18-34`
- 인증: 환경 변수 API 키(`OPENAI_API_KEY`, `GEMINI_API_KEY`, `DEEPSEEK_API_KEY`), 복수 설정 시 OpenAI, Gemini, DeepSeek 순 우선. Bearer 헤더 전송. 확인(소스): `main.go:9-16, 43-50`, `providers.go:79`
- 모델 재정의: `HOWTO_MODEL` 환경 변수. 확인(소스): `main.go:32`
- 엔드포인트 하드코딩, 사용자 지정 base URL 없음, 로컬 모델 미지원, OAuth 없음. 확인(소스): `providers.go:21, 26, 31`. 사용자 지정 URL 추가 PR #3(2025-05-26) 미병합. 확인(문서)
- 요청 제한값: HTTP 타임아웃 30초, `max_tokens` 1000. 확인(소스): `providers.go:66, 81`
- 릴리스 바이너리와 HEAD 차이
  - `0.0.0` 태그 커밋 트리: `openai.go`만 존재, `OPENAI_API_KEY` 미설정 시 panic. 확인(소스): `gh api repos/antonmedv/howto/contents/main.go?ref=7acdaf90d7ea`, 해당 판 `main.go:16-19`
  - 이슈 #5(2026-06-24): Gemini 키만 설정한 Windows 환경에서 `panic: OPENAI_API_KEY not set` 보고. 확인(문서)
  - 결론: 릴리스 바이너리는 OpenAI 전용, 다중 제공자는 `go install ...@latest` 경로 전용. proxy.golang.org `@latest` 응답 `v0.0.0-20260129124455-d190fa1d3d18`(HEAD). 확인(문서)

### 1.4 지원 OS, 셸과 셸 통합 방식

- 빌드 대상: linux, darwin, windows 각각 amd64, arm64. 확인(소스): `.github/scripts/build.mjs:5-13`
- Unix 계열 반환 방식: `TIOCSTI` ioctl로 결과 문자열을 터미널 입력 큐에 1바이트씩 주입. 확인(소스): `ioctl_unix.go:13-24`(ioctl 호출 22행)
  - 셸 위젯 API(Bash `READLINE_LINE`, Zsh `BUFFER`, PSReadLine) 미사용. 셸 종류와 무관한 터미널 수준 주입.
  - rune 값을 `byte`로 변환하여 주입(`ioctl_unix.go:20-22`). 비 ASCII 문자 손상 여부 미확인(추정).
- Windows 반환 방식: `fmt.Println` stdout 출력만 수행. 확인(소스): `ioctl_windows.go:9-11`. PowerShell 통합 없음.
- 문서상 셸 설정: Zsh 1종만 기재. Bash, Fish, PowerShell 설정 문서 없음. 확인(문서): `README.md:23-40`
- Linux TIOCSTI 제약
  - 커널 커밋 `83efeeeb3d04`("tty: Allow TIOCSTI to be disabled")로 `CONFIG_LEGACY_TIOCSTI`와 sysctl `dev.tty.legacy_tiocsti` 도입. v6.1 미포함, v6.2 포함. 확인(문서): `gh api repos/torvalds/linux/compare`
  - v6.2 기준 비활성 시 모든 호출자의 TIOCSTI `-EIO` 반환, 업스트림 Kconfig 기본값 `y`. 확인(소스): torvalds/linux v6.2 `drivers/tty/tty_io.c:2272-2281`, `drivers/tty/Kconfig:152-168`(gh api 열람). 이후 커널의 예외 조건 변경 여부 미확인
  - 이슈 #2: Arch Linux에서 출력 없음 보고, 작성자의 최신 커널 TIOCSTI 비활성 원인 확인 답변과 hstr식 셸별 우회(`READLINE_LINE`, `BUFFER`) 제안. 해당 우회 미구현. 확인(문서), 확인(소스)
  - 배포판별 기본값: 미확인
- 셸 정보 전달: `runtime.GOOS` 값만 전달(darwin은 mac 변환), 셸 종류 미전달. 확인(소스): `utils.go:7-14`, `prompt.go:15`

### 1.5 자동 실행 여부와 안전 기능

- 자동 실행: 없음. 입력 주입 후 사용자 Enter로 실행. 확인(문서): `README.md:64-66`. 소스 내 프로세스 실행 호출 없음. 확인(소스)
- 개행 공백 치환으로 주입 시 개행에 의한 실행 방지. 확인(소스): `security.go:10`
- `\r` 등 기타 제어 문자 미처리. 확인(소스): `security.go:7-11`. 주입 시 실행 영향 미확인.
- 위험 명령 경고: 없음. 확인(소스)
- Zsh 매크로 구조상 자연어 입력이 `howto` 명령 인자로 셸 해석 대상. 리다이렉션, 파이프, 따옴표 포함 질의의 셸 해석 문제 미확인(추정).

### 1.6 배포 방식과 릴리스 대상

- 설치 방법: `go install github.com/antonmedv/howto@latest`, GitHub Release 바이너리 수동 다운로드. 확인(문서): `README.md:18, 21`
- `0.0.0` 릴리스 자산 6종: `howto_darwin_amd64`, `howto_darwin_arm64`, `howto_linux_amd64`, `howto_linux_arm64`, `howto_windows_amd64.exe`, `howto_windows_arm64.exe`. 확인(문서): `gh release view 0.0.0 -R antonmedv/howto`
  - ARMv7: 없음
  - ARM64: Linux, macOS, Windows 모두 있음
  - 체크섬 파일, 설치 스크립트, 패키지 매니저 배포: 없음. 확인(문서)
- 빌드 방식: CI 없이 로컬 zx 스크립트 빌드 후 `gh release upload`. 확인(소스): `.github/scripts/build.mjs:17-30`

### 1.7 LLM 전송 컨텍스트

- 전송 항목: 질의 문자열(입력 줄 전체), OS 이름. 확인(소스): `prompt.go:7-18`
- 미전송 항목: 환경 변수, 셸 히스토리, 현재 디렉터리, 파일 내용. 확인(소스)

---

## 2. noahhefner/how-tui

### 2.1 기본 정보

| 항목 | 내용 | 근거 |
|---|---|---|
| 언어, 스택 | Python 3.14 이상. 의존성 google-genai, groq, keyring, platformdirs, pydantic, questionary, rich | 확인(소스): `pyproject.toml:9-18`, `.python-version` |
| 실행 명령 | `how` | 확인(소스): `pyproject.toml:32-33` |
| 라이선스 | MIT | 확인(소스): `LICENSE.md:1-7`, `README.md:156-158` |
| 최신 배포 | PyPI 1.0.0 (2026-08-21), 1.0.1 (2026-08-27), 1.0.2 (2026-09-07), 1.0.3 (2026-09-25). GitHub Release 없음 | 확인(문서): PyPI JSON API, `gh release list` |
| 태그 상태 | 태그 v1.0.2-v1.0.7 존재. v1.0.7 커밋 `a5cce1b`("Test release 4")는 main과 갈라진 이력(ahead 6, behind 11) | 확인(문서): `gh api repos/noahhefner/how-tui/compare` |
| 배포본 일치 | PyPI 1.0.3 wheel의 `how_tui/` 내용과 HEAD `src/how_tui/` 내용 동일(diff 차이 없음) | 확인(소스) |
| 활동성 | 2026-08-18 생성, 커밋 25건, 마지막 커밋 2026-09-25, 별 0, 포크 0. 열린 이슈 #5(LLM 요청 타임아웃 추가) | 확인(문서) |
| CI | Woodpecker 수동 트리거 `uv build`, `uv publish` | 확인(소스): `.woodpecker.yml:1-14` |

### 2.2 UI 형태와 편집, 후속 수정, 설명 기능

- UI 형태: 명칭과 달리 TUI 없음. 단발 CLI 출력 방식.
  - `how "질문"` 실행, rich 스피너 표시, "Command suggestions:" 목록 출력 후 종료. 확인(소스): `src/how_tui/cli.py:202-207, 377-388`
  - 제공자 설정 단계 한정 questionary 선택 메뉴. 확인(소스): `cli.py:87-89, 116-118, 173-175, 196`
- 복수 후보 제시: 프롬프트 규칙 "Return several command options". 확인(소스): `cli.py:25`
- 편집 기능: 없음(목록 출력만 수행, 선택 UI 없음). 확인(소스)
- 후속 수정 기능: 없음(단일 요청). 확인(소스): `cli.py:378-380`
- 설명 기능: 없음. 응답 스키마 `command` 단일 필드, 추가 필드 금지(`extra="forbid"`). 확인(소스): `src/how_tui/models/command.py:4-11`
- 소스 판독 기반 결함 후보
  - 안내 문구의 `how --setup` 플래그 미정의(실제 플래그 `--add-provider`). 확인(소스): `cli.py:61, 81, 107, 335`
  - 기본 설정 파일 부재 시 `ConfigManager`의 `ConfigError` 발생, 해당 생성이 `--add-provider` 처리보다 앞선 구조. 최초 설정 실패 가능성 미확인(추정). 근거: `cli.py:289-296`, `config.py:47-56`, `tests/test_config_missing.py:34-51`

### 2.3 LLM 제공자와 인증 방식

- 제공자 2종: Gemini, Groq. 확인(소스): `src/how_tui/providers/__init__.py:5-8`
  - Gemini: google-genai `client.interactions.create`, JSON 스키마 응답 지정. 확인(소스): `providers/gemini.py:34-40`
  - Groq: `chat.completions.create`, `json_schema` strict 응답 지정. 확인(소스): `providers/groq.py:26-39`
- 인증: `getpass` 입력 API 키의 OS keyring 저장(서비스명 `how-tui`). 확인(소스): `gemini.py:67-75`, `groq.py:51-60`. 헤드리스 환경의 서드파티 keyring 백엔드 필요. 확인(문서): `README.md:26-27`
- 모델 선택: 제공자 API 모델 목록 조회 후 선택, `--provider`, `--model` 플래그 재정의. 확인(소스): `gemini.py:98-102`, `groq.py:73`, `cli.py:257-265`
- 미지원: OpenAI, Anthropic, 로컬 모델, OAuth. 확인(소스): `providers/__init__.py:5-8`
- 설정 파일: platformdirs 사용자 설정 경로의 `config.json`. 확인(소스): `config.py:49-51`

### 2.4 지원 OS, 셸과 셸 통합 방식

- 셸 통합: 없음. 결과의 stdout 목록 출력이 유일한 반환 경로. 확인(소스): `cli.py:202-207`
- OS, 셸 감지: `platform.system()`, `$SHELL` 환경 변수 값의 프롬프트 삽입. 확인(소스): `cli.py:283-284, 370-374`
- OS 지원 범위: 순수 Python wheel(`py3-none-any`). 플랫폼별 실행 검증 미확인.

### 2.5 자동 실행 여부와 안전 기능

- 자동 실행: 없음. 확인(소스)
- 위험 명령 경고: 없음. 프롬프트 규칙(명령 창작 금지, 비공통 소프트웨어 가정 금지)만 존재. 확인(소스): `cli.py:20-27`

### 2.6 배포 방식과 릴리스 대상

- 설치 방법: `uv tool install how-tui`. 확인(문서): `README.md:20-24`
- 배포 산출물: PyPI wheel(`py3-none-any`)과 sdist. 네이티브 바이너리 없음, ARMv7, ARM64, Windows ARM64 구분 없음. 확인(문서): PyPI JSON API
- 런타임 요구: Python 3.14 이상. 확인(소스): `pyproject.toml:9`

### 2.7 LLM 전송 컨텍스트

- 전송 항목: 질의 문자열, OS 이름, `$SHELL` 경로. 확인(소스): `cli.py:14-37, 370-374`
- 미전송 항목: 셸 히스토리, 현재 디렉터리, 파일 내용, 기타 환경 변수. 확인(소스)

---

## 3. atuinsh/atuin

### 3.1 기본 정보

| 항목 | 내용 | 근거 |
|---|---|---|
| 언어, 스택 | Rust(워크스페이스 버전 18.24.1, MSRV 1.95.0), 크레이트 14개. AI 기능 크레이트 `atuin-ai`(Rust 약 44,000행) | 확인(소스): `Cargo.toml:1-14`, `crates/atuin-ai/` |
| AI TUI 스택 | ratatui 0.30 기반 eye_declare 0.7(Atuin 팀 제작 인라인 TUI 라이브러리), crossterm 0.29, tui-textarea-2 | 확인(소스): `Cargo.toml`, `Cargo.lock`, `crates/atuin-ai/Cargo.toml` |
| 라이선스 | MIT | 확인(소스): `LICENSE:1-3` |
| 최신 릴리스 | `v18.24.1` (2026-10-16). 직전 정식 릴리스 18.24.0 (2026-10-15), 18.23.0 (2026-09-22), 18.22.0 (2026-09-09) | 확인(문서): `gh release list -R atuinsh/atuin` |
| AI 기능 도입 | 18.13.0 (2026-03-14) changelog의 "Add Atuin AI inline CLI MVP", "Generate commands or ask questions with `atuin ai`" | 확인(소스): `CHANGELOG.md:1062-1113` |
| 활동성 | 기본 브랜치 커밋 191건(2026-09-22 이후 30일), 별 32,049, 포크 1,008, 기여자 약 336명(익명 포함) | 확인(문서): `gh api` |

### 3.2 AI 기능 개요

- 호출 방식: 빈 프롬프트 상태의 물음표(`?`) 키 입력 시 `atuin ai inline --hook` 실행. 확인(소스): `crates/atuin-ai/src/shell.rs:16-59`(Zsh), `65-106`(Bash), `112-161`(Fish). 확인(문서): `docs/docs/ai/introduction.md:11-12`
- 기본 바인딩: `atuin init` 출력에 AI 바인딩 기본 포함, `--disable-ai` 또는 `ai.enabled = false`로 해제. 확인(소스): `crates/atuin/src/command/client/init.rs:26-27, 94`
- 최초 사용: `ai.enabled` 미설정 시 `Enable Atuin AI`, `Disable ? Keybind`, `Cancel` 선택 화면. 확인(소스): `crates/atuin-ai/src/commands/inline.rs:26-45, 390-453`
- 기능 범위
  - 명령 생성, 후속 요청 기반 명령 수정, 자연어 질의응답, 서버 측 웹 검색. 확인(문서): `introduction.md:24-46`
  - 슬래시 명령 `/help`, `/new`, `/reload`, `/model`, 사용자 skill 명령. 확인(문서): `docs/docs/ai/slash-commands.md`
  - 클라이언트 측 도구: `AtuinHistory`, `AtuinOutput`, `Read`, `Write`, `Shell`. 확인(문서): `docs/docs/ai/tools-permissions.md:59-157`
  - 세션 지속: 마지막 상호작용 후 기본 1시간 내 재호출 시 같은 세션 이어감. 확인(문서): `docs/docs/ai/settings.md:17-23`. 확인(소스): `inline.rs:235-241`
  - 추가 컨텍스트 파일 `TERMINAL.md`, `.atuin/TERMINAL.md`. 확인(문서): `docs/docs/ai/user-context.md:5-14`

### 3.3 UI 형태와 편집, 후속 수정, 설명 기능

- AI UI 형태: 인라인 TUI(대체 화면 미사용)
  - `eye_declare::RunOptions::default()` 사용. 확인(소스): `inline.rs:367-370`
  - eye_declare `ScreenMode` 기본값 `Inline`(메인 화면, 완료 블록의 스크롤백 유입), `AltScreen`은 명시 선택 시만 사용. 확인(소스): eye-declare `crates/eye_declare/src/runtime.rs:337-364, 652-654`
  - `crates/atuin-ai/src/tui/` 내 `EnterAlternateScreen` 호출 없음(대체 화면 사용은 threads resume 선택기 `resume_tui/terminal.rs:194` 한정). 확인(소스)
- 히스토리 검색 UI(참고): 기본 `inline_height = 40` 인라인 뷰포트, 0 설정 시 대체 화면 전체 화면, tmux 3.2 이상 팝업 모드. 확인(소스): `crates/atuin/src/command/client/search/interactive.rs:1705-1707, 1908-1913`. 확인(문서): `docs/docs/configuration/config.md:334-344`
- 생성 명령의 TUI 내 직접 편집: 없음
  - 키 정책: 입력란이 빈 상태에서 명령 존재 시 Enter는 실행, Tab은 삽입. 입력란은 프롬프트(후속 질의) 전용. 확인(소스): `crates/atuin-ai/src/tui/app.rs:64-99, 266-281, 1265-1271`
  - 입력란 위아래 화살표: 이전 제출 메시지 재호출 기능. 생성 명령 편집 기능 아님. 확인(소스): `tui/recall.rs:1-4`
  - 편집 경로: Tab 삽입 후 셸 라인 에디터에서 편집. 확인(문서): `introduction.md:26`
- 후속 수정 기능: 있음. 후속 프롬프트로 삽입 대상 명령 갱신, 이전 턴 명령의 Enter 또는 Tab 사용 가능. 확인(문서): `introduction.md:30-40`
- 편집 후 재수정 제약: AI 호출 조건이 빈 입력 줄 또는 `?` 단독 입력. 셸 버퍼에서 편집한 명령의 AI 재전달 경로 없음. 확인(소스): `shell.rs:18, 67, 116`. `atuin ai inline [COMMAND]` 인자 정의는 존재하나 셸 훅의 인자 미전달. 확인(소스): `crates/atuin-ai/src/commands.rs:33-45`, `shell.rs:28, 76, 125`
- 설명 기능: 있음. 명령 생성 외 질문에 자연어 응답, 웹 검색 사용. 확인(문서): `introduction.md:36-46`

### 3.4 LLM 제공자와 인증 방식

- 클라이언트의 LLM 직접 호출 없음. 모든 요청의 `<endpoint>/api/cli/chat` SSE POST 전송. 확인(소스): `crates/atuin-ai/src/stream.rs:107, 150-158`
- 기본 엔드포인트: Atuin Hub `https://hub.atuin.sh`. 확인(소스): `crates/atuin-client/src/settings.rs:58-59`
- Hub 인증: 브라우저 기반 Hub 로그인 흐름, 세션 토큰 저장, 기존 sync 계정 연결. 확인(소스): `inline.rs:105-167`. Hub 계정 필수, 현재 무료. 확인(문서): `introduction.md:5-7`
- Hub 로그인 시 sync 설정에 따른 셸 히스토리 동기화 안내 문구. 확인(소스): `inline.rs:116-122`
- Hub 사용 모델과 공급사: `/model` 목록의 서버 제공 방식, 공급사 명칭 비공개. 미확인. 운영자 포럼 답변(2026-07-09): 공급사 주기적 교체, 무보존(zero data retention) 공급사 사용. 확인(문서, 비공식 답변)
- 자체 호스팅: `atuinsh/atuin-ai-server`(Elixir, Gleam 기반 `atuin-ai-core`, Apache-2.0). OpenAI 호환 chat completions 엔드포인트(Ollama, vLLM, LM Studio, llama.cpp, LiteLLM, OpenRouter 등) 연결, 도구 호출 지원 모델 필수. 확인(문서): `docs/docs/ai/self-hosting.md:3-5`, atuin-ai-server README
- 자체 서버 연결: `ai.endpoint`, `ai.api_token`, `ai.endpoint_protocol`(auto, hub, oss) 설정, CLI `--api-endpoint`, `--api-token`. 확인(소스): `commands.rs:16-29`, `inline.rs:47-65`. 확인(문서): `settings.md:25-47`
- 결과: Anthropic, OpenAI 등 공급사 API 키의 클라이언트 직접 설정 경로 없음. Hub 계정 또는 별도 서버 운영 필요. 확인(소스)

### 3.5 지원 OS, 셸과 셸 통합 방식

- AI 지원 셸: Zsh, Bash, Fish(Tier 1). Nushell, Xonsh, PowerShell은 히스토리 검색만 지원, Atuin AI 미지원. 확인(문서): `docs/docs/support.md:13-40`, docs.atuin.sh/latest/support. 확인(소스): `init/powershell.rs` 내 AI 관련 코드 없음, `init/bash.rs:46-48`, `init/zsh.rs:38-40`, `init/fish.rs:86-88`만 AI 초기화 출력
- 공통 반환 프로토콜
  - 셸 함수의 stdout, stderr 교환 실행(`3>&1 1>&2 2>&3`): TUI의 터미널 출력, 결과 토큰의 명령 치환 캡처. 확인(소스): `shell.rs:28, 76, 125`
  - 결과 토큰: `__atuin_ai_execute__:<명령>`, `__atuin_ai_insert__:<명령>`, `__atuin_ai_print__:<텍스트>`, `__atuin_ai_cancel__`(stderr 출력). 확인(소스): `inline.rs:526-532`
  - TUI 시작 전 OSC 133 C(`\033]133;C\007`) 출력: 셸 통합 터미널의 프롬프트 재배치 시 출력 소거 방지 목적. 확인(소스): `shell.rs:20-26, 71-74, 119-122`
- Zsh
  - `zle -N self-atuin-ai-question-mark`, `bindkey '?'` 위젯 등록. 확인(소스): `shell.rs:57-59`
  - 삽입: `RBUFFER=""`, `LBUFFER=<명령>`. 실행: 동일 설정 후 `zle accept-line`. 확인(소스): `shell.rs:36-44`
  - 비어 있지 않은 버퍼: `zle self-insert`로 `?` 입력. bracketed-paste-magic 대응 `self-` 접두 명명. 확인(소스): `shell.rs:13-15, 52-54`
- Bash
  - `atuin-bind '?' _atuin_ai_question_mark`, Bash 4.0 이상 또는 ble.sh 환경 한정. 확인(소스): `shell.rs:100-106`
  - `atuin-bind` 내부: Bash 4.3 이상 `bind -m <keymap> -x`, 4.2 이하 readline 매크로 연쇄. 확인(소스): `crates/atuin/src/shell/atuin.bash:560-587, 707-767`
  - 삽입: `READLINE_LINE`, `READLINE_POINT` 설정. 확인(소스): `atuin.bash:217-232`
  - 실행: 프롬프트 재출력, `history -s`, preexec 함수 호출 후 함수 내부 `eval` 실행(ble.sh 환경은 `ble/widget/accept-line`). 확인(소스): `atuin.bash:234-323`(eval 303행)
- Fish: `bind "?"`, `commandline -r`, 실행 시 `commandline -f execute`. 확인(소스): `shell.rs:110-161`
- PowerShell(히스토리 검색 한정, AI 해당 없음)
  - `Set-PSReadLineKeyHandler`(Ctrl+r, UpArrow), `atuin search -i --result-file <임시 파일>` 실행, `PSConsoleReadLine::Replace` 버퍼 교체, `__atuin_accept__:` 접두 시 `AcceptLine()`. 확인(소스): `crates/atuin/src/shell/atuin.ps1:50-55, 137-232`
  - `PSConsoleHostReadLine` 함수 교체 방식의 히스토리 기록 훅. 확인(소스): `atuin.ps1:57-60`
- 셸 통합 없는 사용: `--hook` 없는 `atuin ai inline` 실행 시 결과 명령의 stdout 출력. 확인(소스): `inline.rs:533-539`. 공식 문서 미기재. 확인(문서). TUI 렌더링도 stdout 사용(eye_declare `runtime.rs:650`), 명령 치환 캡처 용도 적합성 미확인(추정).
- OS: Linux x86_64, arm64, macOS arm64, x86_64, Windows x86_64, WSL-2 x86_64 사전 빌드(Tier 1). 확인(문서): `support.md:42-84`. Windows 빌드의 `atuin-ai` 의존 포함(`crates/atuin/Cargo.toml:162-163`), Windows에서 AI 동작 여부 미확인.

### 3.6 자동 실행 여부와 안전 기능

- 명령 실행 경로 2종
  - (a) 제안 명령 실행: TUI 내 Enter 1회로 셸에서 즉시 실행(Zsh `zle accept-line`, Bash `eval`, Fish `commandline -f execute`). 확인(소스): `fsm/mod.rs:272-294`, `shell.rs:36-40, 84-85, 136-141`. 확인(문서): `introduction.md:26`
  - (b) AI 에이전트 `Shell` 도구: AI 요청에 따른 명령 실행, 기본 매 실행 권한 확인. 확인(문서): `tools-permissions.md:5-7, 139-157`. 확인(소스): `permissions/check.rs:30-64`(규칙 불일치 시 Ask)
- 위험 명령 확인
  - LLM의 `suggest_command` 도구 호출에 `danger`, `confidence` 값 포함, `danger`가 medium 또는 high인 경우 추가 Enter 확인 단계. 확인(소스): `fsm/mod.rs:1209-1223`, `tui/view/turn.rs:409-412`
  - 확인 대기 5초 경과 시 확인 상태 해제. 확인(소스): `fsm/mod.rs:286-290, 334-339`
  - Hub 서버의 위험 패턴 추가 평가. 확인(문서): `introduction.md:48-52`. 서버 측 구현 미확인.
- 권한 체계: `.atuin/permissions.ai.toml`(디렉터리 계층)과 전역 파일의 allow, deny, ask 규칙, 복합 명령 하위 명령별 판정. 확인(문서): `tools-permissions.md:9-38, 159-180`
- `ai.yolo = true`: 전체 권한 확인 생략. 확인(소스): `tui/app.rs:548`. 확인(문서): `settings.md:49-55`
- 자동 승인 도구: `LoadSkill` 1종. 확인(소스): `tools/mod.rs:277-279`
- 파일 쓰기 전 원본 백업. 확인(문서): `tools-permissions.md:135-137`

### 3.7 배포 방식과 릴리스 대상

- 설치 방법: 설치 스크립트(`curl ... https://setup.atuin.sh | sh`, `$HOME/.atuin/bin` 설치), WinGet(`Atuinsh.Atuin`), cargo, Homebrew, MacPorts, mise, Nix, pacman, Termux pkg, GitHub Release. 확인(문서): `docs/docs/guide/installation.md:16, 38, 65, 71, 79, 84, 95, 105, 113, 118`
- 업데이트: `atuin update`(설치 스크립트 설치본 한정). 확인(문서): `installation.md:306-316`
- 빌드 체계: cargo-dist 0.31.0, shell과 powershell 설치기, GitHub Attestations. 확인(소스): `dist-workspace.toml:1-31`
- `v18.24.1` 릴리스 자산(atuin 클라이언트): `aarch64-apple-darwin`, `x86_64-apple-darwin`, `aarch64-unknown-linux-gnu`, `aarch64-unknown-linux-musl`, `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `x86_64-pc-windows-msvc`와 각 sha256, `atuin-installer.sh`, `atuin-installer.ps1`. 확인(문서): `gh release view v18.24.1 -R atuinsh/atuin`
  - ARMv7: 없음
  - Linux ARM64, macOS ARM64: 있음
  - Windows ARM64: 없음
- AI 기능 포함 여부: `ai` 기본 feature 포함, 단일 `atuin` 바이너리 내 내장. 확인(소스): `crates/atuin/Cargo.toml:43-51, 55`

### 3.8 LLM 전송 컨텍스트

- 기본 전송 항목: 메시지, OS, 셸, Linux 배포판 이름, 기능 목록(capabilities), invocation id, 세션 id. 확인(소스): `context.rs:98-139`, `stream.rs:117-148`
- 선택 전송 항목(기본값 false): 현재 디렉터리(`ai.opening.send_cwd`), 직전 명령(`ai.opening.send_last_command`). 확인(소스): `inline.rs:72-80`, `context.rs:117-132`. 확인(문서): `settings.md:98-126`
- 자동 전송 항목: `TERMINAL.md` 파일 내용(최대 10개, 파일당 10,000자). 파일 내 `` !`명령` `` 형식 셸 치환 결과 포함. 확인(문서): `user-context.md:7-38`
- 도구 호출 기반 전송: 히스토리 검색 결과, 캡처된 명령 출력, 파일 내용. 기본 권한 확인 후 전송. 확인(문서): `tools-permissions.md:61-116`, `command-output.md:23-25`
- 기능 목록 기본값: 히스토리 검색, 명령 출력, 파일 도구, 명령 실행 모두 true. 확인(소스): `context.rs:51-81`. 확인(문서): `settings.md:63-96`
- 서버 측 보존: 운영자 포럼 답변 기준 사용량 요약(모델별 토큰 수)만 보존, 프롬프트와 히스토리, 도구 출력 미보존. 확인(문서, 비공식 답변). 공식 정책 문서: 미확인
- 로컬 저장: AI 세션 SQLite 저장, 세션 재개용. 확인(소스): `inline.rs:203-241`

---

## 4. howtui 계획 기능 대비 비교표

| 항목 | howtui 계획 | howto (HEAD `d190fa1`) | how-tui (1.0.3) | Atuin AI (v18.24.1, HEAD `92a8b9a`) |
|---|---|---|---|---|
| 독립 경량 TUI | 지원 예정 | 미지원(TUI 없음, 입력 줄 주입만) | 미지원(TUI 없음, 목록 출력) | 부분(인라인 TUI 존재. Atuin 히스토리 DB 필수 `inline.rs:67-70`, Hub 계정 또는 자체 서버 필요) |
| 자연어 명령 생성 | 지원 예정 | 지원 | 지원(복수 후보) | 지원 |
| TUI 내 생성 명령 직접 편집 | 지원 예정 | 미지원(셸 버퍼 편집) | 미지원 | 미지원(Tab 삽입 후 셸 버퍼 편집) |
| AI 후속 수정 | 지원 예정 | 미지원(질의 재작성 재실행) | 미지원 | 지원(대화형 후속 프롬프트) |
| AI 설명 | 지원 예정(짧은 설명) | 미지원(명령만 출력 지시) | 미지원(스키마상 명령만) | 지원(자연어 응답, 웹 검색) |
| Bash 통합 | 지원 예정(readline) | 미지원(문서 없음, TIOCSTI 주입은 셸 무관) | 미지원 | 지원(`bind -x`, `READLINE_LINE`, Bash 4.0 이상) |
| Zsh 통합 | 지원 예정(ZLE) | 부분(`bindkey -s` 매크로와 TIOCSTI, Linux 6.2 이상 커널 설정 의존) | 미지원 | 지원(ZLE 위젯, `LBUFFER`) |
| PowerShell 통합 | 지원 예정(PSReadLine) | 미지원(Windows stdout 출력만) | 미지원 | 미지원(AI 미지원, 히스토리 검색만 PSReadLine) |
| 다중 LLM 제공자 | 지원 예정 | 부분(OpenAI, Gemini, DeepSeek 환경 변수 키. 릴리스 바이너리는 OpenAI 전용) | 부분(Gemini, Groq, keyring) | 부분(Hub 단일 경로. 자체 서버 경유 OpenAI 호환 엔드포인트) |
| 로컬 모델 | 미정 | 미지원 | 미지원 | 부분(자체 호스팅 서버 경유 Ollama 등) |
| 셸 통합 없는 stdout 출력 | 지원 예정 | 부분(Windows만, Unix는 TIOCSTI 고정) | 지원(유일 경로) | 부분(`--hook` 없는 `atuin ai inline`의 stdout 출력, 문서 미기재) |
| 전체 화면 강제 없음 | 지원 예정 | 해당 없음(TUI 없음) | 해당 없음(TUI 없음) | 지원(eye_declare 인라인 기본) |
| 단일 Rust 바이너리 | 지원 예정 | 미해당(Go 단일 바이너리) | 미해당(Python 패키지) | 부분(Rust 단일 `atuin` 바이너리 내장, 백엔드는 Hub 또는 Elixir 서버) |
| 자동 실행 없음 | 원칙 | 해당(실행 기능 없음) | 해당(실행 기능 없음) | 비해당(Enter 즉시 실행, AI `Shell` 도구 실행) |
| 위험 명령 경고 | 미정 | 없음 | 없음 | 있음(LLM `danger` 값 기반 추가 확인, Hub 추가 평가) |
| ARMv7 릴리스 | 미정 | 없음 | 해당 없음(순수 Python) | 없음 |
| Windows ARM64 릴리스 | 미정 | 있음(OpenAI 전용 구판) | 해당 없음(순수 Python) | 없음 |

---

## 5. howtui 차별화 근거

근거 확인 범위 내 차별화 항목 5종.

- (1) PowerShell 입력 버퍼 삽입 기반 AI 명령 도우미
  - Atuin AI의 PowerShell 미지원. 확인(문서): `support.md:37`. 확인(소스): `init/powershell.rs` AI 코드 부재
  - howto의 Windows stdout 출력 한정. 확인(소스): `ioctl_windows.go:9-11`
  - how-tui의 셸 통합 부재. 확인(소스)
  - 조사 대상 3종 중 Bash, Zsh, PowerShell 3종 동시 지원 사례 없음
- (2) TUI 내 생성 명령 직접 편집과 편집본 기반 AI 재수정
  - 3종 모두 TUI 내 생성 명령 편집 기능 없음. 확인(소스): Atuin `tui/app.rs:1265-1271`, howto 전체 소스, how-tui `cli.py:202-207`
  - Atuin의 셸 버퍼 편집본 AI 재전달 경로 부재(빈 버퍼 조건). 확인(소스): `shell.rs:18, 67, 116`
- (3) 자동 실행 배제 원칙
  - Atuin AI의 Enter 1회 즉시 실행 기본 동작과 AI `Shell` 도구 실행 기능. 확인(소스): `fsm/mod.rs:272-294`, 확인(문서): `tools-permissions.md:139-157`
  - howtui의 삽입 또는 출력 한정 정책의 Atuin 대비 차이
- (4) 계정과 별도 서버 없는 다중 제공자 직접 연결
  - Atuin AI의 Hub 로그인 필수 또는 Elixir 자체 서버 운영 필요. 확인(소스): `inline.rs:58-65, 105-167`, `stream.rs:107`
  - howto, how-tui의 제공자 2-3종 고정, 사용자 지정 엔드포인트 없음. 확인(소스)
- (5) 독립 경량 단일 바이너리와 커널 의존 없는 입력 삽입
  - Atuin AI의 Atuin 히스토리 DB, 세션 DB 의존. 확인(소스): `inline.rs:67-70, 203-205`
  - howto의 TIOCSTI 의존과 Linux 6.2 이상 비활성 가능 구조(이슈 #2). 확인(소스), 확인(문서)
  - how-tui의 Python 3.14 런타임 요구. 확인(소스)

---

## 6. 계획 차별화와 상충 사항

- (1) Atuin AI의 핵심 흐름 선점
  - 인라인 TUI, 자연어 명령 생성, 후속 수정, 자연어 설명, Bash와 Zsh 입력 버퍼 삽입, Rust 단일 바이너리 내장 기능 기존 제공. 확인(소스), 확인(문서)
  - howtui 계획 중 "AI 후속 수정과 설명", "전체 화면 강제 없음", "Bash, Zsh 통합", "Rust 단일 바이너리" 항목의 단독 차별화 근거 부족
- (2) Atuin의 stdout 출력 경로 존재
  - `--hook` 없는 `atuin ai inline`의 결과 stdout 출력. 확인(소스): `inline.rs:533-539`. 문서 미기재와 TUI 동일 stdout 렌더링 차이만 존재
- (3) Atuin의 위험 명령 확인 기능 기존 제공
  - `danger` 값 기반 추가 확인과 Hub 서버 평가. howtui 동등 수준 기능 부재 시 비교 열위
- (4) Atuin의 로컬 모델 경로 존재
  - 자체 호스팅 서버 경유 Ollama, vLLM 등 연결 가능. 확인(문서): `self-hosting.md:3-5`
- (5) howto의 Go 단일 바이너리와 다중 제공자, Windows ARM64 자산
  - 단일 바이너리, 다중 제공자 항목의 howto 대비 차별화 근거 부족. 단 릴리스 바이너리 OpenAI 전용 구판 상태. 확인(문서): 이슈 #5
- (6) 명칭 유사성
  - PyPI `how-tui`(실행 명령 `how`)와 howtui 명칭 유사. 확인(문서). crates.io `howtui`, `how-tui`, `how_tui` 미등록(2026-10-22 API 404 응답). 확인(문서)

---

## 7. 설계 참고 사항

- (1) 입력 버퍼 반환 방식
  - TIOCSTI 주입 방식의 Linux 6.2 이상 비활성 가능성. 셸 위젯 API(Bash `READLINE_LINE`, Zsh `BUFFER`, PSReadLine `Replace`) 기반 반환 필요. 근거: howto 이슈 #2, 커널 커밋 `83efeeeb3d04`
  - Atuin의 stdout, stderr 교환과 접두 토큰 프로토콜(`__atuin_ai_insert__:` 등) 사례. 근거: `shell.rs:28-51`, `inline.rs:526-541`
  - PowerShell 반환 사례: 임시 결과 파일과 `PSConsoleReadLine::Replace`. 근거: `atuin.ps1:137-210`
- (2) 셸 버전 제약
  - Bash 4.0 미만 `READLINE_LINE` 부재, Bash 4.2 이하 `bind -x` 제약. 근거: `shell.rs:100-106`, `atuin.bash:560-603`
- (3) 터미널 호환
  - OSC 133 C 출력으로 셸 통합 터미널(Ghostty 등)의 재배치 소거 방지. 근거: `shell.rs:20-26`
  - Zsh 다중 행 프롬프트 재출력 시 프롬프트 높이 보정. 근거: `shell.rs:2-11`
- (4) 출력 정리
  - 개행 외 제어 문자(`\r`, ESC 등) 제거 필요성. 근거: howto `security.go:7-11`의 개행 한정 처리

---

## 8. 출처 목록

### 8.1 저장소와 릴리스 메타데이터

- https://github.com/antonmedv/howto (커밋 `d190fa1d3d18b13a28f9c874c113b221359b3e68`)
- https://github.com/antonmedv/howto/releases/tag/0.0.0
- https://github.com/antonmedv/howto/issues/2
- https://github.com/antonmedv/howto/issues/5
- https://github.com/antonmedv/howto/pull/3
- https://proxy.golang.org/github.com/antonmedv/howto/@latest
- https://github.com/noahhefner/how-tui (커밋 `395805eadcc936c0a23c85c213cd718401a50d70`)
- https://pypi.org/project/how-tui/ (JSON API: https://pypi.org/pypi/how-tui/json)
- https://github.com/atuinsh/atuin (커밋 `92a8b9ae1d0bf5fff3e7a2378175b7774001fc0a`)
- https://github.com/atuinsh/atuin/releases/tag/v18.24.1
- https://github.com/atuinsh/atuin-ai-server (커밋 `4d582bc5ceea5b5edfdcf3abb49dc850400cda7c`)
- https://github.com/atuinsh/eye-declare (커밋 `e14258bc0965e61e76fa94e39455699bd676eeb5`)
- https://crates.io/crates/eye_declare
- https://crates.io/api/v1/crates/howtui (404 응답, 2026-10-22)

### 8.2 공식 문서

- https://docs.atuin.sh/latest/ai/introduction/
- https://docs.atuin.sh/latest/support/
- Atuin 저장소 문서 원본: `docs/docs/ai/introduction.md`, `settings.md`, `self-hosting.md`, `slash-commands.md`, `tools-permissions.md`, `user-context.md`, `command-output.md`, `docs/docs/support.md`, `docs/docs/guide/installation.md`, `docs/docs/configuration/config.md`
- https://blog.atuin.sh/atuin-ai-oss/ (2026-07-14, 대화 미보존 언급, 자체 호스팅 서버 공개)
- https://forum.atuin.sh/t/document-ai-privacy-policy-please/1459 (운영자 답변 2026-07-09, 비공식)

### 8.3 Linux TIOCSTI 관련

- https://github.com/torvalds/linux/commit/83efeeeb3d04b22aaed1df99bc70a48fe9d22c4d ("tty: Allow TIOCSTI to be disabled", v6.2 포함 확인)
- https://github.com/torvalds/linux/blob/v6.2/drivers/tty/tty_io.c (tiocsti 함수 2272-2281행)
- https://github.com/torvalds/linux/blob/v6.2/drivers/tty/Kconfig (LEGACY_TIOCSTI 152-168행)
- https://lkml.iu.edu/2210.1/07537.html ([PATCH 2/2] tty: Allow TIOCSTI to be disabled)
