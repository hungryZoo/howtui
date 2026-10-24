# howtui 개발 로드맵

- 버전: 0.1 (Phase 0 초안)
- 작성일: 2026-10-22
- 기준 문서: [MRD](MRD.md), [PRD](PRD.md), [SRS](SRS.md), [TC](TC.md)
- 원칙: 각 Phase는 독립 검증 가능 단위. 완료 조건 미충족 시 다음 Phase 착수 금지. 명세에 없는 기능은 요구사항 변경 기록 후 착수.

## 1. Phase 요약

| Phase | 목표 | 완료 조건 (게이트) | 상태 |
| --- | --- | --- | --- |
| 0 | SDD 문서와 기술 검증 | 요구사항 간 모순 없음, 기술 리스크와 미해결 항목 기록, 사용자 검토 승인 | 완료 (2026-10-22 승인) |
| 1 | Inline TUI MVP | 지정 주요 OS에서 생성, 편집, 확정, 출력 일관 동작. P1 필수 TC 통과 | 완료 (v0.1.0 공개, 2026-10-23) |
| 2 | 선택적 쉘 통합 | 3개 쉘에서 자동 실행 없이 입력란 안전 전달. P2 필수 TC 통과 | 완료 (2026-10-23, Windows 사용자 확인) |
| 3 | Fullscreen TUI | 두 모드 동일 기능, 설정 선택 가능. P3 필수 TC 통과 | 보류: 구현과 시험 완료(`fullscreen` 브랜치), 출시 보류 (E-09) |
| 4 | 배포와 1.x 안정화 | 지원 플랫폼에서 문서화된 절차로 설치, 진단, 업데이트 가능 | 완료 (2026-10-25). 실기기 미확인 (E-08), 자격증명 이전 보류 (CR-010) |
| 5 | API Key Provider | 복수 Provider를 동일 UI와 로직으로 사용 | 구현과 Mock 시험 완료 (2026-10-25, TC 22절). 실계정 추론 미확인 (TC-LLM-026) |
| 6 | 로컬 LLM | 실행 중 로컬 서버로 생성과 수정 가능 | 대기 |
| 2.0 이후 | 내장 llama.cpp, 모델 관리, 자동 모델 선택, 하드웨어 최적화 | 별도 장기 과제 | 범위 외 |

## 2. Phase 0 산출물

| 산출물 | 경로 | 상태 |
| --- | --- | --- |
| MRD | [docs/MRD.md](MRD.md) | 초안 |
| PRD | [docs/PRD.md](PRD.md) | 초안 |
| SRS | [docs/SRS.md](SRS.md) | 초안 |
| TC | [docs/TC.md](TC.md) | 초안 |
| 추적성 표 | [docs/TRACEABILITY.md](TRACEABILITY.md) | 스크립트 생성 |
| ADR | [docs/adr/](adr/) | 제안 상태 |
| 조사 자료 | [docs/research/](research/) | 완료 |
| 기술 검증 실험 | [spikes/](../spikes), [결과](research/spikes.md) | 완료 (Linux 기준) |

## 3. Phase 1 작업 순서

각 작업은 관련 TC를 먼저 작성하고 구현 후 `cargo fmt`, `cargo clippy`, `cargo test` 실행. 작업 단위로 커밋.

