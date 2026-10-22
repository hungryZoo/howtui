# Codex CLI 바이너리 위임(codex exec 서브프로세스) 조사

- 조사일: 2026-10-22
- 조사 대상: 사용자가 설치한 공식 `codex` 바이너리를 howtui가 서브프로세스로 실행해 텍스트 전용 1회 응답(`{"command","explanation","notes"}`)을 받는 경로
- 전제 조건: howtui 자체 OAuth 미사용, Codex 자격 증명 파일(`$CODEX_HOME/auth.json`) 직접 열람 금지, 쿠키 추출 금지, 비공식 엔드포인트 금지
- 조사 범위: github.com/openai/codex 소스(shallow clone, 커밋 `c260087efe8c`, 2026-10-22 09:37 UTC), GitHub 릴리스와 이슈, developers.openai.com/codex 문서(현재 learn.chatgpt.com/docs로 308 리다이렉트), 로컬 설치 `codex` 0.136.0의 도움말 출력
- 표기 규칙: 확인(소스), 확인(문서), 확인(실행), 미확인 4종. 경로 표기는 `$CODEX_HOME`(기본값 `$HOME/.codex`) 사용.
- 조사 방식: 빌드와 실행 없음. 로컬 실행은 `codex --version`, `codex --help`, `codex exec --help`, `codex login --help`, `codex login status`(종료 코드와 첫 문구만 확인)로 한정. 프롬프트 포함 `codex exec` 실행 없음.

---

## 0. 핵심 결론 요약

| 항목 | 결론 | 근거 수준 |
| --- | --- | --- |
| 최신 정식 버전 | 0.163.0 (태그 `rust-v0.163.0`, 2026-10-15 21:30 UTC). 프리릴리스 0.164.0-alpha.11 (2026-10-22) | 확인(소스: GitHub releases API) |
| 로컬 설치 버전 | 0.136.0 (2026-06-01 릴리스), npm 전역 설치 | 확인(실행) |
| 공식 "도구 없음, 텍스트 전용" 모드 | 없음. CLI 플래그와 설정 키 중 도구 전체 차단 스위치 부재 | 확인(소스, 문서) |
| `--sandbox read-only` 의미 | 파일 읽기와 명령 실행 허용, 쓰기와 네트워크만 차단. `codex exec` 기본 승인 정책 `never`와 결합 시 `ls`, `cat` 류 자동 실행 가능 | 확인(소스, 문서) |
| 근접 대안 | `features.shell_tool=false` 등 기능 플래그 15종 이상 + `web_search="disabled"` + `--ignore-user-config` + 실행 환경 제거(`CODEX_EXEC_SERVER_URL=none`) 조합 | 확인(소스) |
| 최소 요구 버전 | 0.147.0 (2026-08-07). `features.view_image` 도입 버전. 이전 버전은 로컬 이미지 열람 도구 비활성화 수단 없음 | 확인(소스) |
| 차단 불가 잔여 항목 | (1) `$CODEX_HOME/AGENTS.md` 전역 지침 자동 주입 (2) 모델 카탈로그 기반 `apply_patch` 도구 등록 (3) 관리자 requirements 강제 설정 | 확인(소스) |
| 설치 감지 | `PATH` 탐색 + `codex --version` 출력(`codex-cli X.Y.Z`) 파싱 | 확인(실행) |
| 로그인 감지 | `codex login status` 종료 코드 0/1, stderr 첫 문구 `Logged in using ChatGPT` | 확인(소스, 실행) |
| device code 로그인 | `codex login --device-auth` 존재(베타). ChatGPT 보안 설정에서 device code 로그인 사전 활성화 필요 | 확인(소스, 문서, 실행) |
| 라이선스 | Apache-2.0. 사용자가 설치한 바이너리의 서브프로세스 호출은 재배포 아님 | 확인(소스) |
| 약관상 제3자 도구 연동 | 명시적 허용 또는 금지 문구 미발견. 공식 TypeScript SDK도 `codex exec` 서브프로세스 방식 | 미확인(약관 본문 403), 확인(소스: SDK) |
| 지연 시간 | 공식 수치 없음. 매 호출 콜드 스타트와 에이전트 지침(모델별 약 1.8만-6.2만 자) 동반 | 미확인 |
| 로컬 기록 | `--ephemeral` 시 세션 rollout 파일과 state DB 스레드 기록 생략. `history.jsonl`은 TUI 전용 | 확인(소스) |
| 판정 | 조건부 가능. 0.147.0 이상 + 아래 8장 명령줄 + 전역 AGENTS.md 존재 고지 + JSONL 감시 조합 시 howtui 개인정보 보장 유지 수준 근접. 공식 지원 모드가 아닌 플래그 조합이므로 버전별 회귀 점검 필수 | 종합 |

---

## 1. 최신 버전과 설치 경로

### 1.1 버전

- 최신 정식: 0.163.0, 2026-10-15T21:30:38Z. 확인(소스: `gh api repos/openai/codex/releases/latest`).
- 최신 프리릴리스: 0.164.0-alpha.11, 2026-10-22T04:22:12Z. 확인(소스).
- 로컬: `codex-cli 0.136.0`, 경로 `$HOME/.nvm/versions/node/v24.16.0/bin/codex`(npm 래퍼). 확인(실행).
- 기능 플래그 이력: `view_image` 키는 0.146.0에 없음, 0.147.0(2026-08-07)부터 존재. 확인(소스: 태그별 `codex-rs/features/src/lib.rs`).

