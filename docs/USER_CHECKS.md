# 사용자 확인 목록 (통합)

- 작성일: 2026-10-25
- 대상: 자동 시험으로 확인할 수 없어 미실행 또는 미확인으로 남긴 TC. 실계정, 사용자 API 키, Windows와 macOS 대화형 환경, 실기기 필요 항목
- 준비: `cargo build --release` 후 `./target/release/howtui --version` (Windows는 `target\release\howtui.exe`)
- 결과 회신 형식: 항목 번호와 통과 또는 실패, 실패 시 화면 또는 출력 전문. 결과는 [TC](TC.md) 결과 열에 기록
- 상세 절차: [MANUAL_TESTS](MANUAL_TESTS.md)

## 1. ChatGPT 계정 (Plus 이상)

| 번호 | 절차 | 확인 항목 | TC |
| --- | --- | --- | --- |
| (1) | `howtui` 실행, 질문 입력, 명령어 직접 편집, 후속 요청, Ctrl+E, Enter | 생성, 수정, 설명 3작업 성공, stdout 명령어 | TC-LLM-017 |
| (2) | 로그인 후 1시간 이상 경과, 다시 (1) | 토큰 자동 갱신 후 정상 동작, 재로그인 요구 없음 | TC-AUTH-022 |
| (3) | 원격 Linux에서 `ssh -L <포트>:127.0.0.1:<포트> host` 후 원격 `howtui login --no-browser --port <포트>`, 로컬 브라우저로 URL 열기 | 로그인 성공 | TC-AUTH-009 |

## 2. API 키 (Phase 5)

| 번호 | 절차 | 확인 항목 | TC |
| --- | --- | --- | --- |
| (4) | `howtui auth set-key openai` (붙여넣기), `howtui --provider openai` 에서 생성, 수정, 설명 | 3작업 성공, 상태줄 모델 `gpt-6-astra` 또는 선택 모델 | TC-LLM-026 |
| (5) | 같은 절차로 `anthropic`, `gemini` | 3작업 성공. 실패 시 오류 줄 전문 (요청 형식 확인용) | TC-LLM-026 |
| (6) | Ctrl+O 설정 화면에서 에이전트 전환 후 후속 요청 | 새 에이전트로 이력 포함 요청 | TC-LLM-025 (실계정 보강) |

## 3. Windows

| 번호 | 절차 | 확인 항목 | TC |
| --- | --- | --- | --- |
| (7) | Windows Terminal과 conhost에서 Alt+Enter, Ctrl+J, Ctrl+방향키, Ctrl+O, Ctrl+F 입력 | 키 동작이 README 키 표와 일치 | TC-KEY-005 |
| (8) | 한국어 Windows, PowerShell 5.1, 코드 페이지 949에서 한글 질문과 Ctrl+G | 한글 깨짐 없음, 입력란 교체 정상 | TC-SH-013 |
| (9) | 한국어 Windows에서 `HOWTUI_LANG` 없이 실행 | UI 한국어 | TC-I18N-007 |
| (10) | 공개 설치 명령(`irm ... \| iex`)을 PowerShell 5.1과 7에서 실행 | 사용자 경로 설치, PATH 등록, `howtui --version` | TC-INS-005 |

## 4. macOS와 공통 조합

| 번호 | 절차 | 확인 항목 | TC |
| --- | --- | --- | --- |
| (11) | Linux+Bash, macOS+Zsh, Windows+PowerShell 각각 생성, 편집, 확정, 출력 | 3조합 성공 | TC-PLAT-001 |
| (12) | Shift+Enter: kitty 또는 WezTerm(확장 키보드 지원)과 Terminal.app(미지원) | 지원 터미널은 줄바꿈, 미지원 터미널은 기존 동작 유지 | TC-KEY-003 |

## 5. 실기기 (사용자 결정 E-08로 미확인 표기)

| 번호 | 절차 | 확인 항목 | TC |
| --- | --- | --- | --- |
| (13) | Raspberry Pi 2 또는 3 (32비트 OS) 설치 스크립트 설치, TUI 실행, 원격 LLM 요청 | 동작, OS와 glibc 버전 기록 | TC-PLAT-002 |
| (14) | Raspberry Pi 4 또는 5 (64비트 OS) 같은 절차 | 동작 | TC-PLAT-003 |
| (15) | Raspberry Pi 2에서 `HOWTUI_LOG=debug`로 실행 후 로그의 `first frame` 시간 | 시간 기록 | TC-NFR-005 |
| (16) | Windows ARM64 장치에서 설치와 실행, ARM64 Windows의 x64 PowerShell에서 설치 스크립트 | 동작, ARM64 자산 선택 | TC-PLAT-005, TC-INS-006 |
| (17) | Raspberry Pi에서 Ollama 또는 llama-server로 Gemma 4 E2B 실행 후 `howtui --provider ollama` | 동작, 응답 시간 기록 | TC-LLM-021 (실기기 보강) |

## 6. 보류 기능과 릴리스

| 번호 | 절차 | 확인 항목 | TC |
| --- | --- | --- | --- |
| (18) | Fullscreen 출시 재개 결정 시 `fullscreen` 브랜치 병합 후 `howtui --mode fullscreen` | 대화 기록 스크롤, 종료 후 화면 복구 | TC-FS-001-005, TC-CLI-008 (출시 보류, E-09) |
| (19) | 다음 공개 릴리스 승인과 릴리스 노트 검토 | 승인 후 공개 | TC-REL-005 |