| 순서 | 작업 | 관련 요구사항 | 선행 조건 | 검증 TC |
| --- | --- | --- | --- | --- |
| 1-01 | ChatGPT 로그인 실계정 실측 실험 (SPIKE-06): 동적 등록, 모델 목록, 스트리밍 추론, 구조화 출력, 리다이렉트 URL 붙여넣기, 갱신, 폐기 | PRD-AUTH-001-006, PRD-LLM-009 | 사용자 계정 참여 (U-1) | TC-LLM-017, TC-AUTH-008, TC-AUTH-022 |
| 1-02 | 1-01 결과 반영: ADR-0007 확정, 실측 미확인 항목 갱신 | ADR-0007 | 1-01 | 문서 갱신 |
| 1-03 | 저장소 기반: `Cargo.toml`, 툴체인 고정, CI(fmt, clippy, test 3개 OS, 추적성 검사, grep 검사), LICENSE(MIT), 다국어 메시지 구조 | SRS-BLD-007, SRS-TEST-005, SRS-I18N-001-005 | 없음 | TC-NFR-001, TC-NFR-010, TC-I18N-001-003 |
| 1-04 | 터미널 세션 계층: 제어 터미널, fd 1 전환, 복구, 시그널 | SRS-TERM-001-006 | 1-03 | TC-OUT-001-008, TC-UI-010, TC-ERR-003, TC-ERR-004 |
| 1-05 | Windows 터미널 실측 (CONOUT$, VT 출력, stdout 리디렉션, 키 수신) | SRS-TERM-003 | 1-04, Windows 환경 (U-2) | TC-UI-014, TC-OUT-009, TC-KEY-005 |
| 1-06 | 앱 코어: State, update, keymap, 편집 버퍼 | SRS-DATA-001-006, SRS-TUI-001, SRS-TUI-002 | 1-03 | TC-UI-006, TC-UI-007, TC-KEY-001, TC-EDIT-001-008 |
| 1-07 | Inline 렌더러, 레이아웃, 크기 부족 화면 | SRS-TUI-003, SRS-TUI-004, SRS-TERM-007 | 1-06 | TC-UI-002-005, TC-UI-009 |
| 1-08 | LLM 공통: 타입, 프롬프트, 파싱, SSE, Mock Provider, 이벤트 루프와 취소 | SRS-LLM-001-004, SRS-LLM-009, SRS-ASYNC-001-006 | 1-06 | TC-LLM-001-013, TC-UI-005 |
| 1-09 | 인증: OAuth, 콜백, ID 토큰, 저장소, 갱신, 로그아웃, 상태 | SRS-AUTH-001-014, SRS-AUTH-016 | 1-02 | TC-AUTH-001-007, TC-AUTH-011-021 |
| 1-10 | ChatGPT Provider와 오류 매핑 | SRS-LLM-005-008, SRS-LLM-010 | 1-08, 1-09 | TC-LLM-014-017 |
| 1-11 | 비대화형 모드, Output Layer, 종료 코드 | SRS-OUT-001-004, SRS-ERR-001-004 | 1-08 | TC-CLI-004, TC-CLI-009, TC-CLI-010, TC-OUT-005 |
| 1-12 | 안전 규칙 | SRS-SAFE-001-007 | 1-06 | TC-SAFE-001-007 |
| 1-13 | 설정, 최소 doctor | SRS-CFG-001-006, SRS-CFG-008, SRS-DOC-001-004, SRS-DOC-006 | 1-09 | TC-CFG-001-007, TC-DOC-001-006 |
| 1-14 | 설치 스크립트, 릴리스 워크플로(초안 릴리스), README | SRS-INS-001-011, SRS-BLD-001-009 | 1-03 | TC-INS-001-007, TC-INS-010, TC-REL-001-005 |
| 1-15 | Phase 1 게이트: P1 TC 실행과 결과 기록, 추적성 갱신, 사용자 검토, v0.1.0 공개 승인 | PRD-REL-007 | 1-01-1-14 | 전체 P1 TC |

### 3.1 Phase 1 진행 상태 (2026-10-22)

| 작업 | 상태 | 비고 |
| --- | --- | --- |
| 1-01 실계정 실측 | 완료 | 사용자 실계정 붙여넣기 로그인과 명령어 생성 성공 (2026-10-22), 구조화 출력 지원 확인 |
| 1-02 실측 결과 반영 | 완료 | SRS 22절 미확정 항목 3건 해소, 사용자 피드백 2건은 CR-001, CR-002로 반영 |
| 1-03 저장소 기반 | 완료 | CI 13개 작업 통과 |
| 1-04 터미널 세션 | 완료 | Linux, macOS PTY E2E 18건, 무응답 PTY 종료 코드 3 |
| 1-05 Windows 실측 | 부분 완료 | ConPTY 자동 E2E (x64, ARM64), 설치 스크립트 PowerShell 5.1과 7. Windows Terminal, conhost, 한국어 코드 페이지 수동 확인 대기 (U-2) |
| 1-06 앱 코어 | 완료 | |
| 1-07 Inline 렌더러 | 완료 | |
| 1-08 LLM 공통 | 완료 | |
| 1-09 인증 | 완료 | Mock 인증 서버 기반 전체 흐름, OS 보안 저장소 왕복(Linux) |
| 1-10 ChatGPT Provider | 완료 | Mock API 서버 기반 |
| 1-11 비대화형 모드와 출력 | 완료 | |
| 1-12 안전 규칙 | 완료 | 양성 52건, 음성 27건 |
| 1-13 설정과 doctor | 완료 | |
| 1-14 설치와 릴리스 | 완료 | 릴리스 시험 실행 성공 (7개 타깃). 공개 릴리스 미실행 |
| 1-15 Phase 1 게이트 | 완료 | v0.1.0 공개 (사용자 승인 2026-10-23), 공개 설치 확인 6개 러너 통과 |