### 1.2 설치 방법 (README.md)

| 방법 | 명령 |
| --- | --- |
| 독립 설치 스크립트(macOS, Linux) | `curl -fsSL https://chatgpt.com/codex/install.sh \| sh` |
| 독립 설치 스크립트(Windows) | `powershell -ExecutionPolicy ByPass -c "irm https://chatgpt.com/codex/install.ps1 \| iex"` |
| npm | `npm install -g @openai/codex` |
| Homebrew | `brew install --cask codex` |
| GitHub 릴리스 바이너리 | `codex-x86_64-unknown-linux-musl.tar.gz` 등 플랫폼별 아카이브 |
| 갱신 | `codex update` 하위 명령 |

- 독립 설치 스크립트 기본 다운로드 위치는 `https://releases.openai.com/codex`, 실패 시 GitHub Releases 대체. 확인(소스: README.md).
- 시스템 요구 사항: macOS 12+, Ubuntu 20.04+/Debian 10+, Windows 11(WSL2). 확인(소스: docs/install.md).

---

## 2. `codex exec` 비대화형 모드 플래그

| 플래그 | 용도 | 근거 |
| --- | --- | --- |
| `-m, --model <MODEL>` | 모델 지정 | 확인(실행: `codex exec --help`) |
| `--json` (`--experimental-json`) | stdout JSONL 이벤트 스트림 | 확인(실행, 소스: exec/src/cli.rs:65-70) |
| `-o, --output-last-message <FILE>` | 최종 메시지 파일 저장 | 확인(실행, 소스: exec/src/cli.rs:74-79) |
| `--output-schema <FILE>` | 최종 응답 JSON Schema 강제. Responses API `text.format`의 `json_schema`, `strict: true`로 전달 | 확인(소스: codex-api/src/common.rs:392-409, core/src/client_common.rs:52) |
| `PROMPT` 생략 또는 `-` | stdin에서 프롬프트 읽기 | 확인(실행, 소스: exec/src/lib.rs:2403-2413) |
| `--skip-git-repo-check` | Git 저장소 밖 실행 허용 | 확인(실행, 소스: exec/src/cli.rs:37-38) |
| `-C, --cd <DIR>` | 작업 루트 지정 | 확인(실행) |
| `--ephemeral` | 세션 rollout 파일 미저장 | 확인(실행, 문서) |
| `--ignore-user-config` | `$CODEX_HOME/config.toml` 미적재. 인증은 계속 `CODEX_HOME` 사용 | 확인(실행, 소스: config/src/loader/mod.rs:515-531) |
| `--ignore-rules` | execpolicy `.rules` 미적재 | 확인(실행) |
| `--color never` | ANSI 색상 제거 | 확인(실행, 소스: exec/src/lib.rs:335-341) |
| `-s, --sandbox` | `read-only`, `workspace-write`, `danger-full-access` | 확인(실행) |
| `-c key=value` | 설정 덮어쓰기, 값은 TOML 파싱 후 실패 시 문자열 | 확인(실행) |
| 타임아웃 | 전용 플래그 없음. 호출 측 프로세스 종료로 처리 | 확인(실행: 도움말에 부재) |

- 승인 정책: `codex exec`는 하네스 수준에서 `approval_policy = never` 강제. 확인(소스: exec/src/lib.rs:589-591, "Default to never ask for approvals in headless mode").
- 기본 샌드박스: read-only. 확인(문서: non-interactive-mode, "By default, `codex exec` runs in a read-only sandbox.").
- stdin 주의: 프롬프트를 인자로 주고 stdin이 파이프이면 stdin 내용이 `<stdin>` 블록으로 추가. howtui 자신이 파이프 입력을 받은 상태에서 자식 stdin을 상속하면 그 내용이 모델로 전송. 자식 stdin 명시 지정 필수. 확인(소스: exec/src/lib.rs:2403-2413).
- 종료 코드: 오류 또는 턴 실패 시 1. 확인(소스: exec/src/lib.rs:1404-1405).
- JSONL 이벤트: `thread.started`, `turn.started`, `turn.completed`, `turn.failed`, `item.started`, `item.updated`, `item.completed`, `error`. item 종류는 `agent_message`, `reasoning`, `command_execution`, `file_change`, `mcp_tool_call`, `collab_tool_call`, `web_search`, `todo_list`, `error`. 확인(소스: exec/src/exec_events.rs:11-37, 107-133).
- 공식 SDK 사례: TypeScript SDK가 `codex exec --experimental-json`을 서브프로세스로 실행하고 `--config web_search="disabled"` 등 전달. 확인(소스: sdk/typescript/README.md:5, sdk/typescript/src/exec.ts:95, 166).

---

## 3. 도구 비활성화 가능 범위

### 3.1 공식 텍스트 전용 모드 부재

- 모델 요청의 `tool_choice`는 `"auto"` 고정. 확인(소스: core/src/client.rs:1116).
- `--output-schema`는 최종 텍스트 형식만 제약, 도구 호출 자체는 차단하지 않음. 확인(소스: codex-api/src/common.rs:392-409).
- 내부 확장 API `ToolPolicy`에 "an empty list permits no tools" 기능 존재. 단 guardian reviewer 내부 전용, CLI와 설정 키로 미노출. 확인(소스: ext/extension-api/src/tool_policy.rs:11, ext/guardian-reviewer/src/settings.rs:44).
- 문서의 도구 차단 안내는 커스텀 에이전트용 `features.shell_tool`, `features.apps`, `features.plugins`, `features.memories`, `features.request_permissions_tool` 및 `skills.*` 수준. 확인(문서: codex-manual.md, "To reduce a custom agent's capabilities, set supported feature keys to `false`").

### 3.2 read-only 샌드박스 의미

- `SandboxPolicy::has_full_disk_read_access()`가 모든 정책에서 `true` 반환. read-only는 쓰기 불가와 네트워크 기본 차단만 의미. 확인(소스: protocol/src/protocol.rs:1333-1335, 1188-1195).
- 문서 표: "Read-only non-interactive (CI) | `--sandbox read-only --ask-for-approval never` | Codex can read files and run commands within the read-only sandbox; it never asks for approval." 확인(문서: agent-approvals-security).
- 결론: read-only 단독 사용 시 모델의 `cat $HOME/.ssh/...` 류 자동 실행과 결과의 모델 전송 가능. howtui 개인정보 보장 위반.

### 3.3 도구별 비활성화 수단 (최신 0.163.0 기준)

| 도구 또는 컨텍스트 | 기본값 | 비활성화 수단 | 근거 |
| --- | --- | --- | --- |
| 셸 실행(`exec_command`, `write_stdin`) | 켜짐 | `features.shell_tool=false` | core/src/tools/spec_plan.rs:1235-1243 |
| 로컬 이미지 열람(`view_image`) | 켜짐 | `features.view_image=false` (0.147.0 이상) | spec_plan.rs:1423 |
| 웹 검색(호스티드, 독립 `web.run`) | `cached` | `web_search="disabled"` | core/src/config/mod.rs:3928-3929, spec_plan.rs:1630-1640 |
| ChatGPT 커넥터 MCP(codex_apps) | ChatGPT 로그인 시 켜짐 | `features.apps=false` | codex-mcp/src/mcp/mod.rs:353-355, 452-454 |
| 사용자 MCP 서버 | `config.toml` 정의 시 켜짐 | `--ignore-user-config` (개별 차단은 `mcp_servers.<id>.enabled=false`) | config/src/loader/mod.rs:515-531 |
| 플러그인과 원격 플러그인 카탈로그 | 켜짐 | `features.plugins=false`, `features.remote_plugin=false`, `features.plugin_sharing=false` | features/src/lib.rs, 이슈 #47483 |
| 도구 제안(플러그인 설치 요청) | 켜짐 | `features.tool_suggest=false` | spec_plan.rs:718-723 |
| 멀티 에이전트 | 켜짐(v1) | `features.multi_agent=false`, `features.multi_agent_v2=false` | features/src/lib.rs (`Collab` 키 `multi_agent`) |
| 이미지 생성 | 켜짐 | `features.image_generation=false` | spec_plan.rs:783-818 |
| 메모리 | 꺼짐 | `features.memories=false` 명시 | features/src/lib.rs |
| 목표(goals) 도구 | 켜짐 | `features.goals=false` | app-server/src/extensions.rs:96 |
| 훅(hooks.json, 인라인 훅) | 켜짐 | `features.hooks=false` + 플러그인 비활성화 | hooks/src/engine/mod.rs:267, core/src/session/mod.rs:5608 |
| 사용자 입력 요청 도구 | 켜짐 | `tools.experimental_request_user_input.enabled=false` | core/src/config/mod.rs:2783-2789 |
| 계획 도구(update_plan) | 꺼짐 | 조치 불필요 | core/src/config/mod.rs:2791-2797 |
| 스킬 카탈로그 블록 | 켜짐 | `skills.include_instructions=false`, `skills.bundled.enabled=false` | config/src/skills_config.rs:40-55 |
| 스킬 list/read 도구 | 클라우드 스킬 또는 플러그인 루트 존재 시 켜짐 | `cloud.skills.enabled=false` + `features.plugins=false` | ext/skills/src/preparation.rs:24, 49-58 |
| 셸 환경 스냅샷(사용자 셸 rc 실행) | 켜짐 | `features.shell_snapshot=false` | core/src/session/session.rs:1472-1480 |
| `apply_patch` | 모델 카탈로그의 `apply_patch_tool_type` 존재 시 등록 | 기능 플래그 없음. 실행 환경 제거로 무력화 | spec_plan.rs:1409-1412 |
| MCP 리소스 도구 3종 | code_mode_only 모델에서 항상 등록 | MCP 서버 부재 시 빈 결과 | spec_plan.rs:1269-1272 |
| Code Mode `exec` 도구(V8) | 번들 모델 전부 `code_mode_only` | 차단 수단 없음. V8 전역에 파일 시스템, 네트워크 API 없음 | models-manager/models.json, code-mode-runtime/src/runtime/globals.rs:15-51 |

- 미지정 기능 키 처리: `-c features.<미지원키>=false`는 경고 후 무시, `--disable <미지원키>`는 오류 종료. 버전 혼재 대응에는 `-c` 형식 적합. 확인(소스: features/src/lib.rs:756, cli/src/main.rs:1003-1006).
- 문서와 소스 불일치: 설정 문서의 `tools.view_image` 키가 0.136.0, 0.146.0, 0.163.0 소스의 `ToolsToml`에 부재. 실제 키는 `features.view_image`. 확인(소스: config/src/config_toml.rs:702-710), 확인(문서: config-reference).