Phase 1 이월 항목 (요구사항 변경 없이 다음 작업으로 이월):

| 항목 | 사유 | 이월 대상 | 처리 (2026-10-24) |
| --- | --- | --- | --- |
| PRD-KEY-002 Shift+Enter (키보드 확장 플래그) | 권장 항목, 터미널별 지원 차이 실측 필요 | Phase 2 | 미처리 (권장 항목, 실측 대기) |
| TC-LLM-011 제한 시간, TC-NFR-006 TLS 자체 서명 거부, TC-NFR-007 프록시 | 시험 서버 구성 필요 | Phase 2 착수 전 | 통과. TC-NFR-007은 `HTTP_PROXY` 경로만 (`HTTPS_PROXY` CONNECT 미실행) |
| TC-NFR-008 의존성 취약점 점검 (CI) | CI 작업 미구성 | Phase 2 착수 전 | CI `audit` 작업 추가. 첫 실행에서 ctutils 취약점(RUSTSEC-2026-0385) 발견 후 0.4.5로 갱신, keyring 의존 zbus 계열 unmaintained 경고 3건은 `.cargo/audit.toml`에 사유 기록 후 예외 |
| TC-AUTH-016 TUI 로그인 E2E | Mock 인증 서버 프로세스 필요 (현재는 단위와 통합 수준 검증) | Phase 2 착수 전 | 통과 (`tests/pty_e2e.rs`) |
| 저장소 immutable releases 설정 | 저장소 관리자 설정 (사용자 작업) | v0.1.0 공개 전 | 사용자 작업 |
| 실기기 시험 (TC-PLAT-002, TC-PLAT-003, TC-PLAT-005) | 실기기 없음 | Phase 4 | 미확인. 사용자 결정 E-08 |

### 3.2 Phase 2 진행 상태 (2026-10-22)

| 작업 | 상태 | 비고 |
| --- | --- | --- |
| CR-001 응답 후 포커스 유지 | 완료 | 빈 질문 Enter 확정, ↓와 ↑ 영역 이동, 빈 질문 영역 Undo는 명령어 대상 |
| CR-002 로그인 간소화 | 완료 | TUI 시작 시 로그인 자동 시작, 붙여넣기 자동 제출, 스킴 없는 주소 허용, 2단계 안내 |
| `init` 스크립트 3종과 `--widget` | 완료 | Bash, Zsh, PowerShell. 키 변경, 바인딩 생략, 함수 생략 옵션 |
| CR-003 직접 실행 결과 다음 입력 전달 | 완료 | Zsh 버퍼 스택, Bash 터미널 상태 보고 응답, PowerShell 유휴 이벤트 |
| 쉘 통합 PTY E2E | 완료 | 로컬 Bash 5.0, 컨테이너 Bash 5.2, Zsh 5.9, PowerShell 7.4. CI ubuntu-24.04와 macos-15 전체 통과 (TC 18절) |
| Windows PowerShell 5.1, 7 대화형 확인 | 완료 | 사용자 확인 (2026-10-23): 화면, 리디렉션, Ctrl+G, 직접 실행 전달. 코드 페이지 949 조건은 미확인 |
| 쉘별 지원 상태 표기 | 완료 | README 6절 |
| CR-004 설치 시 쉘 통합 등록 | 완료 | 설치 스크립트 확인 질문(동의 시에만), `init --install`, `--uninstall`, doctor 등록 안내 |
| CR-005 설정 화면 | 완료 | Ctrl+O, `/model`, 클릭으로 모델 변경과 로그아웃, 기본 모델 `gpt-6-luna`, 선택 저장 |
| CR-006 제거 스크립트와 Windows 쉘 통합 질문 | 완료 | `howtui-uninstall.sh`, `howtui-uninstall.ps1`, PowerShell Profile 등록, README 설치와 제거 절차 |