### 3.4 실행 환경 제거 (`CODEX_EXEC_SERVER_URL=none`)

- 소스 주석: "Setting `CODEX_EXEC_SERVER_URL=none` disables environment access by leaving the default environment unset and omitting the local environment. Callers use `default_environment().is_some()` as the signal for model-facing shell/filesystem tool availability." 확인(소스: exec-server/src/environment.rs:94-97).
- `--ignore-user-config` 사용 시 `EnvironmentManager::from_env` 경로로 환경 변수 직접 적용. 확인(소스: exec/src/lib.rs:717-722, exec-server/src/environment_provider.rs:118-123).
- 환경 부재 시 셸, `apply_patch`, `view_image` 핸들러가 "No usable execution environment is available." 오류 반환, 로컬 파일 접근 경로 소멸. 확인(소스: core/src/tools/handlers/mod.rs:161-187).
- 환경 부재 시 프로젝트 AGENTS.md 미적재, 전역 AGENTS.md는 적재. 확인(소스: core/tests/suite/agents_md.rs:1069-1096).
- 공개 사용자 문서 미기재 항목. 테스트 코드(app-server/tests/suite/v2/thread_shell_command.rs:181-183)에서 사용. exec 경로 동작은 미확인(실행).
- 0.136.0에도 동일 `none` 처리 존재. 확인(소스: 0.136.0 exec-server/src/environment_provider.rs:93).
- 0.136.0-0.146.x의 환경 부재 시 셸, `apply_patch`, `view_image` 도구 자체 미등록. 확인(소스: 0.136.0 core/src/tools/spec_plan.rs:548, 625, 640). 단 공개 문서 미기재 경로 단독 의존이므로 권장 최소 버전은 플래그 차단이 가능한 0.147.0 유지.
- 0.163.0의 환경 부재 시 도구 등록은 유지, 호출 시점 오류 반환 구조. 확인(소스: core/src/tools/spec_plan.rs:1235-1267에 환경 수 조건 없음).

### 3.5 설정 계층과 사용자 설정 상속

- 계층 순서: 패키지 기본값 < 관리자(macOS) < `/etc/codex/config.toml` < 클라우드 관리 설정 < 사용자 `config.toml` < 프로필 < 프로젝트(cwd, 상위 `.codex/config.toml`, 신뢰되지 않은 경로는 비활성) < 런타임(`-c`). 확인(소스: config/src/loader/mod.rs:123-137).
- `-c` 값은 최상위 우선. 관리자 `requirements.toml`과 엔터프라이즈 클라우드 requirements는 별도 제약으로 `-c`보다 우선. 확인(소스: config/src/loader/mod.rs:108-121).
- `--ignore-user-config` 효과: 사용자 MCP 서버, 훅, notify, developer_instructions, model_instructions_file, 프로젝트 신뢰 목록 제거. 프로젝트 신뢰 목록 제거로 프로젝트 계층 설정 비활성. 확인(소스).
- `--ignore-user-config` 부작용: `cli_auth_credentials_store` 사용자 설정도 무시, 기본값 `file`로 복귀. keyring 저장 사용자는 "Not logged in" 상태. 확인(소스: config/defaults.toml `cli_auth_credentials_store = "file"`).
- `cli_auth_credentials_store="auto"` 대체 불가: 저장 시 keyring 우선 기록. 파일 저장 사용자의 토큰 갱신 결과가 keyring으로 이동, 사용자 자신의 codex 설정과 어긋남. 확인(소스: login/src/auth/storage.rs:491-547).

### 3.6 잔여 위험

| 번호 | 항목 | 내용 | 완화 |
| --- | --- | --- | --- |
| (1) | 전역 AGENTS.md | `$CODEX_HOME/AGENTS.override.md` 또는 `AGENTS.md` 내용이 매 요청 사용자 지침으로 전송. 설정으로 차단 불가 | 파일 존재와 크기만 `stat`으로 확인 후 사용자 고지 또는 위임 거부 |
| (2) | `apply_patch` | 모델 카탈로그(서버 제공) 기반 등록. 실행 환경 존재 시 패치 검증 단계에서 대상 파일 읽기 후 safety 판정, 오류 문구로 파일 존재 여부와 문맥 일치 여부 노출 가능 | `CODEX_EXEC_SERVER_URL=none` |
| (3) | 버전 회귀 | 신규 도구가 기본 켜짐으로 추가될 가능성. 공식 지원 모드가 아닌 플래그 조합 | 검증 버전 범위 관리, `codex features list` 사전 점검, JSONL 감시 |
| (4) | 관리 설정 | 엔터프라이즈 requirements, 클라우드 관리 설정, 관리 훅의 `-c` 무력화 가능 | `codex features list` 결과 검증, 불일치 시 위임 거부 |
| (5) | 감시 사각 | `view_image`, `apply_patch` 검증 실패 등은 JSONL item으로 미노출 | 사전 차단 위주 설계, 감시는 보조 수단 |
| (6) | 서버 전송 | 프롬프트는 사용자 ChatGPT 계정 경유 OpenAI 전송. 보존 정책은 계정 또는 워크스페이스 설정 적용 | howtui 자체 OAuth 경로와 동일 수준, 최초 1회 고지 |
| (7) | 0.147.0 미만 | `view_image` 차단 수단 없음 | 버전 미달 시 위임 거부와 `codex update` 안내 |