### 3.3 Codex CLI 위임 검토 결과: 미채택 (ADR-0017)

- 출처: 사용자 제안 "codex가 깔려 있으면 bin을 바로 사용" (2026-10-22)
- 조사: [research/codex-cli-delegation.md](research/codex-cli-delegation.md)
- 판단: 사용자 위임에 따라 실험(codex 0.164.0) 후 미채택 (2026-10-23, [ADR-0017](adr/0017-no-codex-cli-delegation.md), 조사 문서 9-1절). 아래 표는 실험 전 조사 결과
- 로그인 간소화 목적은 CR-002로 우선 대응

| 항목 | 조사 결과 |
| --- | --- |
| 도구 차단 | 공식 텍스트 전용 모드 없음. `tool_choice`가 `auto`로 고정되어 있고, `--sandbox read-only`에서도 파일 읽기 허용. 기능 플래그와 문서에 없는 `CODEX_EXEC_SERVER_URL=none` 조합으로만 차단 가능 (미실측) |
| 개인정보 원칙 | `$CODEX_HOME/AGENTS.md` 항상 주입, 끌 수 있는 설정 없음. 기업 관리 설정이 `-c` 값을 재정의할 수 있음. 버전 갱신 시 기본 활성 도구 추가 가능 |
| 비용과 지연 | 호출마다 Codex 자체 지시문 18k-62k자 추가 (플랜 사용량 소모), 매 호출 모델 카탈로그 조회 |
| 버전 조건 | 0.147.0 이상 필요 (로컬 설치 0.136.0) |
| 대안 | `codex login --device-auth` 경로는 존재하나 ChatGPT 보안 설정에서 기기 코드 로그인 활성화 선행 필요 (beta) |

### 3.4 Phase 3 상태: 출시 보류 (2026-10-25)

- 구현과 시험: 완료 (2026-10-24, 단위 4건, PTY 3건, CI 15개 작업 통과, run 40073443763)
- 보관 위치: `fullscreen` 브랜치. main에는 미포함 (`--mode fullscreen`은 기존 미지원 안내 후 Inline 실행)
- 사유: 사용자 결정 E-09 (기존 기능 안정 확인, Fullscreen 출시 연기)
- 재개 절차: `fullscreen` 브랜치를 main 기준으로 재정렬 후 병합, TC-CLI-008과 TC-FS-001-005 재실행

### 3.5 Phase 5 진행 상태 (2026-10-25)

| 작업 | 상태 | 비고 |
| --- | --- | --- |
| 명세 (CR-011, SRS 5.5절) | 완료 | 에이전트 전환, 키 관리 명령, `--provider` |
| Provider 4종 (`src/llm/api.rs`) | 완료 | OpenAI Responses, Anthropic Messages, Gemini generateContent, OpenAI 호환 chat/completions |
| 키 조회와 저장 (`src/llm/keys.rs`, `auth set-key`, `auth remove-key`) | 완료 | 환경변수 > 보안 저장소, 설정 파일 키 필드 거부, 민감 헤더 |
| 에이전트 전환 (`src/llm/switch.rs`, 설정 화면) | 완료 | 선택 저장, 사용 불가 항목 안내, 대화 이력 유지 |
| doctor, `auth status` | 완료 | 선택 에이전트 기준 점검, 키 출처 표시 |
| 실계정 추론 (TC-LLM-026) | 미실행 | 사용자 API 키 필요 |

## 4. Phase 2-6 작업 개요