---

## 4. 작업 디렉터리와 자동 컨텍스트 주입

| 컨텍스트 | 출처 | 차단 수단 | 근거 |
| --- | --- | --- | --- |
| 프로젝트 AGENTS.md | 프로젝트 루트(기본 마커 `.git`)부터 cwd까지 `AGENTS.override.md`, `AGENTS.md`, fallback 파일명 | `project_doc_max_bytes=0`(경로 탐색 전 조기 반환), 실행 환경 제거, 빈 cwd | core/src/agents_md.rs:1-18, 130-133 |
| 전역 AGENTS.md | `$CODEX_HOME/AGENTS.override.md` 우선, 없으면 `AGENTS.md` | 없음 | codex-home/src/instructions/mod.rs:40-80, app-server/src/message_processor.rs:370-372 |
| `<environment_context>` | cwd 경로, 셸 이름, 날짜, 시간대, 네트워크와 파일 시스템 권한 | `include_environment_context=false` | config/src/config_toml.rs:265-269, core/src/session/world_state.rs:283 |
| 권한 안내 블록 | 샌드박스와 승인 정책 설명 | `include_permissions_instructions=false` | config_toml.rs:256-257 |
| 앱 안내 블록 | 커넥터 안내 | `include_apps_instructions=false` | config_toml.rs:259-260 |
| 협업 모드 블록 | 협업 모드 안내 | `include_collaboration_mode_instructions=false` | config_toml.rs:262-263 |
| 스킬 카탈로그 | `$CODEX_HOME/skills`, 번들 스킬, 플러그인 스킬, 클라우드 스킬 | 3.3 표의 스킬 키 3종 | config/src/skills_config.rs |
| 메모리 | `$CODEX_HOME` 메모리 저장소 | `features.memories=false` | features/src/lib.rs |

- cwd 결정: `--cd` 지정 경로를 canonicalize, 미지정 시 프로세스 현재 디렉터리. 확인(소스: exec/src/lib.rs:362-368).
- 권장 cwd: `/tmp` 직하가 아닌 howtui 전용 0700 빈 디렉터리. `/tmp` 상위의 타인 생성 `.codex/config.toml`, `AGENTS.md` 배치 가능성 제거 목적.
- 프롬프트 디버그 수단: `codex debug prompt-input [PROMPT]`가 모델 입력 목록(JSON) 출력. 도구 목록은 미포함. 개발 단계 점검용. 확인(소스: cli/src/main.rs:274, 303-312, core/src/prompt_debug.rs:26-33). 실행 결과 미확인.

---

## 5. 인증 감지와 device code 로그인

### 5.1 설치와 로그인 감지

- 설치: `PATH`에서 `codex` 탐색 후 `codex --version` 출력 `codex-cli 0.136.0` 형식 파싱. 확인(실행).
- 로그인: `codex login status`. 출력은 stderr, 종료 코드 기준 판정. 확인(소스: cli/src/login.rs:448-513).

| 상태 | stderr 첫 문구 | 종료 코드 |
| --- | --- | --- |
| ChatGPT 로그인 | `Logged in using ChatGPT` | 0 |
| API 키 | `API key configured - <앞 8자>***<뒤 5자>` | 0 |
| 액세스 토큰, PAT, Bedrock, workload identity | `Logged in using ...` | 0 |
| 미로그인 | `Not logged in` | 1 |
| 오류 | `Error checking login status: ...` | 1 |

- API 키 모드 출력에 키 앞 8자와 뒤 5자 포함. howtui 화면과 로그에 stderr 원문 출력 금지, 첫 문구 접두어만 판정. 확인(소스: cli/src/login.rs:617-624).
- 로컬 결과: 종료 코드 0, `Logged in using ChatGPT`. 확인(실행).
- 저장소 모드 판별(파일 직접 열람 없이): `codex login status -c 'cli_auth_credentials_store="file"'`, 실패 시 `-c 'cli_auth_credentials_store="keyring"'` 순서 탐침, 성공 모드를 `codex exec`에 동일 전달. `login status`의 `-c` 반영 확인(소스: cli/src/login.rs:449). 탐침 동작 자체는 미확인(실행).
- ChatGPT 플랜 사용: ChatGPT 로그인 시 Plus, Pro, Business, Edu, Enterprise 플랜 포함 사용량 소비. API 키 로그인은 API 요금. 확인(문서: README.md, auth). 플랜 종류 조회 수단은 `login status`에 없음.
- 환경 변수 충돌: `OPENAI_API_KEY`, `CODEX_API_KEY` 존재 시 ChatGPT 로그인 상태에서도 401 실패 사례(이슈 #27651, 0.139.0, OPEN). `codex exec`는 `CODEX_API_KEY` 환경 변수 인증 허용(`enable_codex_api_key_env: true`). 자식 프로세스 환경에서 두 변수 제거 권장. 확인(소스: exec/src/lib.rs:745), 확인(이슈).

### 5.2 device code 로그인

- 명령: `codex login --device-auth`. 로컬 0.136.0 도움말에 존재. 확인(실행, 소스: cli/src/main.rs:548).
- 브라우저 로그인 실패 안내 문구에 "On a remote or headless machine? Use `codex login --device-auth` instead." 포함. 확인(소스: cli/src/login.rs:123).
- 문서 절차: (1) ChatGPT 보안 설정(개인) 또는 워크스페이스 권한(관리자)에서 device code 로그인 활성화 (2) `codex login --device-auth` 실행 (3) 브라우저에서 링크 열기, 로그인, 일회용 코드 입력. 상태 "beta". 확인(문서: auth, codex-manual.md "Preferred: Device code authentication (beta)").
- 대체 경로(문서): 브라우저 기기에서 로그인 후 `auth.json` 복사, 또는 `ssh -L 1455:localhost:1455` 콜백 포워딩. 확인(문서).
- howtui 연계: 초보자 SSH 환경 안내 문구로 `codex login --device-auth` 제시, TTY 상속 실행만 수행, 출력 캡처 없음. 토큰은 Codex 자체 저장소에만 존재.

---

## 6. 라이선스와 약관

- 저장소 라이선스: Apache-2.0 (LICENSE, NOTICE "OpenAI Codex Copyright 2025 OpenAI", codex-cli/package.json `"license": "Apache-2.0"`). 확인(소스).
- 사용자 설치 바이너리의 서브프로세스 호출: 코드 복제, 수정, 재배포 없음. NOTICE 고지 의무 발생 조건(재배포) 해당 없음. Apache-2.0 제6조 상표 조항상 "Codex" 명칭의 보증 암시 표현 회피 필요. 확인(소스: LICENSE).
- 공식 연동 선례: TypeScript SDK(Apache-2.0)가 동일 방식으로 `codex exec` 실행. 확인(소스: sdk/typescript).
- 약관: Codex 문서에 "All users remain subject to the Usage Policies and Terms of Use." 문구. 제3자 오픈소스 도구의 `codex exec` 호출 허용 또는 금지 문구 미발견. 확인(문서: codex-manual.md), Terms of Use 본문과 help.openai.com 문서는 403 응답으로 미열람, 미확인.
- 자동화 관련 문서 표현: API 키 플랜 "Great for automation in shared environments like CI", Codex 액세스 토큰 "for trusted scripts, schedulers, and private CI runners". ChatGPT 플랜의 스크립트 사용 제한 문구 미발견. 확인(문서), 해석은 미확인.
- 위험 판단: 사용자 본인 기기, 본인 계정, 사용자 조작 1회당 1요청 구조에서 위험 낮음 추정. 대량 배치 호출 금지와 최초 사용 고지 권장. 미확인.

---

## 7. 지연 시간과 로컬 기록

### 7.1 지연 시간과 부가 비용

- 공식 수치: 없음. 미확인.
- 매 호출 구성 요소: 프로세스 시작(npm 설치 시 Node 래퍼 경유), 설정과 인증 적재, 모델 카탈로그 조회(`/backend-api/codex/models`, 약 522 KB), 원격 플러그인 카탈로그(플러그인 활성 시 15-24 MB, 이슈 #47483, 0.156.0, OPEN), 분석 이벤트, Responses 요청 1회. 확인(이슈 #47483).
- 이슈 #47483 측정: 기본 약 23.7 MB, `plugins`, `remote_plugin`, `apps`, `tool_suggest`, `plugin_sharing`, `skill_search` 6종 비활성 시 0.33 MB. 확인(이슈).
- 신규 스레드 첫 턴 지연 사례: 이슈 #14795 (gpt-5.4, 0.113.0). howtui 호출은 매번 신규 ephemeral 스레드. 확인(이슈).
- 요청 크기: 번들 모델 카탈로그의 모델별 지침과 도구 설명(`model_messages`) 약 1.8만-6.2만 자. 매 요청에 Codex 에이전트 지침 동반, 사용자 플랜 사용량 소비 증가. 확인(소스: models-manager/models.json).
- 추정: 작은 프롬프트, `model_reasoning_effort="low"` 기준 수 초 단위. 측정 전 수치 미확인. 9장 스파이크로 p50, p95 측정 필요.

### 7.2 `$CODEX_HOME` 기록

| 기록 | 내용 | `--ephemeral` 시 | 근거 |
| --- | --- | --- | --- |
| 세션 rollout(`sessions/`) | 프롬프트와 응답 전문 | 미생성 | core/src/session/session.rs:1069-1071 |
| state DB 스레드 기록 | 스레드 메타데이터 | 미기록 | core/src/session/session.rs:1186-1188 |
| `history.jsonl` | 입력 이력 | exec 미기록(TUI 전용 기록) | tui/src/app/thread_routing.rs:564 |
| SQLite 로그 DB | 추적 로그 | exec는 `log_db: None` | exec/src/lib.rs:740 |
| 텍스트 로그 | 추적 로그 | exec는 stderr 출력, 기본 `RUST_LOG=error` | docs/install.md |
| 셸 스냅샷(`shell_snapshots/`) | 사용자 셸 환경 | `features.shell_snapshot=false`로 미생성 | core/src/session/session.rs:1476-1480 |
| `models_cache.json` 등 캐시 | 모델 카탈로그 | 생성 | models-manager/src/manager.rs:36 |
| `auth.json` 또는 keyring | 토큰 갱신 결과 | Codex 자체 기록 | login/src/auth/storage.rs |

- OpenTelemetry: `otel.log_user_prompt` 기본 꺼짐, 내보내기 기본 `none`. 확인(문서: config-reference).
- 분석 이벤트: `codex-rs/analytics/src`에서 프롬프트 원문 필드 미발견(소스 grep 기준). `analytics.enabled=false` 지정 권장. 확인(소스), 전송 내용 전수 검증은 미확인.
- 자식 환경의 `RUST_LOG` 상속 시 상세 로그 stderr 출력 가능. `RUST_LOG=error` 명시 권장.

---

## 8. 권장 명령줄과 설계

### 8.1 명령줄 (codex 0.147.0 이상)

```sh
CODEX_EXEC_SERVER_URL=none RUST_LOG=error \
codex exec \
  --ephemeral --ignore-user-config --ignore-rules --skip-git-repo-check \
  --cd "$HOWTUI_CODEX_EMPTY_DIR" --sandbox read-only --color never --json \
  --output-schema "$HOWTUI_CODEX_SCHEMA" -o "$HOWTUI_CODEX_OUT" \
  -c 'cli_auth_credentials_store="file"' \
  -c features.shell_tool=false \
  -c features.view_image=false \
  -c features.apps=false \
  -c features.plugins=false \
  -c features.remote_plugin=false \
  -c features.plugin_sharing=false \
  -c features.tool_suggest=false \
  -c features.skill_search=false \
  -c features.multi_agent=false \
  -c features.multi_agent_v2=false \
  -c features.image_generation=false \
  -c features.memories=false \
  -c features.hooks=false \
  -c features.goals=false \
  -c features.shell_snapshot=false \
  -c 'web_search="disabled"' \
  -c tools.experimental_request_user_input.enabled=false \
  -c skills.include_instructions=false \
  -c skills.bundled.enabled=false \
  -c cloud.skills.enabled=false \
  -c project_doc_max_bytes=0 \
  -c include_environment_context=false \
  -c include_permissions_instructions=false \
  -c include_apps_instructions=false \
  -c include_collaboration_mode_instructions=false \
  -c 'history.persistence="none"' \
  -c analytics.enabled=false \
  -c hide_agent_reasoning=true \
  -c 'model_reasoning_effort="low"' \
  -
```

- `cli_auth_credentials_store` 값: 5.1 탐침 결과(`file` 또는 `keyring`) 사용.
- `-m` 미지정 권장: ChatGPT 로그인 사용자 대상 GPT-5.5 제공 종료(2026-10-14) 사례처럼 모델 목록 변동. 사용자 설정 옵션으로만 노출. 확인(문서: codex-manual.md "GPT-5.5 retirement").
- stdin 내용: howtui 지침 + 사용자 질문 + OS와 셸 종류. 파일 내용, 환경 변수, 셸 이력, 디렉터리 목록 미포함.
- 대안 키 `model_instructions_file`: Codex 기본 지침 대체 가능, 요청 크기 감소 효과. 문서상 "STRONGLY DISCOURAGED". 채택 보류. 확인(소스: config/src/config_toml.rs:271-275).

### 8.2 출력 스키마

```json
{
  "type": "object",
  "additionalProperties": false,
  "required": ["command", "explanation", "notes"],
  "properties": {
    "command": { "type": "string" },
    "explanation": { "type": "string" },
    "notes": { "type": "array", "items": { "type": "string" } }
  }
}
```

- strict 모드 전달 조건상 `additionalProperties: false`, 전 필드 `required` 필수. 확인(소스: core/src/client_common.rs:52).

### 8.3 howtui 실행 절차 (6단계 파이프라인)

(1) 감지: `PATH` 탐색, `codex --version` 파싱, 0.147.0 미만이면 위임 거부와 `codex update` 안내.

(2) 인증 점검: `codex login status` 종료 코드와 첫 문구 판정, 저장소 모드 탐침. 미로그인 시 `codex login` 또는 `codex login --device-auth` 안내.

(3) 사전 점검(버전별 캐시): 8.1과 동일한 `-c` 목록으로 `codex features list` 실행, `shell_tool`, `view_image`, `apps`, `plugins`, `multi_agent`, `image_generation`, `hooks` 키 존재와 `false` 확인. 불일치 시 위임 거부. 확인(소스: cli/src/main.rs:1922-1953).

(4) 고지: `$CODEX_HOME/AGENTS.override.md`, `$CODEX_HOME/AGENTS.md` 존재와 크기만 `stat`으로 확인, 비어 있지 않으면 "Codex 전역 지침 파일이 요청에 포함" 고지 후 사용자 선택. 최초 1회 "요청이 ChatGPT 플랜 사용량 소비" 고지.

(5) 실행: Rust `std::process::Command` 기준 `env_clear()` 후 허용 목록만 전달(`PATH`, `HOME`, `USER`, `LANG`, `LC_ALL`, `CODEX_HOME`, `XDG_RUNTIME_DIR`, `DBUS_SESSION_BUS_ADDRESS`, `HTTPS_PROXY`, `HTTP_PROXY`, `NO_PROXY`, `SSL_CERT_FILE`, `CODEX_EXEC_SERVER_URL=none`, `RUST_LOG=error`), `OPENAI_API_KEY`와 `CODEX_API_KEY` 제외, stdin 파이프로 프롬프트 기록 후 닫기, cwd는 howtui 전용 0700 빈 디렉터리, 호출 측 타임아웃(예: 60초) 초과 시 프로세스 종료.

(6) 감시와 검증: JSONL 스트림에서 `command_execution`, `file_change`, `mcp_tool_call`, `collab_tool_call`, `web_search`, `todo_list` item 발생 시 즉시 종료와 결과 폐기, `agent_message` 최종 텍스트를 스키마로 재검증, 생성 명령은 표시만 수행하고 실행 없음.

### 8.4 판정

- 개인정보 보장과의 정합: (1)-(6) 적용 시 셸, 파일 열람 도구, 웹 검색, MCP, 이미지 열람, 패치 적용 경로가 플래그 차단과 실행 환경 제거로 이중 차단. 전역 AGENTS.md만 고지 대상 잔존.
- 지원 수준: 공식 텍스트 전용 모드 아님. 기능 플래그 조합과 공개 문서 미기재 환경 변수 의존. Codex 갱신마다 사전 점검 결과 재확인 필요.
- 권장 위치: howtui 자체 OAuth의 대체가 아닌 선택 가능한 공급자(`codex-cli`)로 구현, 설치와 로그인 감지 시 초보자 기본값으로 제안.

---

## 9. 후속 스파이크 항목

| 번호 | 항목 | 방법 | 판정 기준 |
| --- | --- | --- | --- |
| (1) | 도구 목록 실측 | `-c model_provider=<mock>`과 `model_providers.<mock>.base_url=http://127.0.0.1:<port>/v1`로 로컬 모의 Responses 서버 지정, 요청 본문의 `tools` 배열 기록 | 셸, 파일, 웹, MCP, 이미지 도구 부재 |
| (2) | 환경 제거 동작 | `CODEX_EXEC_SERVER_URL=none`과 `--ignore-user-config` 조합의 정상 응답 여부 | 턴 완료, 오류 없음 |
| (3) | 지연 시간 | 동일 프롬프트 20회 실행, 시작부터 최종 메시지까지 측정 | p50, p95 기록 |
| (4) | 로컬 기록 | 실행 전후 `$CODEX_HOME/sessions` 디렉터리 항목 수 비교(파일명 목록만, 내용 미열람) | 신규 항목 없음 |
| (5) | 저장소 모드 탐침 | file, keyring 사용자 각각 `login status -c` 탐침 결과 | 기존 저장소 모드 판별 일치 |
| (6) | 프롬프트 입력 점검 | `codex debug prompt-input`에 8.1 `-c` 목록 적용 | environment_context, 스킬, 프로젝트 문서 블록 부재 |

---

## 10. 출처

- openai/codex 저장소, 커밋 `c260087efe8cc6dcc0526e7018ae9e0e3fc134e3`: https://github.com/openai/codex
  - README.md, LICENSE, NOTICE, docs/install.md, docs/exec.md, docs/config.md
  - codex-rs/exec/src/cli.rs, codex-rs/exec/src/lib.rs, codex-rs/exec/src/exec_events.rs
  - codex-rs/core/src/tools/spec_plan.rs, codex-rs/core/src/tools/mod.rs, codex-rs/core/src/tools/handlers/mod.rs, codex-rs/core/src/tools/handlers/apply_patch.rs
  - codex-rs/core/src/agents_md.rs, codex-rs/codex-home/src/instructions/mod.rs, codex-rs/core/src/config/mod.rs, codex-rs/core/src/session/session.rs
  - codex-rs/config/src/config_toml.rs, codex-rs/config/src/loader/mod.rs, codex-rs/config/src/skills_config.rs, codex-rs/config/defaults.toml
  - codex-rs/features/src/lib.rs, codex-rs/protocol/src/protocol.rs, codex-rs/codex-mcp/src/mcp/mod.rs
  - codex-rs/exec-server/src/environment.rs, codex-rs/exec-server/src/environment_provider.rs
  - codex-rs/cli/src/login.rs, codex-rs/cli/src/main.rs, codex-rs/login/src/auth/storage.rs
  - codex-rs/models-manager/models.json, codex-rs/code-mode-runtime/src/runtime/globals.rs
  - sdk/typescript/README.md, sdk/typescript/src/exec.ts
- 태그별 비교: `rust-v0.136.0`, `rust-v0.146.0`, `rust-v0.147.0`의 codex-rs/features/src/lib.rs, codex-rs/core/src/tools/spec_plan.rs, codex-rs/config/src/config_toml.rs
- 릴리스: https://github.com/openai/codex/releases/tag/rust-v0.163.0, https://github.com/openai/codex/releases/tag/rust-v0.147.0
- 문서
  - https://developers.openai.com/codex/noninteractive (→ https://learn.chatgpt.com/docs/non-interactive-mode)
  - https://developers.openai.com/codex/config-reference (→ https://learn.chatgpt.com/docs/config-file/config-reference)
  - https://developers.openai.com/codex/auth (→ https://learn.chatgpt.com/docs/auth)
  - https://learn.chatgpt.com/docs/agent-approvals-security
  - https://developers.openai.com/codex/codex-manual.md (→ https://learn.chatgpt.com/docs/codex-manual.md)
- 이슈
  - https://github.com/openai/codex/issues/47483 (codex exec 콜드 스타트 원격 플러그인 카탈로그 다운로드)
  - https://github.com/openai/codex/issues/27651 (API 키 환경 변수 존재 시 ChatGPT 인증 실패)
  - https://github.com/openai/codex/issues/14795 (신규 스레드 첫 턴 지연)
- 미열람(403): https://openai.com/policies/row-terms-of-use/, https://help.openai.com/en/articles/11369540-codex
- 로컬 실행: `codex --version`, `codex --help`, `codex exec --help`, `codex login --help`, `codex login status`(종료 코드와 첫 문구만)