| Phase | 작업 | 검증 TC |
| --- | --- | --- |
| 2 | `init` 스크립트 3종, 위젯 프로토콜(`--widget`, `HOWTUI_BUFFER`, 출력 파일), 단축키, 입력란 초기값, 대체 경로, 직접 실행 결과 다음 입력 전달(CR-003), doctor 통합 점검, 쉘별 지원 버전 표기 | TC-SH-001-019, TC-EDIT-009, TC-OUT-010, TC-CLI-007 |
| 3 | Fullscreen 렌더러, Alternate Screen, 이력 스크롤, 모드 선택 | TC-FS-001-005, TC-CLI-008 |
| 4 | 7개 타깃 빌드, 출처 증명, Raspberry Pi와 Windows ARM64 실기기 검증, doctor 업데이트 점검, 업데이트와 제거 문서, 자격증명 이전, 보안 검토 | TC-REL-003, TC-REL-006, TC-PLAT-002-005, TC-DOC-007, TC-INS-008, TC-INS-009, TC-AUTH-010, TC-NFR-005 |
| 5 | OpenAI, Gemini, Anthropic, OpenAI 호환 Provider, Provider 전환, 자격증명 보안 검증 | TC-LLM-018, TC-LLM-019 |
| 6 | Ollama, llama-server 연동, 로컬 모델 설정, Gemma 4 E2B와 E4B 평가, 사용자 Unsloth 평가 반영, 플랫폼별 권장사항 | TC-LLM-020, TC-LLM-021, TC-DOC-008 |

## 5. 사용자 참여 필요 항목

| ID | 항목 | 필요 시점 |
| --- | --- | --- |
| U-1 | ChatGPT Plus 또는 Pro 계정으로 실측 로그인 (브라우저 동의 화면 진행) | Phase 1 작업 1-01 |
| U-2 | Windows 실환경 (PowerShell 5.1, PowerShell 7, Windows Terminal, conhost) 테스트 | Phase 1 작업 1-05, Phase 2 |
| U-3 | macOS 실환경 (Zsh, Terminal.app 또는 iTerm2) 테스트 | Phase 1 게이트, Phase 2 |
| U-4 | Raspberry Pi 2-5 실기기, Windows ARM64 실기기 | Phase 4 |
| U-6 | 공개 릴리스와 원격 push 승인 | 각 릴리스 |
| U-7 | Unsloth Gemma 4 모델 벤치마크 결과 제공 | Phase 6 |

## 6. 사용자 결정 사항 (2026-10-22)

| ID | 사항 | 결정 | 반영 위치 |
| --- | --- | --- | --- |
| E-01 | ChatGPT 로그인 사용 불가 판정 시 | 대체 Provider 없이 해당 기간 사용 불가. API Key Provider는 계획대로 Phase 5 | PRD-AUTH-019, ADR-0007, MRD-RSK-001 |
| E-02 | 라이선스 | MIT | `LICENSE`, `Cargo.toml` |
| E-03 | 메시지 언어 | 영어와 한국어 기본 제공, 언어 전환 가능 구조 | PRD-I18N 계열, SRS 7.5절, ADR-0016 |
| E-04 | 리다이렉트 URL 붙여넣기 로그인 | 필수 기능으로 제공 | PRD-AUTH-006, SRS-AUTH-006 |
| E-05 | 쉘 통합 실패 시 복사 방식 | 터미널 표시 후 수동 복사 | PRD-SH-008 |
| E-06 | Windows 코드 서명, macOS 공증 | 둘 다 미적용 | SRS 13.1절 |
| E-07 | 위험 높음 명령어 확정 | Enter 재확인 유지 | PRD-SAFE-003 |
| E-08 | 실기기 시험 (Raspberry Pi, Windows ARM64) (2026-10-24) | 실기기 없음. 동작 가정 후 `미확인` 표기, 실패 보고 시 다음 업데이트에서 수정 | TC-PLAT-002, TC-PLAT-003, TC-PLAT-005, TC 20절 |
| E-09 | Phase 3 Fullscreen 출시 (2026-10-25) | 출시 보류. `fullscreen` 브랜치 보관, main은 Phase 4 이후 진행 | ROADMAP 3.4절 |

Phase 0 승인 (2026-10-22). 작업 1-01(실계정 실측)은 사용자 참여 가능 시점에 수행하고, 그 전까지 공식 문서 기준으로 1-03 이후 작업을 진행. E-01 결정에 따라 1-01 결과가 다른 작업 순서를 바꾸지 않음.
