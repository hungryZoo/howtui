# TC: howtui 테스트 케이스

- 문서 ID: TC
- 버전: 0.1 (Phase 0 초안)
- 작성일: 2026-10-22
- 상태: 사용자 검토 대기
- 상위 문서: [PRD](PRD.md), [SRS](SRS.md)
- 추적성 표: [TRACEABILITY](TRACEABILITY.md) (`uv run scripts/trace_check.py`로 생성)

## 0. 표기 규칙

- 열 구성: ID, 목적, 관련 ID, 사전 조건, 절차 및 입력, 기대 결과, 유형, 플랫폼, 결과, 이슈
- 유형: 단위, 스냅샷, 통합(바이너리 실행), PTY(가상 터미널 E2E), 실연동(실제 외부 서비스, CI 제외), 수동, 실기기
- 플랫폼: 전체, Linux, macOS, Windows, Unix(Linux와 macOS), RPi(Raspberry Pi 실기기), WinARM(Windows ARM64 실기기)
- 결과: 미실행, 통과, 실패, 차단 (실행하지 않은 테스트를 통과로 기록하지 않음)
- 공통 전제: 기본 Mock Provider는 cargo 기능 `mock-provider` 빌드와 `HOWTUI_MOCK_SCENARIO` 시나리오 파일 사용
- 위험 명령어 관련 TC는 문자열 판정만 수행, 생성 명령어 실행 금지

## 1. Phase 0 선행 검증과 TC 대응

Phase 0 실험(SPIKE)은 제품 코드 테스트가 아님. 아래 TC는 구현 후 재실행 대상이며, 실험 결과는 선행 근거로만 기록.

| SPIKE | 대응 TC | 실험 결과 |
| --- | --- | --- |
| SPIKE-01 | TC-OUT-001-007, TC-UI-008, TC-UI-009, TC-UI-010, TC-ERR-003 | 통과 (Linux, 12개 케이스) |
| SPIKE-02 | TC-PLAT-002, TC-REL-004 | 빌드와 qemu 실행 통과, 실기기 미검증 |
| SPIKE-03 | TC-SH-001, TC-SH-004, TC-SH-005 | Bash 5.0.17, 5.2.15 통과 |
| SPIKE-04 | TC-SH-002, TC-SH-004, TC-SH-005 | Zsh 5.9 통과 |
| SPIKE-05 | TC-SH-003 | PowerShell 7.4.2 Linux 컨테이너 API 동작 통과, Windows 미검증 |

---

## 2. CLI

| ID | 목적 | 관련 ID | 사전 조건 | 절차 및 입력 | 기대 결과 | 유형 | 플랫폼 | 결과 | 이슈 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TC-CLI-001 | 질문 인자 즉시 생성 | PRD-CLI-001, SRS-ARC-001, SRS-CLI-002 | Mock 시나리오(정상 응답) | PTY에서 `howtui find big files` 실행 | 질문 영역에 인자 표시, 생성 요청 1회 발생, 명령어 영역에 응답 표시 | PTY | Unix | 통과 | - |
| TC-CLI-002 | 버전 출력 | PRD-CLI-002, SRS-CLI-005 | 빌드 완료 | `howtui --version` | stdout `howtui <Cargo 버전>` + LF, 종료 코드 0 | 통합 | 전체 | 통과 | - |
| TC-CLI-003 | 하위 명령 존재 | PRD-CLI-003, PRD-CLI-004, PRD-CLI-011, SRS-CLI-001, SRS-CLI-004 | 없음 | `howtui --help` | login, logout, auth, doctor 항목과 종료 코드 요약 표시 | 통합 | 전체 | 통과 | - |
| TC-CLI-004 | 비대화형 모드 | PRD-CLI-005, PRD-OUT-008, SRS-OUT-003 | Mock 시나리오(정상) | `howtui --no-tui "list files" > out 2> err` | out에 명령어 + LF만, err에 설명 평문, 두 파일 모두 ESC(0x1B) 0건, 종료 코드 0 | 통합 | 전체 | 통과 | - |
| TC-CLI-005 | 높이 옵션 | PRD-CLI-006, SRS-TERM-007, SRS-CLI-006 | 터미널 100x30 | `howtui --height 5` | Inline 영역 5줄 | PTY | Unix | 통과 | - |
| TC-CLI-006 | 쉘 지정 옵션과 기본값 | PRD-CLI-007, SRS-SH-007 | 쉘 판정 환경변수 제거 | (a) `--shell zsh` 지정 (b) 미지정 | (a) 요청 컨텍스트 shell=Zsh (b) OS 기본 조합 값 | 단위 | 전체 | 통과 | - |
| TC-CLI-007 | init 출력 | PRD-CLI-008, SRS-SH-001 | 없음 | `howtui init bash`, `init zsh`, `init powershell` | 각 쉘 스크립트 stdout 출력, 종료 코드 0, 알 수 없는 쉘은 종료 코드 2 | 통합 | 전체 | 통과 | - |
| TC-CLI-008 | 모드 옵션 | PRD-CLI-009, PRD-FS-005, SRS-TUI-007, SRS-CLI-007 | P3 구현 | `howtui --mode fullscreen` | Alternate Screen 진입 | PTY | Unix | 미실행 | - |
| TC-CLI-009 | 종료 코드 매핑 | PRD-CLI-010, SRS-ERR-001 | 없음 | 오류 분류별 매핑 함수 호출 | 8.1절 표와 일치 | 단위 | 전체 | 통과 | - |
| TC-CLI-010 | 사용법 오류 | PRD-CLI-010, SRS-CLI-003 | 없음 | `howtui --unknown-flag` | 종료 코드 2, stdout 0바이트 | 통합 | 전체 | 통과 | - |

## 3. Inline TUI 화면

| ID | 목적 | 관련 ID | 사전 조건 | 절차 및 입력 | 기대 결과 | 유형 | 플랫폼 | 결과 | 이슈 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TC-UI-001 | 기존 출력 유지 | PRD-UI-001, SRS-ARC-003 | PTY에 기존 출력 20줄 | howtui 실행 후 취소 | 기존 20줄 화면 유지, Alternate Screen 시퀀스(`?1049h`) 미출력 | PTY | Unix | 통과 | - |
| TC-UI-002 | 높이 제한 | PRD-UI-002, SRS-TERM-007 | 터미널 높이 6 | 기본 설정 실행 | 뷰포트 6줄로 축소 | PTY | Unix | 미실행 | - |
| TC-UI-003 | 화면 영역 렌더링 | PRD-UI-003, PRD-UI-004, PRD-UI-005, PRD-UI-006, SRS-TUI-003, SRS-TUI-004 | 고정 State | TestBackend 80x10 렌더링 | 질문, 명령어, 설명, 상태줄 스냅샷 일치 | 스냅샷 | 전체 | 통과 | - |
| TC-UI-004 | 레이아웃 우선순위 | PRD-UI-002, SRS-TUI-003 | 명령어 8줄, 설명 5줄, 높이 6 | TestBackend 렌더링 | 상태줄 1, 질문 1, 명령어 4(스크롤 표시), 설명 0 | 스냅샷 | 전체 | 통과 | - |
| TC-UI-005 | 진행 표시 | PRD-UI-007, SRS-ASYNC-005 | Mock 지연 2초 | 질문 제출 | 상태줄 진행 표시와 "Esc 취소", 응답 후 진행 표시 제거 | PTY | Unix | 통과 | - |
| TC-UI-006 | 상태 전이 전체 | PRD-UI-013, SRS-TUI-001, SRS-DATA-001 | 없음 | PRD 3절 표 각 행 입력 | 각 행 다음 상태와 Effect 일치 | 단위 | 전체 | 통과 | - |
| TC-UI-007 | 포커스 규칙 | PRD-UI-012, SRS-TUI-005 | 없음 | 시작, 응답 수신, ↓, 명령어 2행에서 ↑ 2회, Tab | 응답 후 질문 영역 유지, ↓ 명령어 영역, 첫 줄 ↑ 질문 영역, 명령어 없으면 ↓ 이동 없음 | 단위 | 전체 | 통과 | - |
| TC-UI-008 | 리사이즈 | PRD-UI-008, SRS-TERM-008 | 편집 중 상태 | 100x30 → 60x30 → 100x12 → 100x30 | 각 단계 화면 재구성, 확정 출력 정상 | PTY | Unix | 통과 | - |
| TC-UI-009 | 최소 크기 미만 | PRD-UI-009, SRS-TERM-007 | 터미널 25x3 | 실행, Enter, Esc | 크기 부족 안내 1줄, Enter 무시, Esc 종료 130 | PTY | Unix | 통과 | - |
| TC-UI-010 | 종료 시 복구 | PRD-UI-010, SRS-TERM-001, SRS-TERM-006 | 없음 | 확정, Esc, Ctrl+C, 오류, panic 주입 각각 | 종료 후 `stty -a`에 icanon, echo, isig 존재, 커서 표시, Inline 영역 삭제 | PTY | Unix | 통과 | - |
| TC-UI-011 | 프롬프트 줄 중간 시작 | PRD-UI-011, SRS-TERM-005 | 커서 열 10에서 실행 | 실행 후 취소 | 시작 시 다음 줄부터 영역 표시, 종료 후 커서 열 10 원위치 | PTY | Unix | 통과 | - |
| TC-UI-012 | 로그인 자동 시작 화면 | PRD-AUTH-013, SRS-TUI-009 | 자격증명 없음, `HOWTUI_NO_BROWSER=1` | 실행, Esc, Esc | Enter 없이 로그인 진행 화면(전송 범위 고지, 단계 안내 1, 2와 URL 전체), 첫 Esc 후 로그인 필요 화면, 두 번째 Esc 종료 130 | PTY | Unix | 통과 | - |
| TC-UI-013 | 팝업 표현 문구 부재 | PRD-UI-014, SRS-TUI-013 | 없음 | `--help`, README에서 "popup", "팝업" 검색 | 팝업 오버레이 동등 표현 없음 | 수동 | 전체 | 통과 | - |
| TC-UI-015 | 커서 질의 무응답 터미널 | SRS-CMP-003, PRD-UI-001, PRD-OUT-007 | 커서 위치 질의에 응답하지 않는 PTY 하네스 | 실행 | 2초 이내 TUI 미시작, 종료 코드 3, 터미널 모드 복구, stdout 0바이트 | PTY | Unix | 통과 | - |
| TC-UI-014 | Windows Inline 렌더링 | PRD-UI-001, PRD-UI-010, SRS-TERM-003, SRS-CMP-004 | Windows Terminal, conhost | 실행, 편집, 확정, 취소 | 렌더링 정상, 종료 후 콘솔 모드 복구 | 수동 | Windows | 통과 | - |
| TC-UI-017 | 빈 질문 Enter 확정 | PRD-UI-012, SRS-TUI-005 | Mock 응답 | 응답 수신 후 질문 영역에서 Enter, 위험 높음 명령어는 Enter 2회 | 명령어 확정, 위험 높음은 재확인 후 확정 | 단위 | 전체 | 통과 | - |
| TC-UI-018 | 빈 질문 영역 Undo | PRD-EDIT-007, SRS-TUI-005 | Mock 응답 | 후속 수정 응답 후 질문 영역에서 Ctrl+Z | AI 반영 직전 명령어 복원 | 단위 | 전체 | 통과 | - |
| TC-UI-019 | 설정 화면 키보드 조작 | PRD-UI-015, SRS-TUI-014, SRS-LLM-013 | Mock Provider | Ctrl+O, `/model`, 위아래, Enter, Esc, 현재 모델 Enter, 목록 오류, 인증 오류 | 목록과 현재 모델 표시, 선택 반영과 저장 안내, Esc 닫기(종료 아님), 오류 표시, 인증 오류는 로그인 필요 화면 | 단위, PTY | 전체 | 통과 | - |
| TC-UI-020 | 클릭 입력 | PRD-UI-016, SRS-TUI-015 | Mock Provider, SGR 마우스 시퀀스 | 상태줄 `[모델]` 클릭, 항목 클릭, 질문과 명령어 칸 클릭, 휠, 종료 | 설정 화면 열림, 모델 변경, 포커스 이동, 커서 이동, 종료 후 마우스 보고 해제(`?1000l`) | 단위, PTY | Unix | 통과 | - |
| TC-UI-016 | Windows ConPTY 자동 E2E | PRD-UI-001, PRD-UI-010, SRS-TERM-003, SRS-TEST-006 | windows-2025, windows-11-arm 러너, Mock 시나리오 | `tests/conpty.rs`: 의사 콘솔에서 질문 입력, 응답 표시 확인, 확정, Ctrl+C 취소 | 화면 렌더링(한글, 이모지 포함), 확정 0, 취소 130 | PTY | Windows | 통과 | - |

## 4. 키 바인딩과 편집

| ID | 목적 | 관련 ID | 사전 조건 | 절차 및 입력 | 기대 결과 | 유형 | 플랫폼 | 결과 | 이슈 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TC-KEY-001 | 키 표 대응 | PRD-KEY-001, SRS-TUI-002 | 없음 | PRD 4절 표의 (키, 영역, 상태) 조합 입력 | 표와 동일 KeyAction | 단위 | 전체 | 통과 | - |
| TC-KEY-002 | 줄바꿈 키 실수신 | PRD-EDIT-002, PRD-KEY-001 | PTY | 명령어 영역에서 Ctrl+J, Alt+Enter | 각각 줄 추가 | PTY | Unix | 통과 | - |
| TC-KEY-003 | Shift+Enter 조건부 | PRD-KEY-002, SRS-TUI-008 | 키보드 확장 지원 터미널, 미지원 터미널 | Shift+Enter | 지원 시 줄 추가, 미지원 시 확정 동작 없음 확인 | 수동 | 전체 | 미실행 | - |
| TC-KEY-004 | 키 재정의 미제공 | PRD-KEY-003, SRS-TUI-012 | 없음 | 설정 스키마 확인 | 키 바인딩 설정 항목 없음 | 단위 | 전체 | 통과 | - |
| TC-KEY-005 | Windows 키 수신 | PRD-KEY-001, PRD-EDIT-002 | Windows Terminal, conhost | Ctrl+J, Tab, Esc, Ctrl+Z | 표와 동일 동작 | 수동 | Windows | 미실행 | - |
| TC-EDIT-001 | 기본 편집 | PRD-EDIT-001, SRS-DATA-002, SRS-TUI-011 | 빈 버퍼 | 입력, 이동, 단어 이동, 삭제 시퀀스 | 기대 문자열과 커서 위치 일치 | 단위 | 전체 | 통과 | - |
| TC-EDIT-002 | 여러 줄 편집 | PRD-EDIT-002, SRS-DATA-002, SRS-TUI-011 | 3줄 버퍼 | 상하 이동, 줄 끝 이동, 줄 병합 삭제 | 기대 결과 일치 | 단위 | 전체 | 통과 | - |
| TC-EDIT-003 | Undo, Redo | PRD-EDIT-003, SRS-DATA-003 | 없음 | 입력 묶음, 삭제, 붙여넣기 후 Undo 3회, Redo 1회 | 스냅샷 단위 복원 | 단위 | 전체 | 통과 | - |
| TC-EDIT-004 | 붙여넣기 | PRD-EDIT-004, SRS-DATA-006 | bracketed paste | 명령어 영역에 "a\r\nb" 붙여넣기 | "a\nb" 삽입, 확정 미발생 | PTY | Unix | 통과 | - |
| TC-EDIT-005 | Unicode 커서 | PRD-EDIT-005, SRS-DATA-002, SRS-TUI-011 | 없음 | "한글 😀 é(결합)" 입력 후 좌우 이동, 삭제 | grapheme 단위 이동, 표시 열 계산 일치 | 단위 | 전체 | 통과 | - |
| TC-EDIT-006 | 재생성 보존 정책 | PRD-EDIT-006, SRS-DATA-003, SRS-DATA-004 | 편집 중 상태, 사용자 수정 반영 | 후속 요청 제출, 요청 중 입력, 응답 수신, Undo | 요청 페이로드에 수정본 포함, 요청 중 입력 무시, 응답 반영, Undo 시 수정본 복원 | 단위 | 전체 | 통과 | - |
| TC-EDIT-007 | 취소 시 편집 보존 | PRD-EDIT-006 | 편집 중 상태 | 후속 요청 후 Esc | 명령어 버퍼 요청 전과 동일 | 단위 | 전체 | 통과 | - |
| TC-EDIT-008 | 설명 갱신 필요 표시 | PRD-EDIT-007, SRS-TUI-005 | 응답 반영 상태 | 명령어 1글자 수정, Ctrl+E | 수정 직후 stale 표시, Explain 요청 발생, 응답 후 표시 해제 | 단위 | 전체 | 통과 | - |
| TC-EDIT-009 | 쉘 입력란 초기값 | PRD-EDIT-008, SRS-SH-005 | P2, `HOWTUI_BUFFER="tar -x"` | 어댑터 모드 실행 | 명령어 영역 초기값 "tar -x", 포커스 질문 영역 | 단위 | 전체 | 통과 | - |

## 5. LLM

| ID | 목적 | 관련 ID | 사전 조건 | 절차 및 입력 | 기대 결과 | 유형 | 플랫폼 | 결과 | 이슈 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TC-LLM-001 | Generate 요청 구성 | PRD-LLM-001, PRD-LLM-006, SRS-LLM-002, SRS-SEC-004 | EnvContext(Linux, Bash) | 요청 생성 | instructions에 OS, 쉘 포함, input에 질문 포함, 허용 필드 외 없음 | 단위 | 전체 | 통과 | - |
| TC-LLM-002 | Revise 요청 구성 | PRD-LLM-002, PRD-LLM-007, SRS-DATA-005 | 이력 8개 | Revise 요청 생성 | 최근 6개 이력과 현재 명령어 블록 포함 | 단위 | 전체 | 통과 | - |
| TC-LLM-003 | Explain 요청과 응답 | PRD-LLM-003, SRS-LLM-001 | Mock 설명 응답 | Ctrl+E | 명령어 무변경, 설명 갱신 | 단위 | 전체 | 통과 | - |
| TC-LLM-004 | 취소와 늦은 응답 폐기 | PRD-LLM-004, SRS-ASYNC-003, SRS-ASYNC-004, SRS-LLM-010 | Mock 지연 1초 | 요청 후 0.2초에 Esc, 2초 대기 | 100ms 이내 직전 상태 복귀, 지연 응답 미반영 | 단위 | 전체 | 통과 | - |
| TC-LLM-005 | 표준 응답 변환 | PRD-LLM-005, SRS-LLM-001 | 없음 | Provider 응답 픽스처 변환 | command, explanation, notes, model 필드 일치 | 단위 | 전체 | 통과 | - |
| TC-LLM-006 | 쉘별 문법 지시 | PRD-LLM-006, SRS-LLM-002 | Bash, Zsh, PowerShell | 요청 생성 | 각 쉘 이름과 문법 지시 문구 포함 (스냅샷) | 스냅샷 | 전체 | 통과 | - |
| TC-LLM-007 | 설명 언어 지시 | PRD-LLM-008, SRS-LLM-002 | 없음 | 요청 생성 | "사용자 요청 언어" 지시 포함 | 단위 | 전체 | 통과 | - |
| TC-LLM-008 | 모델 선택 | PRD-LLM-009, SRS-LLM-005, SRS-LLM-006 | 모델 목록 픽스처(list 2개, hidden 1개) | (a) 설정 없음 (b) 설정 모델 지정 (c) 목록에 없는 설정 | (a) 첫 list 모델 (b) 설정 모델 (c) 첫 list 모델 (PRD-LLM-009 기준, doctor provider 항목에 선택 모델 표시) | 단위 | 전체 | 통과 | - |
| TC-LLM-009 | 응답 파싱 폴백 | PRD-LLM-010, SRS-LLM-003 | 픽스처 4종(JSON, 코드 블록 JSON, 일반 코드 블록, 형식 없음) | 파싱 | 1-3 성공, 4 InvalidResponse | 단위 | 전체 | 통과 | - |
| TC-LLM-010 | 형식 오류 재시도 | PRD-LLM-010, SRS-LLM-003 | Mock: 첫 응답 형식 오류, 두 번째 정상 | 질문 제출 | 요청 2회, 결과 반영 | 단위 | 전체 | 통과 | - |
| TC-LLM-011 | 제한 시간 | PRD-LLM-011, SRS-LLM-007 | Mock HTTP 서버: 첫 이벤트 지연 | 요청 | Timeout 분류, TUI 유지 | 통합 | 전체 | 미실행 | - |
| TC-LLM-012 | Mock Provider 기능 | PRD-LLM-012, SRS-LLM-009, SRS-MOD-003 | `mock-provider` 기능 빌드 | 지연, 오류, 스트림 중단 시나리오 | 시나리오대로 동작, 기본 빌드에는 `mock` 문자열 경로 부재 | 통합 | 전체 | 통과 | - |
| TC-LLM-013 | SSE 파서 | PRD-LLM-001, SRS-LLM-004 | 이벤트 픽스처(주석, 여러 줄 data, 분할 수신) | 파싱 | 이벤트 순서와 내용 일치 | 단위 | 전체 | 통과 | - |
| TC-LLM-014 | ChatGPT 요청 본문 | PRD-LLM-009, SRS-LLM-005, SRS-MOD-005 | 로컬 Mock HTTP 서버 | 요청 캡처 | `store:false`, `stream:true`, 미지원 필드 부재, Bearer 헤더 | 통합 | 전체 | 통과 | - |
| TC-LLM-015 | ChatGPT 오류 매핑 | PRD-AUTH-017, SRS-LLM-008, SRS-ERR-002 | Mock HTTP 서버 응답 13종 (research 8.6절) | 요청 | 각 `LlmError` 분류 일치 | 통합 | 전체 | 통과 | - |
| TC-LLM-016 | 구조화 출력 폴백 | SRS-LLM-005, PRD-LLM-010 | Mock: `unsupported_capability` param=text.format | 요청 | 지시문 방식 재요청, 상태 파일에 미지원 기록 | 통합 | 전체 | 통과 | - |
| TC-LLM-017 | 실계정 추론 | PRD-LLM-001, PRD-LLM-009, SRS-LLM-005 | ChatGPT Plus 이상 계정 로그인 | 모델 목록 조회, Generate, Revise, Explain | 3작업 성공, `response.completed` 수신 | 실연동 | 전체 | 미실행 | - |
| TC-LLM-018 | API Key Provider 4종 | PRD-LLM-013, PRD-LLM-014, PRD-LLM-015, PRD-LLM-016, SRS-LLM-011 | P5, Mock HTTP 서버 | Provider별 요청과 오류 | 요청 형식과 오류 매핑 일치 | 통합 | 전체 | 미실행 | - |
| TC-LLM-019 | Provider 전환 | PRD-LLM-017, SRS-LLM-011 | P5 | 설정과 CLI 옵션으로 전환 | 선택 Provider로 요청 | 통합 | 전체 | 미실행 | - |
| TC-LLM-020 | 로컬 서버 연동 | PRD-LLM-018, PRD-LLM-019, SRS-LLM-012 | P6, Ollama와 llama-server 실행 | 생성, 수정, 취소 | 두 서버 모두 성공, 취소 시 연결 종료 | 실연동 | Linux | 미실행 | - |
| TC-LLM-021 | 로컬 모델 평가 | PRD-LLM-020, SRS-LLM-012 | P6, Gemma 4 E2B와 E4B | research/local-llm.md 평가 계획 실행 | 지표별 측정값 기록 (평가 전 성능 단정 없음) | 수동 | Linux, RPi | 미실행 | - |
| TC-LLM-022 | 모델 선택 순서와 저장 | PRD-LLM-009, SRS-LLM-006, SRS-LLM-013 | Mock HTTP 서버, 숨김 모델 포함 목록 | 설정 조회, 숨김 모델 선택, 표시 모델 선택, 새 Provider로 준비 | 기본 `gpt-6-luna`, 숨김 모델 거부, 선택 저장과 다음 실행 유지, 설정 파일 값보다 우선 | 단위 | 전체 | 통과 | - |

## 6. 인증

| ID | 목적 | 관련 ID | 사전 조건 | 절차 및 입력 | 기대 결과 | 유형 | 플랫폼 | 결과 | 이슈 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TC-AUTH-001 | 인가 URL 구성 | PRD-AUTH-001, PRD-AUTH-002, SRS-AUTH-001 | 고정 난수 주입 | 최초 등록 URL, 재인가 URL 생성 | 파라미터 13종 규칙 일치, 재인가 시 `agent_name_hint` 부재 | 단위 | 전체 | 통과 | - |
| TC-AUTH-002 | PKCE 계산 | PRD-AUTH-001, SRS-AUTH-001 | RFC 7636 부록 B 벡터 | challenge 계산 | 기대값 일치 | 단위 | 전체 | 통과 | - |
| TC-AUTH-003 | 콜백 검증 | PRD-AUTH-001, SRS-AUTH-003 | 로컬 리스너 | 정상, state 불일치, 경로 불일치, `error=access_denied`, 다른 client_id | 정상만 성공, 나머지 거부와 코드 교환 미발생 | 통합 | 전체 | 통과 | - |
| TC-AUTH-004 | host ID 유지 | PRD-AUTH-003, SRS-AUTH-002 | 상태 파일 없음 | 로그인, 로그아웃, 재로그인, `--forget` | host ID 1회 생성 후 불변 | 단위 | 전체 | 통과 | - |
| TC-AUTH-005 | ID 토큰 검증 | PRD-AUTH-004, SRS-AUTH-004 | 테스트 RSA 키와 JWKS | 정상, 서명 불일치, aud 불일치, 만료, nonce 불일치 | 정상만 통과 | 단위 | 전체 | 통과 | - |
| TC-AUTH-006 | scope 미부여 | PRD-AUTH-004, PRD-AUTH-012, SRS-AUTH-011 | 토큰 응답 scope에 플랜 scope 없음 | 로그인 완료 처리 | 로그인 유지, 플랜 비활성 안내, 재동의 경로 안내 | 단위 | 전체 | 통과 | - |
| TC-AUTH-007 | 헤드리스 옵션 | PRD-AUTH-005, SRS-AUTH-005 | 없음 | `howtui login --no-browser --port 18765` | 브라우저 미실행, URL과 SSH 포트 포워딩 안내 표시, 127.0.0.1:18765 리스닝 | 통합 | 전체 | 통과 | - |
| TC-AUTH-008 | 리다이렉트 URL 붙여넣기 (실계정) | PRD-AUTH-006, SRS-AUTH-006 | 실계정, `--no-browser` | 다른 장치 브라우저 로그인 후 주소창 URL 붙여넣기 | 로그인 성공 여부와 OpenAI 측 응답 기록 | 실연동 | 전체 | 통과 | - |
| TC-AUTH-023 | 붙여넣기 URL 검증 | PRD-AUTH-006, SRS-AUTH-006 | Mock 토큰 서버 | 정상 URL, state 불일치, 호스트 `localhost`, 경로 불일치, 공백과 따옴표 포함 URL, 스킴 없는 주소, 줄바꿈 포함 주소 입력 | 정상, 공백 제거, 스킴 보완 URL만 코드 교환, 나머지 거부 | 단위 | 전체 | 통과 | - |
| TC-AUTH-025 | 콜백 주소 붙여넣기 자동 제출 | PRD-AUTH-006, SRS-AUTH-006 | 로그인 진행 상태 | `/auth/callback` 포함 문자열 붙여넣기, 그 외 문자열 붙여넣기 후 Enter | 콜백 주소는 Enter 없이 제출, 그 외 문자열은 Enter로 제출 | 단위 | 전체 | 통과 | - |
| TC-AUTH-024 | 플랜 사용 불가 처리 | PRD-AUTH-019, SRS-AUTH-017 | Mock 응답: scope 미부여, not_eligible, client_not_enabled, 지역 403 | 질문 제출, `--no-tui` 실행 | 원인별 메시지, 요청 중단, 다른 Provider 요청 없음, `--no-tui` 종료 코드 5 또는 7 | 통합 | 전체 | 통과 | - |
| TC-AUTH-009 | SSH 포트 포워딩 로그인 | PRD-AUTH-005, SRS-AUTH-005 | 원격 Linux와 로컬 브라우저 | `ssh -L` 후 원격 `howtui login --no-browser --port N` | 로그인 성공, 원격 장치 host ID 사용 | 실연동 | Linux, RPi | 미실행 | - |
| TC-AUTH-010 | 자격증명 이전 | PRD-AUTH-007, SRS-AUTH-015 | P4 | 내보내기 후 다른 장치 가져오기 | 대상 host ID 유지, 원본 사용 중지 안내 | 수동 | 전체 | 미실행 | - |
| TC-AUTH-011 | 저장소 우선순위와 대체 | PRD-AUTH-008, SRS-AUTH-007, SRS-MOD-006 | (a) 보안 저장소 사용 가능 (b) Linux Secret Service 없음 | 로그인 | (a) 보안 저장소 저장 (b) 0600 파일 저장과 고지 | 통합 | 전체 | 통과 | - |
| TC-AUTH-012 | 갱신 조건 | PRD-AUTH-009, SRS-AUTH-008 | 만료 30초 전 토큰, earliest_refresh_at 미래/과거 | 요청 전 갱신 판정 | 과거: 갱신, 미래: 보류 | 단위 | 전체 | 통과 | - |
| TC-AUTH-013 | 갱신 직렬화 | PRD-AUTH-009, SRS-AUTH-008 | Mock 토큰 서버, 프로세스 2개 동시 실행 | 동시 갱신 | 토큰 서버 갱신 요청 1회, 두 프로세스 같은 새 토큰 사용 | 통합 | 전체 | 통과 | - |
| TC-AUTH-014 | 로그아웃 | PRD-AUTH-010, SRS-AUTH-009 | Mock revoke 서버 (200, 5xx) | `howtui logout` | 200: 토큰 삭제, 5xx: 재시도 후 삭제와 미확인 고지, client 매핑과 host ID 유지 | 통합 | 전체 | 통과 | - |
| TC-AUTH-015 | 상태 출력 마스킹 | PRD-AUTH-011, SRS-AUTH-010, SRS-AUTH-012 | 로그인 상태 | `howtui auth status` | 마스킹 이메일, 만료 시각, 토큰 문자열 부재 | 통합 | 전체 | 통과 | - |
| TC-AUTH-016 | TUI 내 로그인 | PRD-AUTH-013, SRS-TUI-009 | 자격증명 없음, Mock 인증 서버 | Enter, 콜백 수신 | 로그인 진행 표시 후 입력 대기 상태 | PTY | Unix | 미실행 | - |
| TC-AUTH-017 | 브랜딩 문구 | PRD-AUTH-014, SRS-TUI-010 | 최초 로그인 | 로그인 완료 후 재실행 | 최초 1회 안내, 이후 미표시, 상태줄 플랜 표시 | 스냅샷 | 전체 | 통과 | - |
| TC-AUTH-018 | 개인정보 고지 | PRD-AUTH-015, PRD-PRIV-003, SRS-AUTH-013 | 자격증명 없음 | `howtui login` 시작 | 수신 정보, 저장 위치, 전송 범위 고지 표시 | 통합 | 전체 | 통과 | - |
| TC-AUTH-019 | 비공식 경로 부재 | PRD-AUTH-016, SRS-AUTH-014 | 소스 트리 | CI grep: `.codex`, `backend-api`, 쿠키 저장소 경로 | 검색 결과 0건 | 단위 | 전체 | 통과 | - |
| TC-AUTH-020 | 일시 장애 시 보존 | PRD-AUTH-018, SRS-AUTH-016, SRS-ERR-004 | Mock 503, 연결 실패 | 갱신과 추론 | 자격증명 유지, 재시도 2회 | 통합 | 전체 | 통과 | - |
| TC-AUTH-021 | 갱신 종결 오류 | PRD-AUTH-017, SRS-AUTH-008 | Mock `invalid_grant` 등 6종 | 갱신 | 토큰 삭제, 재로그인 안내 | 통합 | 전체 | 통과 | - |
| TC-AUTH-022 | 실계정 전체 흐름 | PRD-AUTH-001, PRD-AUTH-009, PRD-AUTH-010, SRS-AUTH-001 | Plus 이상 계정 | 로그인, 추론, 1시간 후 갱신, 로그아웃 | 전 단계 성공, 결과와 미확인 항목 기록 | 실연동 | 전체 | 미실행 | - |

## 7. stdout 출력 계약

| ID | 목적 | 관련 ID | 사전 조건 | 절차 및 입력 | 기대 결과 | 유형 | 플랫폼 | 결과 | 이슈 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TC-OUT-001 | 파이프 순수성 | PRD-OUT-001, PRD-OUT-006, SRS-OUT-001, SRS-TERM-002, SRS-TEST-006 | Mock 여러 줄 Unicode 응답 | PTY에서 `howtui \| od -c`, 확정 | 명령어 바이트 + LF 1개, ESC 0건 | PTY | Unix | 통과 | - |
| TC-OUT-002 | 명령 치환 | PRD-OUT-005, PRD-OUT-006 | 동일 | `x=$(howtui)` 후 `printf '%s' "$x" \| od -c` | 바이트 단위 일치 | PTY | Unix | 통과 | - |
| TC-OUT-003 | 파일 리디렉션 | PRD-OUT-006 | 동일 | `howtui > f` | f에 명령어 + LF만 | PTY | Unix | 통과 | - |
| TC-OUT-004 | 취소 무출력 | PRD-OUT-003, SRS-OUT-002 | 동일 | Ctrl+C, Esc 각각 | stdout 0바이트, 종료 코드 130 | PTY | Unix | 통과 | - |
| TC-OUT-005 | 오류 무출력 | PRD-OUT-004, SRS-OUT-002 | Mock 오류 후 Ctrl+C, `--no-tui` 오류 | 실행 | stdout 0바이트, stderr 평문, 종료 코드 매핑 일치 | 통합 | 전체 | 통과 | - |
| TC-OUT-006 | 제어 터미널 없음 | PRD-OUT-007, SRS-TERM-004 | 제어 터미널 없는 프로세스 (`setsid -w`, CI 비대화형) | `howtui < /dev/null` | 종료 코드 3, stdout 0바이트, stderr ESC 0건 | 통합 | Unix | 통과 | - |
| TC-OUT-007 | stdin 파이프 | PRD-OUT-006 | PTY | `echo x \| howtui` | TUI 정상 동작, 확정 출력 정상 | PTY | Unix | 통과 | - |
| TC-OUT-008 | 렌더링 대상 분리 | PRD-OUT-002, SRS-TERM-001 | PTY, stderr 파일 리디렉션 | `howtui 2> e \| cat > o` | e 0바이트(TUI 미출력), o 명령어만 | PTY | Unix | 통과 | - |
| TC-OUT-009 | Windows 리디렉션 | PRD-OUT-001, PRD-OUT-006, SRS-TERM-003 | Windows Terminal | `howtui > out.txt`, `$x = howtui` | out.txt 명령어 UTF-8 + LF, TUI 정상 표시 | 수동 | Windows | 통과 | - |
| TC-OUT-010 | 출력 파일 대상 | PRD-OUT-009, SRS-OUT-005 | P2 | `--output-file` 지정 확정과 취소 | 확정: 파일에 명령어(UTF-8, LF 미추가), 취소: 파일 미생성 | 통합 | 전체 | 미실행 | - |
| TC-OUT-011 | 로그 비출력 | PRD-OUT-010, SRS-OUT-004, PRD-PRIV-005 | `HOWTUI_LOG=debug` | 확정 실행 | stdout과 stderr에 로그 없음, 로그 파일에 토큰과 질문 본문 없음 | 통합 | 전체 | 통과 | - |
| TC-OUT-012 | Windows stdout 리디렉션 자동 검증 | PRD-OUT-001, PRD-OUT-006, SRS-TERM-003 | ConPTY, `cmd /c howtui > out` 와 `> out 2> err` | `tests/conpty.rs` 2개 경로 실행 후 확정 | out에 명령어 UTF-8 + LF만, ESC 0건, err 0바이트 (TUI는 stderr 콘솔 또는 CONOUT$) | PTY | Windows | 통과 | - |

## 8. 쉘 통합

| ID | 목적 | 관련 ID | 사전 조건 | 절차 및 입력 | 기대 결과 | 유형 | 플랫폼 | 결과 | 이슈 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TC-SH-001 | Bash 삽입 | PRD-SH-001, PRD-SH-006, SRS-SH-002 | Bash 4.0 이상, init 등록 | 입력 "ls -l" 후 단축키, 질문, 확정. 화면 맨 아래에서 반복 | 명령어 초기값 "ls -l", 입력란 교체, 커서 끝, 미실행. 화면 스크롤 후 입력 줄 1개 | PTY | Linux | 통과 | - |
| TC-SH-002 | Zsh 삽입 | PRD-SH-002, PRD-SH-006, SRS-SH-003 | Zsh 5.9, emacs 키맵 | 동일 | 동일 | PTY | Unix | 통과 | - |
| TC-SH-003 | PowerShell 삽입 | PRD-SH-003, PRD-SH-006, SRS-SH-004, SRS-SH-011 | PowerShell 5.1과 7, PSReadLine | 입력 후 단축키, 확정 | 입력란 교체, 미실행 | 수동 | Windows | 통과 | - |
| TC-SH-004 | 단축키와 변경 | PRD-SH-004, SRS-SH-006 | init 기본값, 키 지정 옵션 | 기본 키, 지정 키 입력 | 각각 호출 | PTY | Unix | 통과 | - |
| TC-SH-005 | 취소 시 보존 | PRD-SH-005, PRD-SH-007, SRS-SH-005 | 입력 "echo keep" | 단축키 후 Esc | 입력란 무변경 | PTY | Unix | 통과 | - |
| TC-SH-006 | 오류 시 보존 | PRD-SH-007, SRS-SH-009 | Mock 오류 후 종료, 바이너리 경로 오류 | 단축키 | 입력란 무변경, 원인 1줄 표시 | PTY | Unix | 통과 | - |
| TC-SH-007 | 대체 경로 | PRD-SH-008, SRS-SH-009 | TUI 실행 중 출력 파일 권한 제거 (`chmod 000`) | 확정 | 원인 1줄과 명령어 터미널 표시, 입력란 무변경 | PTY | Unix | 통과 | - |
| TC-SH-008 | 등록과 제거 안내 | PRD-SH-009, SRS-SH-010 | 없음 | init 출력 확인 | 등록 줄과 제거 방법 주석 포함 | 단위 | 전체 | 통과 | - |
| TC-SH-009 | 설치 시 rc 무변경 | PRD-SH-010, PRD-INS-001, SRS-INS-005 | 임시 HOME | install.sh 실행 (터미널 없음, 질문에 n, `--no-shell-integration`) | `.bashrc`, `.zshrc`, `.profile` 내용 불변, 터미널 없음은 등록 명령 안내 | 통합 | Unix | 통과 | - |
| TC-SH-010 | 통합 표식 | PRD-SH-011, PRD-DOC-006, SRS-SH-008, SRS-DOC-005 | init 등록 쉘 | `howtui doctor` | integration 항목에 쉘 종류 표시 | PTY | Unix | 통과 | - |
| TC-SH-011 | 지원 상태 표기 | PRD-SH-012, SRS-TEST-004, SRS-SH-012 | Phase 2 테스트 완료 | 문서 확인 | 쉘별 검증 버전과 결과 기재 | 수동 | 전체 | 미실행 | - |
| TC-SH-012 | 위젯 출력 경로 | PRD-OUT-009, SRS-SH-005 | 위젯 모드 (3개 쉘 공통 출력 파일) | 단축키 후 확정, `HOWTUI_OUTPUT_FILE` 없이 `--widget` 실행 | 출력 파일로만 전달(화면에 명령어 출력 없음), 출력 파일 미지정 시 종료 코드 2 | 통합 | 전체 | 통과 | - |
| TC-SH-013 | PowerShell 한글 인코딩 | PRD-SH-003, SRS-SH-004 | 한국어 Windows, PowerShell 5.1, 코드 페이지 949 | 한글 이모지 포함 명령어 확정 | 입력란 문자 손상 없음 | 수동 | Windows | 미실행 | - |
| TC-SH-015 | 쉘 하한 미만 처리 | SRS-CMP-002, PRD-SH-001, PRD-SH-003 | Bash 3.2 (macOS 기본), PSReadLine 미로드 PowerShell | init 스크립트 로드, `howtui` 직접 실행 후 확정, ↑ | 바인딩 생략과 경고 1줄, 쉘 정상 동작, 다음 입력란 비어 있음, ↑로 명령어 호출 | PTY | macOS, Windows | 통과 | - |
| TC-SH-016 | 직접 실행 결과 다음 입력 전달 | PRD-SH-013, SRS-SH-013, SRS-SH-014 | init 등록 (Bash, Zsh, PowerShell) | 프롬프트에서 `howtui show disk usage` 실행, 확정 | 다음 프롬프트 입력란에 명령어, 미실행 | PTY | Unix | 통과 | - |
| TC-SH-017 | 캡처와 취소 시 동작 유지 | PRD-SH-013, SRS-SH-013, PRD-OUT-002 | init 등록 | 쉘 변수 캡처로 `howtui` 실행 후 확정, `howtui` 실행 후 Esc | 캡처 변수에 명령어, 다음 입력란 비어 있음. 취소 시 종료 코드 130 전달, 입력란 비어 있음 | PTY | Unix | 통과 | - |
| TC-SH-018 | PowerShell 7 자동 E2E (Linux) | PRD-SH-003, SRS-SH-004 | Linux pwsh 7.4, PSReadLine 2.3.5 | TC-SH-001, 004-007, 012, 014, 016, 017과 같은 절차 | 같은 기대 결과 | PTY | Linux | 통과 | - |
| TC-SH-019 | Bash 버전별 스크립트 적재 | SRS-SH-002, SRS-CMP-002 | Bash 3.2, 4.0, 4.4, 5.1, 5.2 컨테이너 | `bash -n`, 대화형 쉘에서 init 스크립트 적재 | 문법 오류 없음, 3.2만 경고 1줄, 함수 정의 | 통합 | Linux | 통과 | - |
| TC-SH-020 | rc 파일 표식 구간 편집 | PRD-SH-014, SRS-SH-015 | 임시 HOME, 기존 rc 내용 | `init zsh --install` 2회, `--key alt+h --install`, `--uninstall`, 심볼릭 링크 rc, PowerShell `--install` | 구간 1개, 재실행은 "이미 있음", 키 변경은 구간 교체, 제거 후 원래 내용, 링크와 권한 유지, PowerShell 종료 코드 2 | 통합 | Unix | 통과 | - |
| TC-SH-021 | PowerShell Profile 등록 | PRD-SH-014, SRS-SH-015 | 임시 HOME, Profile 폴더 없음 | `init powershell --install`, `--uninstall`, 키 옵션 | Profile 폴더 생성, ASCII 구간, 제거 후 빈 파일, Windows는 7과 5.1 두 경로 | 단위, 통합 | 전체 | 통과 | - |
| TC-SH-014 | 여러 줄 삽입 | PRD-SH-006, SRS-SH-002, SRS-SH-003, SRS-SH-004 | 3개 쉘 | 2줄 명령어 확정 | 입력란 2줄 유지 (PowerShell 연속 프롬프트) | PTY | 전체 | 통과 | - |

## 9. Fullscreen

| ID | 목적 | 관련 ID | 사전 조건 | 절차 및 입력 | 기대 결과 | 유형 | 플랫폼 | 결과 | 이슈 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TC-FS-001 | Alternate Screen | PRD-FS-001, PRD-FS-004, SRS-TUI-007 | P3 | fullscreen 실행 후 확정 | 진입(`?1049h`)과 해제(`?1049l`), 종료 후 기존 화면 복구 | PTY | Unix | 미실행 | - |
| TC-FS-002 | 기능 동등성 | PRD-FS-002, SRS-ARC-003 | P3 | TC-UI-006 상태 전이 시나리오를 fullscreen 렌더러로 실행 | Inline과 동일 상태 결과 | 단위 | 전체 | 미실행 | - |
| TC-FS-003 | 긴 내용 레이아웃 | PRD-FS-003, SRS-TUI-007 | 이력 10개, 설명 30줄 | TestBackend 렌더링, 스크롤 | 이력 스크롤 동작, 스냅샷 일치 | 스냅샷 | 전체 | 미실행 | - |
| TC-FS-004 | 모드 선택 우선순위 | PRD-FS-005, PRD-CFG-004, SRS-CFG-007 | 설정 fullscreen, CLI inline | 실행 | CLI 값 우선 | 단위 | 전체 | 미실행 | - |
| TC-FS-005 | 리사이즈와 복구 | PRD-FS-004 | P3 | 리사이즈, panic 주입 | 화면 재구성, 종료 후 기존 화면 복구 | PTY | Unix | 미실행 | - |

## 10. 설정

| ID | 목적 | 관련 ID | 사전 조건 | 절차 및 입력 | 기대 결과 | 유형 | 플랫폼 | 결과 | 이슈 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TC-CFG-001 | 경로 규칙 | PRD-CFG-001, SRS-CFG-001 | 환경변수 조합 | XDG 설정, 미설정, `HOWTUI_CONFIG` | 12.1절 경로 일치 | 단위 | 전체 | 통과 | - |
| TC-CFG-002 | 파일 부재 기본값 | PRD-CFG-002, SRS-CFG-003 | 설정 없음 | 로드 | 기본값, 파일 미생성 | 단위 | 전체 | 통과 | - |
| TC-CFG-003 | 스키마 검증 | PRD-CFG-003, SRS-CFG-002 | `inline_height = 2`, `41`, `"a"`, `language = "fr"` | 로드 | 값 오류와 키 이름 포함 메시지 | 단위 | 전체 | 통과 | - |
| TC-CFG-004 | 문법 오류 | PRD-CFG-005, SRS-CFG-004 | 잘못된 TOML | `howtui --no-tui q` | 경로, 행, 열 포함 메시지, 종료 코드 8 | 통합 | 전체 | 통과 | - |
| TC-CFG-005 | 알 수 없는 키 | PRD-CFG-006, SRS-CFG-005 | `[ui] colour = 1` | 실행과 doctor | 실행 계속, doctor 경고 | 통합 | 전체 | 통과 | - |
| TC-CFG-006 | 비밀값 필드 부재 | PRD-CFG-007, SRS-CFG-006 | 없음 | 스키마 필드 목록 검사 | token, key, secret, password 이름 필드 부재 | 단위 | 전체 | 통과 | - |
| TC-CFG-007 | 권한 | PRD-PRIV-006, SRS-CFG-008, SRS-SEC-003 | 데이터 디렉터리 없음 | 로그인 처리 | 디렉터리 0700, 파일 0600 | 통합 | Unix | 통과 | - |

## 11. doctor

| ID | 목적 | 관련 ID | 사전 조건 | 절차 및 입력 | 기대 결과 | 유형 | 플랫폼 | 결과 | 이슈 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TC-DOC-001 | 기본 점검 항목 | PRD-DOC-001, SRS-DOC-001 | 로그인 상태 | `howtui doctor` | P1 점검 항목 전체 출력 | 통합 | 전체 | 통과 | - |
| TC-DOC-002 | 렌더링 점검 | PRD-DOC-002, SRS-DOC-001 | PTY와 비대화형 두 환경 | `howtui doctor` | PTY: render 통과, 비대화형: tty 실패와 render skip | 통합 | Unix | 통과 | - |
| TC-DOC-003 | Provider 점검 | PRD-DOC-003, SRS-DOC-006 | Mock HTTP 서버 | `howtui doctor` | 모델 목록 요청 1회, 추론 요청 0회 | 통합 | 전체 | 통과 | - |
| TC-DOC-004 | JSON 출력 | PRD-DOC-004, SRS-DOC-002 | 없음 | `howtui doctor --json` | JSON 파싱 성공, `schema_version` 1, 상태 값 집합 일치 | 통합 | 전체 | 통과 | - |
| TC-DOC-005 | 마스킹 | PRD-DOC-005, SRS-DOC-003 | 토큰, 이메일, 홈 경로, 비밀 환경변수 설정 | `doctor`, `doctor --json` | 토큰 0건, 이메일 마스킹, `~` 치환, 환경변수 값 부재 | 단위 | 전체 | 통과 | - |
| TC-DOC-006 | 종료 코드 | PRD-DOC-009, SRS-DOC-004 | 실패 항목 유무 | doctor 실행 | 실패 있음 1, 경고만 0 | 통합 | 전체 | 통과 | - |
| TC-DOC-007 | 업데이트 점검 | PRD-DOC-007, SRS-DOC-005 | P4, Mock 릴리스 응답 | doctor | 최신 버전 비교와 업데이트 방법 표시 | 통합 | 전체 | 미실행 | - |
| TC-DOC-008 | 로컬 LLM 점검 | PRD-DOC-008, SRS-DOC-005 | P6 | doctor | 서버별 연결과 모델 상태 표시 | 실연동 | Linux | 미실행 | - |

## 12. 설치와 릴리스

| ID | 목적 | 관련 ID | 사전 조건 | 절차 및 입력 | 기대 결과 | 유형 | 플랫폼 | 결과 | 이슈 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TC-INS-001 | install.sh 정상 설치 | PRD-INS-001, PRD-INS-005, SRS-INS-001, SRS-INS-004, SRS-INS-005, SRS-INS-006 | 임시 HOME, 테스트 릴리스 | `sh install.sh` | `~/.local/bin/howtui` 설치, 체크섬 검증 통과, 버전 출력 확인, sudo 미사용 | 통합 | Unix | 통과 | - |
| TC-INS-002 | 버전 고정과 재설치 | PRD-INS-001, SRS-INS-003 | 기존 설치 | `HOWTUI_VERSION=x.y.z`, `--version x.y.z` 재실행 | 지정 버전으로 교체 | 통합 | Unix | 통과 | - |
| TC-INS-003 | 체크섬 불일치 | PRD-INS-001, PRD-INS-009, SRS-INS-004, SRS-SEC-006, SRS-INS-007 | 변조 자산 | 설치 | 설치 중단, 임시 파일 삭제, 기존 바이너리 유지 | 통합 | Unix | 통과 | - |
| TC-INS-004 | 아키텍처 판별 | PRD-INS-003, PRD-INS-004, SRS-INS-002 | `uname` 출력 모의 (x86_64, aarch64, armv7l, armv6l, aarch64 커널 + armhf 사용자 공간, riscv64) | 판별 함수 실행 | 표와 일치, armv6l과 riscv64는 미지원 종료 | 단위 | Unix | 통과 | - |
| TC-INS-005 | install.ps1 정상 설치 | PRD-INS-002, SRS-INS-008, SRS-INS-010 | Windows PowerShell 5.1, PowerShell 7 | `irm ... \| iex` 동등 실행 | 사용자 경로 설치, PATH 등록, 버전 확인 | 수동 | Windows | 통과 | - |
| TC-INS-006 | install.ps1 아키텍처 | PRD-INS-002, SRS-INS-009 | ARM64 Windows의 x64 PowerShell | 판별 | aarch64 자산 선택 | 수동 | WinARM | 미실행 | - |
| TC-INS-007 | 대안 설치 문서 | PRD-INS-006, SRS-INS-011 | 릴리스 존재 | 문서 절차 수행 (스크립트 검토 후 실행, 직접 다운로드, 체크섬 독립 검증) | 각 절차 성공 | 수동 | 전체 | 미실행 | - |
| TC-INS-008 | 업데이트 | PRD-INS-007, SRS-INS-012 | P4, 구버전 설치 | 같은 방식 재실행 | 기존 경로에 신버전 | 통합 | Unix | 미실행 | - |
| TC-INS-009 | 제거 문서 | PRD-INS-008, SRS-INS-013 | P4 | 문서 절차 수행 | 바이너리, 설정, 자격증명, 쉘 통합 줄 제거 | 수동 | 전체 | 미실행 | - |
| TC-INS-010 | 원격 코드 실행 부재 | PRD-INS-009, SRS-INS-004 | 스크립트 원문 | 다운로드 대상 검사 | 릴리스 자산과 SHA256SUMS 외 다운로드 없음, 다운로드 내용 실행 없음 | 단위 | 전체 | 통과 | - |
| TC-INS-011 | 설치 후 쉘 통합 확인 질문 | PRD-INS-001, PRD-SH-014, SRS-INS-014 | 임시 HOME, PATH 밖 설치 디렉터리 | 질문에 y 2회, `--shell-integration`(zsh), `--no-shell-integration`, `cat install.sh \| sh`로 실제 터미널 질문에 Enter | 구간 1개와 PATH 추가 줄, 재실행 중복 없음, zsh는 `.zshrc`, 거부 옵션은 무변경, 파이프 실행에서도 질문 표시 | 통합 | Unix | 통과 | - |
| TC-INS-012 | 제거 스크립트 | PRD-INS-008, SRS-INS-013 | 임시 HOME, 설치와 쉘 통합 상태, 데이터와 설정 파일 | 설치 후 `howtui-uninstall.sh` 존재 확인, 터미널 없이 실행, `-y` 실행 | 확인 없이는 중단, 바이너리와 스크립트와 rc 구간과 데이터 삭제, 설정 유지 | 통합 | Unix | 통과 | - |
| TC-INS-013 | Windows 설치 질문과 제거 스크립트 | PRD-INS-002, PRD-INS-008, SRS-INS-015, SRS-INS-013 | Windows PowerShell 5.1과 PowerShell 7, 시험용 Profile 경로 | 질문에 n, yes 2회, `howtui-uninstall.ps1` 확인 없이와 `-Yes` 실행 | 거절 시 Profile 미작성, 구간 1개, 확인 없이는 중단, 실행 후 설치 폴더와 PATH 항목과 Profile 구간과 데이터 삭제 | 통합 | Windows | 통과 | - |
| TC-REL-001 | 태그 릴리스 | PRD-REL-001, PRD-REL-007, SRS-BLD-008 | 테스트 저장소 또는 초안 릴리스 | `vX.Y.Z` 태그 | 빌드 성공, 초안 릴리스 생성, 공개는 수동 | 수동 | 전체 | 통과 | - |
| TC-REL-002 | 자산 명명과 체크섬 | PRD-REL-002, SRS-BLD-006 | 릴리스 산출물 | 파일명 규칙 검사, `sha256sum -c SHA256SUMS` | 규칙 일치, 검증 통과 | 통합 | Linux | 통과 | - |
| TC-REL-003 | 7개 타깃 | PRD-REL-003, PRD-PLAT-002, SRS-BLD-001, SRS-BLD-003, SRS-BLD-004, SRS-BLD-009, SRS-CMP-005 | P4 | 릴리스 산출물 검사 (`file`, ELF 속성, PE 머신 타입과 임포트 DLL, Mach-O minos와 `codesign -v`) | 7개 타깃 형식 일치, Windows VCRUNTIME 임포트 없음, macOS minos 11.0과 서명 유효 | 통합 | Linux, macOS, Windows | 통과 | - |
| TC-REL-007 | CI 워크플로 동작 | SRS-BLD-007, PRD-REL-001 | 테스트 PR | PR 생성 | fmt, clippy, 5개 러너 테스트, armv7 교차 빌드와 GLIBC 검사, 추적성 검사, 설치 스크립트 구문 검사 실행과 결과 표시 | 통합 | 전체 | 미실행 | - |
| TC-REL-008 | 공개 릴리스 설치 확인 | PRD-INS-001, PRD-INS-002, PRD-INS-008, SRS-INS-013 | 공개 릴리스, GitHub 러너 (ubuntu x64와 arm64, macos, windows x64 5.1과 7, windows arm64) | `.github/workflows/release-smoke.yml`: 공개 설치 명령, `--version`, 제거 스크립트 | 설치와 제거 성공 | 통합 | 전체 | 통과 | - |
| TC-REL-004 | 최소 타깃과 glibc 하한 | PRD-REL-004, SRS-BLD-002, SRS-CMP-001, SRS-MOD-007 | P1 릴리스 | Linux 자산 GLIBC 심볼 최대 버전 검사 | 2.17 이하 | 통합 | Linux | 통과 | - |
| TC-REL-005 | 릴리스 노트 | PRD-REL-005, SRS-BLD-011 | 릴리스 초안 | 노트 확인 | 변경 사항, 설치 명령, 체크섬 검증 방법 포함 | 수동 | 전체 | 미실행 | - |
| TC-REL-006 | 출처 증명 | PRD-REL-006, SRS-BLD-010 | P4 | `gh attestation verify` | 검증 통과 | 통합 | 전체 | 미실행 | - |

## 13. 안전과 개인정보

| ID | 목적 | 관련 ID | 사전 조건 | 절차 및 입력 | 기대 결과 | 유형 | 플랫폼 | 결과 | 이슈 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TC-SAFE-001 | 자동 실행 부재 | PRD-SAFE-001, PRD-SAFE-006, SRS-SAFE-004 | 소스 트리 | `std::process::Command` 사용 위치 grep | src 직접 프로세스 실행 0곳, 브라우저 실행은 open crate 호출 1곳 | 단위 | 전체 | 통과 | - |
| TC-SAFE-002 | 규칙 판정 | PRD-SAFE-002, SRS-SAFE-001, SRS-SAFE-002 | 범주별 양성 50건, 음성 50건 문자열 | 판정 | 양성 전부 감지, 음성 오탐 0건 (목표, 결과 기록) | 단위 | 전체 | 통과 | - |
| TC-SAFE-003 | 높음 재확인 | PRD-SAFE-003, SRS-TUI-006 | 높음 판정 명령어 | Enter, 다른 키, Enter 2회 | 1회: 확인 상태, 다른 키: 해제, 2회: 확정 | 단위 | 전체 | 통과 | - |
| TC-SAFE-004 | 한계 고지 | PRD-SAFE-004, SRS-SAFE-006 | 없음 | `--help`, README 확인 | 한계 문구 존재 | 수동 | 전체 | 통과 | - |
| TC-SAFE-005 | 모델 주의 사항 표시 | PRD-SAFE-005, SRS-SAFE-007 | notes 포함 응답, 규칙 미감지 | 렌더링 | notes 표시, 경고 영역 미표시 | 스냅샷 | 전체 | 통과 | - |
| TC-SAFE-006 | 문자열 판정 한정 | PRD-SAFE-007, SRS-SAFE-005, SRS-TEST-003 | 테스트 코드 | 안전 테스트 코드 검사 | 프로세스 실행과 파일 변경 코드 부재 | 단위 | 전체 | 통과 | - |
| TC-SAFE-007 | 판정 시점 | SRS-SAFE-003, PRD-SAFE-002 | 편집 중 | 응답 반영, 편집 후 300ms, 확정 직전 | 각 시점 재판정 | 단위 | 전체 | 통과 | - |
| TC-PRIV-001 | 전송 필드 한정 | PRD-PRIV-001, PRD-PRIV-002, SRS-SEC-004, SRS-SEC-007 | 비밀 환경변수, 작업 디렉터리 파일 존재 | 요청 페이로드 캡처 | 허용 필드만, 환경변수 값과 파일 내용 부재 | 통합 | 전체 | 통과 | - |
| TC-PRIV-002 | 전송 고지 | PRD-PRIV-003, PRD-PRIV-004 | 없음 | 문서, 로그인 고지, `--help` 확인 | 전송 범위와 수정 명령어 전송 고지 존재 | 수동 | 전체 | 통과 | - |
| TC-PRIV-003 | 로그 내용 | PRD-PRIV-005, SRS-AUTH-012 | `HOWTUI_LOG=trace` | 로그인, 추론 | 로그에 토큰, 질문, 명령어 본문 부재 | 통합 | 전체 | 통과 | - |
| TC-PRIV-004 | 파일 권한 | PRD-PRIV-006, SRS-AUTH-007 | 파일 대체 저장소 | 로그인 | credentials.json 0600 | 통합 | Unix | 통과 | - |
| TC-PRIV-005 | 제출 전 전송 없음 | PRD-PRIV-004, SRS-SEC-007 | Mock HTTP 서버 요청 기록, `HOWTUI_BUFFER` 설정 위젯 모드 | 실행 후 10초 대기, 질문 제출 | 제출 전 추론 요청 0건(모델 목록 조회만), 제출 후 요청에 버퍼 내용 포함 | 통합 | 전체 | 미실행 | - |

## 14. 플랫폼과 오류 처리

| ID | 목적 | 관련 ID | 사전 조건 | 절차 및 입력 | 기대 결과 | 유형 | 플랫폼 | 결과 | 이슈 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TC-PLAT-001 | 1차 조합 동작 | PRD-PLAT-001, SRS-ARC-004 | Linux+Bash, macOS+Zsh, Windows+PowerShell | 생성, 편집, 확정, 출력 | 3조합 모두 성공 | 수동 | 전체 | 미실행 | - |
| TC-PLAT-002 | ARMv7 실행 | PRD-PLAT-002, PRD-PLAT-003, SRS-BLD-001 | Raspberry Pi 2, 3 (32비트 OS) | 설치, TUI 실행, 원격 LLM 요청 | 동작, OS 버전과 glibc 버전 기록 | 실기기 | RPi | 미실행 | - |
| TC-PLAT-003 | ARM64 Raspberry Pi | PRD-PLAT-003 | Raspberry Pi 4, 5 (64비트 OS), 64비트 커널 + 32비트 사용자 공간 | 설치, 실행 | 사용자 공간 기준 자산 선택, 동작 | 실기기 | RPi | 미실행 | - |
| TC-PLAT-004 | 지원 상태 표기 | PRD-PLAT-004, SRS-TEST-004, SRS-BLD-012, SRS-CMP-006 | 릴리스 문서 | 표기 확인 | 실기기, 에뮬레이션, 빌드만, 미검증 구분 | 수동 | 전체 | 미실행 | - |
| TC-PLAT-005 | Windows ARM64 실행 | PRD-PLAT-002 | Windows ARM64 장치 | 설치, 실행 | 동작 | 실기기 | WinARM | 미실행 | - |
| TC-ERR-001 | 오류 메시지 구성 | PRD-ERR-001, SRS-ERR-002 | 오류 분류 전체 | 메시지 생성 | 원인 1줄 + 조치 1줄 | 단위 | 전체 | 통과 | - |
| TC-ERR-002 | TUI 오류 유지 | PRD-ERR-002, SRS-ERR-002 | Mock 오류 후 정상 | 질문 제출 2회 | 1회차 오류 표시 후 상태 유지, 2회차 성공 | 단위 | 전체 | 통과 | - |
| TC-ERR-003 | panic 복구 | PRD-ERR-005, SRS-TERM-006 | panic 주입 기능 | 실행 | 터미널 복구, stdout 0바이트, 종료 코드 101 | PTY | Unix | 통과 | - |
| TC-ERR-004 | 시그널 복구 | PRD-ERR-004, SRS-TERM-006 | PTY | `kill -TERM`, `kill -HUP` | 복구, 무출력, 143과 129 | PTY | Unix | 통과 | - |
| TC-ERR-005 | 사용량 한도 안내 | PRD-ERR-003, SRS-ERR-003 | Mock 429 `usage_limit_exceeded` | 질문 제출 | 리셋 시각 미표시, 사용량 관리 URL 표시 | 단위 | 전체 | 통과 | - |

## 15. 다국어 메시지

| ID | 목적 | 관련 ID | 사전 조건 | 절차 및 입력 | 기대 결과 | 유형 | 플랫폼 | 결과 | 이슈 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TC-I18N-001 | 메시지 완전성 | PRD-I18N-001, PRD-I18N-003, SRS-I18N-001 | 없음 | 모든 `Msg` 값에 대해 언어별 `tr` 호출 | 빈 문자열 없음, 컴파일 성공 | 단위 | 전체 | 통과 | - |
| TC-I18N-002 | 언어 결정 순서 | PRD-I18N-002, SRS-I18N-002 | 환경변수와 설정 조합 | (a) `HOWTUI_LANG=ko`, 설정 en (b) 설정 ko (c) `LANG=ko_KR.UTF-8` (d) `LANG=C` | (a) ko (b) ko (c) ko (d) en | 단위 | 전체 | 통과 | - |
| TC-I18N-003 | 자리 채움 일치 | PRD-I18N-003, SRS-I18N-003 | 없음 | 자리 채움 메시지의 언어별 자리 이름 비교 | 집합 일치 | 단위 | 전체 | 통과 | - |
| TC-I18N-004 | 화면 언어 전환 | PRD-I18N-001, PRD-I18N-002 | `HOWTUI_LANG=en`, `ko` | TestBackend 렌더링 | 각 언어 스냅샷 일치 | 스냅샷 | 전체 | 통과 | - |
| TC-I18N-005 | 도움말 언어 | PRD-I18N-004, SRS-I18N-004 | `HOWTUI_LANG=ko` | `howtui --help` | 한국어 설명 출력 | 통합 | 전체 | 통과 | - |
| TC-I18N-006 | 기계 판독 출력 고정 | PRD-I18N-005, SRS-I18N-005 | `HOWTUI_LANG=ko`와 `en` | `doctor --json`, `--no-tui` 실행 | JSON 키와 상태 값, stdout 바이트, 종료 코드 동일 | 통합 | 전체 | 통과 | - |
| TC-I18N-007 | Windows 언어 판정 | PRD-I18N-002, SRS-I18N-002 | 한국어 Windows | `auto` 설정으로 실행 | 한국어 표시 | 수동 | Windows | 미실행 | - |

## 16. 구조와 비기능

| ID | 목적 | 관련 ID | 사전 조건 | 절차 및 입력 | 기대 결과 | 유형 | 플랫폼 | 결과 | 이슈 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TC-NFR-001 | 모듈 의존 규칙 | SRS-MOD-001, SRS-MOD-002, SRS-ARC-002, PRD-FS-002, SRS-MOD-008 | 소스 트리 | CI grep: `src/app/`의 금지 crate 사용 | 0건 | 단위 | 전체 | 통과 | - |
| TC-NFR-002 | 첫 화면 시간 | SRS-PERF-001, SRS-ASYNC-006, PRD-UI-001 | 릴리스 빌드, 네트워크 차단 | 실행부터 첫 프레임까지 측정 10회 | 중앙값 기록, 100ms 목표 대비 판정 | 수동 | 전체 | 통과 | - |
| TC-NFR-003 | 유휴 CPU | SRS-PERF-002, SRS-ASYNC-005, PRD-UI-007 | 입력 대기 상태 10초 | CPU 시간 측정 | 1% 미만 (기록) | 수동 | Linux | 통과 | - |
| TC-NFR-004 | 바이너리 크기 | SRS-PERF-003, PRD-REL-001, SRS-BLD-005 | 릴리스 빌드 | 타깃별 크기 기록 | 기록 완료 | 통합 | 전체 | 통과 | - |
| TC-NFR-005 | Raspberry Pi 첫 화면 | SRS-PERF-004, PRD-PLAT-003 | Pi 2 | 첫 화면 시간 측정 | 기록 | 실기기 | RPi | 미실행 | - |
| TC-NFR-006 | TLS 검증 고정 | SRS-SEC-001, PRD-AUTH-001, SRS-MOD-005 | 자체 서명 인증서 Mock 서버 | 요청 | 연결 거부 | 통합 | 전체 | 미실행 | - |
| TC-NFR-007 | 프록시 | SRS-SEC-002, PRD-LLM-001 | 로컬 프록시, `HTTPS_PROXY` | 모델 목록 요청 | 프록시 경유 | 통합 | Linux | 미실행 | - |
| TC-NFR-008 | 의존성 취약점 | SRS-SEC-005, PRD-REL-001 | CI | `cargo deny check` 또는 `cargo audit` | 오류 0건 | 통합 | Linux | 미실행 | - |
| TC-NFR-009 | 외부 인증 테스트 격리 | SRS-TEST-001, SRS-TEST-002, PRD-LLM-012 | CI 기본 실행 | `cargo test` | 실연동 테스트 미실행(ignored), 네트워크 외부 접속 0건 | 통합 | 전체 | 통과 | - |
| TC-NFR-010 | 추적성 검사 | SRS-TEST-005, PRD-REL-001 | 문서 | `uv run scripts/trace_check.py` | 오류 0건 | 통합 | Linux | 통과 | - |
| TC-NFR-012 | 의존성 목록 일치 | SRS-MOD-004, PRD-REL-001 | `Cargo.lock` | `cargo tree --depth 1`과 SRS 3.3절 표 비교 | 표 외 직접 의존성 없음 | 수동 | 전체 | 통과 | - |
| TC-NFR-011 | 비동기 런타임 구성 | SRS-ASYNC-001, SRS-ASYNC-002, PRD-LLM-004 | 없음 | 런타임 생성 코드 검사와 TC-LLM-004 | 작업 스레드 2개, 단일 이벤트 루프 | 단위 | 전체 | 통과 | - |

## 17. 실행 결과 기록 (Phase 1, 2026-10-22)

- 실행 환경: 로컬 Ubuntu 20.04 x86_64 (rustc 1.99.1), GitHub Actions (ubuntu-24.04, ubuntu-24.04-arm, macos-15, windows-2025, windows-11-arm)
- 결과 열에 기록하지 않은 TC는 미실행 (Phase 2 이후 범위, 실계정, 실기기, 수동 Windows 확인 대상)

| TC | 결과 | 실행 수단 | 근거 |
| --- | --- | --- | --- |
| `TC-AUTH-001` | 통과 | 단위 `cargo test` | auth::oauth::tests (최초 등록, 재인가 URL) |
| `TC-AUTH-002` | 통과 | 단위 `cargo test` | rfc7636_vector |
| `TC-AUTH-003` | 통과 | 단위 `cargo test` | listener_ignores_other_paths_and_returns_callback, state_mismatch_and_denied_and_cancel, reauthorization_rejects_different_client_id |
| `TC-AUTH-004` | 통과 | 단위 `cargo test` | first_login_registers_verifies_and_stores, logout 테스트 2건 |
| `TC-AUTH-005` | 통과 | 단위 `cargo test` | auth::idtoken::tests |
| `TC-AUTH-006` | 통과 | 단위 `cargo test` | pasted_redirect_completes_login_and_bad_paste_is_reported (prompt=consent 기록) |
| `TC-AUTH-007` | 통과 | 수동 실행 | --no-browser --port 18765: URL과 SSH 포워딩 안내 출력, 127.0.0.1:18765 리스닝, SIGINT 취소 |
| `TC-AUTH-011` | 통과 | 수동 `--ignored keyring_roundtrip` | Linux Secret Service 왕복 (1800바이트 토큰), 파일 대체는 단위 테스트 |
| `TC-AUTH-012` | 통과 | 단위 `cargo test` | store::tests::refresh_timing |
| `TC-AUTH-013` | 통과 | 단위 `cargo test` | refresh_is_serialized_across_managers (갱신 요청 1회) |
| `TC-AUTH-014` | 통과 | 단위 `cargo test` | logout_revokes_and_keeps_registration, logout_with_server_error_reports_unconfirmed_and_forget_clears_registration |
| `TC-AUTH-015` | 통과 | 통합 `tests/cli.rs` | auth_status_masks_email_and_hides_tokens |
| `TC-AUTH-017` | 통과 | 렌더링 TestBackend | main_layout_shows_all_areas (Using ChatGPT plan), login_success_shows_plan_notice_once |
| `TC-AUTH-018` | 통과 | 수동 실행 | howtui login 시작 시 수신 정보, 저장 위치, 전송 범위 고지 출력 |
| `TC-AUTH-019` | 통과 | CI `scripts/ci/check_rules.sh` | 비공식 경로 문자열 0건 |
| `TC-AUTH-020` | 통과 | 단위 `cargo test` | transient_refresh_failure_keeps_credentials |
| `TC-AUTH-021` | 통과 | 단위 `cargo test` | terminal_refresh_error_deletes_tokens, token_errors_are_classified |
| `TC-AUTH-023` | 통과 | 단위 `cargo test` | pasted_url_validation, pasted_url_without_scheme_or_with_line_breaks |
| `TC-AUTH-024` | 통과 | 통합 `tests/cli.rs` | plan_not_granted_stops_without_request, client_not_enabled_is_reported |
| `TC-CFG-001` | 통과 | 단위 `cargo test` | platform::paths::tests |
| `TC-CFG-002` | 통과 | 단위 `cargo test` | missing_file_uses_defaults |
| `TC-CFG-003` | 통과 | 단위 `cargo test` | rejects_out_of_range_height, rejects_wrong_types_and_values |
| `TC-CFG-004` | 통과 | 통합 `tests/cli.rs` | config_errors_exit_8_with_location |
| `TC-CFG-005` | 통과 | 통합 `tests/cli.rs` | unknown_config_keys_are_not_fatal_and_doctor_warns |
| `TC-CFG-006` | 통과 | 단위 `cargo test` | schema_has_no_secret_fields |
| `TC-CFG-007` | 통과 | 단위 `cargo test` | platform::fs::tests::private_permissions |
| `TC-CLI-001` | 통과 | PTY `tests/pty/run.sh` | multiline (질문 인자 자동 제출) |
| `TC-CLI-002` | 통과 | 통합 `tests/cli.rs` | version_prints_semver |
| `TC-CLI-003` | 통과 | 통합 `tests/cli.rs` | help_lists_commands_and_exit_codes |
| `TC-CLI-004` | 통과 | 통합 `tests/cli.rs` | no_tui_prints_only_command_on_stdout |
| `TC-CLI-005` | 통과 | PTY `tests/pty/run.sh` | height |
| `TC-CLI-006` | 통과 | 단위 `cargo test` | shell::tests::resolution_order, chatgpt_e2e --shell zsh |
| `TC-CLI-009` | 통과 | 통합 `tests/cli.rs` | no_tui_errors_map_to_exit_codes |
| `TC-CLI-010` | 통과 | 통합 `tests/cli.rs` | usage_error_exits_2_without_stdout |
| `TC-DOC-001` | 통과 | 수동 PTY | doctor 11개 항목 출력 (2026-10-22 실행) |
| `TC-DOC-002` | 통과 | 수동 PTY, 통합 `tests/cli.rs` | PTY render 정상, 비대화형 tty 실패와 render 생략 |
| `TC-DOC-003` | 통과 | 통합 `tests/cli.rs` | doctor_lists_models_without_inference |
| `TC-DOC-004` | 통과 | 통합 `tests/cli.rs` | doctor_json_is_stable_and_language_independent |
| `TC-DOC-005` | 통과 | 단위 `cargo test` | doctor::tests::masks_tokens_and_home, doctor JSON 홈 경로 미포함 |
| `TC-DOC-006` | 통과 | 통합 `tests/cli.rs` | doctor_json_is_stable_and_language_independent (로그아웃 상태 1) |
| `TC-EDIT-001` | 통과 | 단위 `cargo test` | app::editor::tests::basic_editing_and_cursor, word_motion_and_kill |
| `TC-EDIT-002` | 통과 | 단위 `cargo test` | multiline_navigation_and_join |
| `TC-EDIT-003` | 통과 | 단위 `cargo test` | undo_groups_typing_and_steps, undo_limit_is_enforced |
| `TC-EDIT-004` | 통과 | PTY `tests/pty/run.sh` | paste (tmux bracketed paste), paste_keeps_newlines_and_normalizes_crlf |
| `TC-EDIT-005` | 통과 | 단위 `cargo test` | unicode_graphemes_and_width, vertical_move_respects_wide_chars |
| `TC-EDIT-006` | 통과 | 단위 `cargo test` | follow_up_revises_with_user_edit_and_undo_restores_edit |
| `TC-EDIT-007` | 통과 | 단위 `cargo test` | esc_cancels_request_and_late_response_is_dropped |
| `TC-EDIT-008` | 통과 | 단위 `cargo test` | explain_updates_only_explanation |
| `TC-ERR-001` | 통과 | 단위 `cargo test` | i18n::tests, update::error_message |
| `TC-ERR-002` | 통과 | 단위 `cargo test` | error_keeps_previous_state_and_reports |
| `TC-ERR-003` | 통과 | PTY `tests/pty/run.sh` | panic (101) |
| `TC-ERR-004` | 통과 | PTY `tests/pty/run.sh` | signal_TERM (143), signal_HUP (129) |
| `TC-ERR-005` | 통과 | 단위 `cargo test` | ErrUsageLimit 문구 (사용량 관리 URL, 리셋 시각 없음) |
| `TC-I18N-001` | 통과 | 단위 `cargo test` | every_message_is_translated |
| `TC-I18N-002` | 통과 | 단위 `cargo test` | i18n::detect::tests |
| `TC-I18N-003` | 통과 | 단위 `cargo test` | placeholders_match_across_languages |
| `TC-I18N-004` | 통과 | PTY `tests/pty/run.sh`, 렌더링 TestBackend | korean, korean_labels_and_too_small_message |
| `TC-I18N-005` | 통과 | 통합 `tests/cli.rs` | help_lists_commands_and_exit_codes (한국어) |
| `TC-I18N-006` | 통과 | 통합 `tests/cli.rs` | doctor_json_is_stable_and_language_independent |
| `TC-INS-001` | 통과 | `tests/install/run_unix.sh` | Linux 로컬, CI Linux와 macOS |
| `TC-INS-002` | 통과 | `tests/install/run_unix.sh` | 버전 지정과 --bin-dir 재설치 |
| `TC-INS-003` | 통과 | `tests/install/run_unix.sh` | 변조 감지, 기존 바이너리 유지 |
| `TC-INS-004` | 통과 | `tests/install/detect.sh` | 판별 표 12건 |
| `TC-INS-005` | 통과 | CI `tests/install/run_windows.ps1` | windows-2025의 powershell(5.1), pwsh(7): latest 조회, PATH ExpandString 등록, 변조 감지 |
| `TC-INS-010` | 통과 | 수동 검토 | install.sh, install.ps1 다운로드 대상은 릴리스 자산과 SHA256SUMS뿐, 다운로드 내용 실행 없음 |
| `TC-KEY-001` | 통과 | 단위 `cargo test` | ui::keymap::tests::table_bindings |
| `TC-KEY-002` | 통과 | PTY `tests/pty/run.sh` | multiline (Ctrl+J, Alt+Enter) |
| `TC-KEY-004` | 통과 | 단위 `cargo test` | config::tests::schema_has_no_key_binding_fields |
| `TC-LLM-001` | 통과 | 단위 `cargo test` | llm::prompt::tests, chatgpt_e2e::generates_through_chatgpt_api |
| `TC-LLM-002` | 통과 | 단위 `cargo test` | history_is_limited, revise_includes_edited_command |
| `TC-LLM-003` | 통과 | 단위 `cargo test` | explain_updates_only_explanation |
| `TC-LLM-004` | 통과 | 단위 `cargo test` | esc_cancels_request_and_late_response_is_dropped, chatgpt cancel_returns_promptly (1초 이내) |
| `TC-LLM-005` | 통과 | 단위 `cargo test` | llm::parse::tests::plain_json |
| `TC-LLM-006` | 통과 | 단위 `cargo test` | instructions_name_environment_and_language_rule |
| `TC-LLM-007` | 통과 | 단위 `cargo test` | instructions_name_environment_and_language_rule |
| `TC-LLM-008` | 통과 | 단위 `cargo test` | model_preference_and_cache |
| `TC-LLM-009` | 통과 | 단위 `cargo test` | llm::parse::tests (4단계) |
| `TC-LLM-010` | 통과 | 통합 `tests/cli.rs` | no_tui_retries_invalid_response_once |
| `TC-LLM-012` | 통과 | 통합 `tests/cli.rs` | Mock 시나리오 전체, 릴리스 빌드 테스트 훅 문자열 0건 (`strings`) |
| `TC-LLM-013` | 통과 | 단위 `cargo test` | llm::sse::tests |
| `TC-LLM-014` | 통과 | 단위 `cargo test` | generates_with_required_fields_and_streaming, chatgpt_e2e |
| `TC-LLM-015` | 통과 | 단위 `cargo test` | error_mapping_table (16종) |
| `TC-LLM-016` | 통과 | 단위 `cargo test` | structured_output_fallback_is_remembered |
| `TC-NFR-001` | 통과 | CI `scripts/ci/check_rules.sh` | src/app 금지 crate 0건 |
| `TC-NFR-002` | 통과 | 수동 측정 | 릴리스 빌드 10회 296-591 µs (중앙값 약 343 µs), 목표 100 ms |
| `TC-NFR-003` | 통과 | 수동 측정 | 유휴 11초 CPU 시간 0초, RSS 3.9 MB |
| `TC-NFR-004` | 통과 | 수동 측정 | x86_64 릴리스 7,970,088 바이트 |
| `TC-NFR-009` | 통과 | 단위 `cargo test` | 로컬 Mock 서버만 사용, 실연동 테스트 없음 |
| `TC-NFR-010` | 통과 | CI lint | trace_check.py 오류 0건 |
| `TC-NFR-011` | 통과 | 코드 검토 | worker_threads(2), runtime::run_tui 단일 루프 |
| `TC-NFR-012` | 통과 | `cargo tree --depth 1` | SRS 3.3절 표와 직접 의존성 일치 |
| `TC-OUT-001` | 통과 | PTY `tests/pty/run.sh` | pipe |
| `TC-OUT-002` | 통과 | PTY `tests/pty/run.sh` | capture |
| `TC-OUT-003` | 통과 | PTY `tests/pty/run.sh` | cancel, danger 등 파일 리디렉션 케이스 |
| `TC-OUT-004` | 통과 | PTY `tests/pty/run.sh` | cancel_C-c, cancel_Escape |
| `TC-OUT-005` | 통과 | 통합 `tests/cli.rs` | no_tui_errors_map_to_exit_codes |
| `TC-OUT-006` | 통과 | 통합 `tests/cli.rs` | tui_without_controlling_terminal_exits_3 |
| `TC-OUT-007` | 통과 | PTY `tests/pty/run.sh` | stdin_pipe |
| `TC-OUT-008` | 통과 | PTY `tests/pty/run.sh` | pipe (stderr 0바이트) |
| `TC-OUT-011` | 통과 | 통합 `tests/cli.rs` | logging_goes_to_file_without_content |
| `TC-OUT-012` | 통과 | CI `tests/conpty.rs` | stdout 리디렉션, stdout과 stderr 리디렉션 2개 경로, err 0바이트 |
| `TC-PRIV-001` | 통과 | 통합 `tests/cli.rs` | generates_through_chatgpt_api (환경변수 값, 작업 디렉터리 미포함) |
| `TC-PRIV-003` | 통과 | 통합 `tests/cli.rs` | logging_goes_to_file_without_content |
| `TC-PRIV-004` | 통과 | 단위 `cargo test` | store::tests::file_store_is_private |
| `TC-REL-002` | 통과 | CI release 시험 실행 | run 39725469350: 7개 압축 파일과 설치 스크립트 2개 SHA256SUMS |
| `TC-REL-003` | 통과 | CI release 시험 실행 | 7개 타깃 빌드, Linux GLIBC_2.17, macOS minos 11.0과 codesign, Windows 시스템 DLL만 의존 |
| `TC-REL-004` | 통과 | CI armv7-glibc | 3개 타깃 GLIBC_2.17, 공유 라이브러리 의존 없음 |
| `TC-SAFE-001` | 통과 | CI `scripts/ci/check_rules.sh` | src 직접 프로세스 실행 0곳 |
| `TC-SAFE-002` | 통과 | 단위 `cargo test` | 양성 52건(Bash 36, PowerShell 16) 전부 감지, 음성 27건 오탐 0건 |
| `TC-SAFE-003` | 통과 | 단위 `cargo test`, PTY `tests/pty/run.sh` | high_risk_command_needs_second_enter, danger |
| `TC-SAFE-004` | 통과 | 통합 `tests/cli.rs` | help_lists_commands_and_exit_codes (한계 문구) |
| `TC-SAFE-005` | 통과 | 렌더링 TestBackend | main_layout_shows_all_areas (Notes 표시, 경고 영역 없음) |
| `TC-SAFE-006` | 통과 | CI `scripts/ci/check_rules.sh` | 안전 테스트에 프로세스 실행과 파일 변경 없음 |
| `TC-SAFE-007` | 통과 | 단위 `cargo test` | findings_follow_edits |
| `TC-SH-009` | 통과 | `tests/install/run_unix.sh` | 터미널 없음, 질문에 n, `--no-shell-integration` 3개 경로 rc 불변 (CR-004 이후 재실행) |
| `TC-UI-001` | 통과 | PTY `tests/pty/run.sh` | inline_keep (원시 출력 ?1049h 0건) |
| `TC-UI-003` | 통과 | 렌더링 TestBackend | ui::inline::tests::main_layout_shows_all_areas |
| `TC-UI-004` | 통과 | 렌더링 TestBackend | layout_priority_on_short_screen |
| `TC-UI-005` | 통과 | 렌더링 TestBackend | requesting_shows_progress_and_cancel_hint |
| `TC-UI-006` | 통과 | 단위 `cargo test` | app::update::tests (상태 전이 표 행별 27건) |
| `TC-UI-007` | 통과 | 단위 `cargo test` | success_applies_command_and_keeps_question_focus, arrows_move_between_question_and_command (CR-001 이후 재실행) |
| `TC-UI-008` | 통과 | PTY `tests/pty/run.sh` | resize |
| `TC-UI-009` | 통과 | PTY `tests/pty/run.sh` | tiny (25x5) |
| `TC-UI-010` | 통과 | PTY `tests/pty/run.sh` | cancel, panic, signal 케이스 stty 복구 판정 |
| `TC-UI-011` | 통과 | PTY `tests/pty/run.sh` | midline (prefixEND) |
| `TC-UI-012` | 통과 | PTY `tests/pty/run.sh` | login, login_cancelled 화면 (CR-002 이후 재실행) |
| `TC-UI-013` | 통과 | 수동 | README와 --help에 팝업 동등 표현 없음 확인 |
| `TC-UI-015` | 통과 | PTY `tests/pty/no_dsr.py` | 종료 코드 3, 2.01초 |
| `TC-UI-016` | 통과 | CI `tests/conpty.rs` | windows-2025, windows-11-arm (run 39726403517) |

## 18. 실행 결과 기록 (요구사항 변경 CR-001-003, Phase 2)

- 실행 환경: 로컬 Ubuntu 20.04 x86_64 (rustc 1.99.1, tmux 3.7c, Bash 5.0.17), 컨테이너 Zsh 5.9 (Debian 12), PowerShell 7.4.2 (Ubuntu 22.04), Bash 3.2-5.2 공식 이미지. 컨테이너 이미지는 시험 후 삭제
- 변경 전 통과 기록이 있는 TC는 17절 행에 재실행 근거 추가

| TC | 결과 | 실행 수단 | 근거 |
| --- | --- | --- | --- |
| `TC-AUTH-008` | 통과 | 실계정 수동 (사용자) | 붙여넣기 로그인 성공 보고, `state.json` 구조화 출력 `true` (2026-10-22) |
| `TC-AUTH-025` | 통과 | 단위 `cargo test` | login_paste_submits_redirect_and_invalid_paste_keeps_waiting |
| `TC-UI-017` | 통과 | 단위 `cargo test`, PTY `tests/pty/run.sh` | empty_question_enter_accepts_command, empty_question_enter_on_high_risk_needs_second_enter, pipe, danger |
| `TC-UI-018` | 통과 | 단위 `cargo test` | follow_up_revises_with_user_edit_and_undo_restores_edit |
| `TC-CLI-007` | 통과 | 통합 `tests/cli.rs` | init_prints_scripts_and_validates_key (bash, zsh, powershell, pwsh 별칭, fish 종료 코드 2) |
| `TC-EDIT-009` | 통과 | 단위 `cargo test`, PTY `tests/pty/shell.sh` | widget_input_requires_output_file, widget_open 화면 (명령어 초기값, 질문 영역 포커스 안내) |
| `TC-SH-001` | 통과 | PTY `tests/pty/shell.sh bash` | Bash 5.0 로컬, Bash 5.2 (Ubuntu 24.04 컨테이너): widget_replace, widget_bottom. 5.2의 `$(<file)` 결함 발견 후 `read` 방식으로 수정, 재실행 통과 |
| `TC-SH-002` | 통과 | PTY `tests/pty/shell.sh zsh` | Zsh 5.9 (Debian 12 컨테이너): widget_replace, widget_bottom |
| `TC-SH-004` | 통과 | PTY `tests/pty/shell.sh` | key_option (alt+h, `--no-wrap`), 3개 쉘 |
| `TC-SH-005` | 통과 | PTY `tests/pty/shell.sh` | widget_cancel, 3개 쉘 |
| `TC-SH-006` | 통과 | PTY `tests/pty/shell.sh` | widget_missing (바이너리 없음), widget_error (Mock 사용량 한도 오류 후 Esc), 3개 쉘 |
| `TC-SH-007` | 통과 | PTY `tests/pty/shell.sh`, 단위 `cargo test` | widget_fallback 3개 쉘, file_destinations_write_and_widget_failure_exits_1 |
| `TC-SH-008` | 통과 | 단위 `cargo test`, 통합 `tests/cli.rs` | default_scripts_bind_ctrl_g_and_define_wrapper, init_prints_scripts_and_validates_key |
| `TC-SH-010` | 통과 | PTY `tests/pty/shell.sh` | doctor (`shell integration: <쉘>`), 3개 쉘 |
| `TC-SH-012` | 통과 | PTY `tests/pty/shell.sh`, 통합 `tests/cli.rs` | widget_replace 3개 쉘, widget_without_output_file_exits_2 |
| `TC-SH-014` | 통과 | PTY `tests/pty/shell.sh` | widget_multiline 3개 쉘 (PowerShell 연속 프롬프트 `>>`) |
| `TC-SH-016` | 통과 | PTY `tests/pty/shell.sh` | next_input: Bash 5.0, Bash 5.2, Zsh 5.9, PowerShell 7.4 |
| `TC-SH-017` | 통과 | PTY `tests/pty/shell.sh` | wrapper_capture, wrapper_cancel, 3개 쉘 |
| `TC-SH-018` | 통과 | PTY `tests/pty/shell.sh powershell` | mcr.microsoft.com/powershell 7.4.2 (Ubuntu 22.04 컨테이너), 12건 |
| `TC-SH-015` | 통과 | CI macos-15 `tests/pty/shell.sh bash` (`/bin/bash` 3.2.57) | legacy_warning (경고 1줄, 프롬프트 정상), legacy_history (다음 입력란 비어 있음, ↑로 명령어). PSReadLine 미로드 PowerShell은 미실행 |
| `TC-SH-019` | 통과 | 컨테이너 `bash:3.2`, `bash:4.0`, `bash:4.4`, `bash:5.1`, `bash:5.2` | 5개 버전 문법 검사와 적재, 3.2만 바인딩 생략 경고 |
| `TC-SH-020` | 통과 | 통합 `tests/cli.rs`, 단위 `cargo test` | init_install_edits_only_the_marked_block, install_appends_once_and_updates_in_place, uninstall_restores_original_text, rewrite_keeps_symlink_and_mode |
| `TC-INS-011` | 통과 | `tests/install/run_unix.sh`, 로컬 tmux | answer no, answer yes 2회, uninstall, `--shell-integration`, `--no-shell-integration`. `cat install.sh \| sh` 실행에서 질문 표시와 Enter 동의 확인, 새 쉘에서 함수와 바인딩 적재 |
| `TC-PRIV-002` | 통과 | 문서 검토, PTY `tests/pty/run.sh` | README 7절, `howtui login` 고지(PrivacyNotice), TUI 로그인 화면 고지(login 화면 `to OpenAI`), `--help` 끝 문단. TUI 고지 누락은 CR-002 이후 발견해 추가 |

CI 실행 기록 (run 39766949660, 커밋 3108a88, 13개 작업 전체 통과):

| 작업 | 쉘 | 결과 |
| --- | --- | --- |
| pty (ubuntu-24.04) | Bash 5.2.21, Zsh 5.9, PowerShell 7.6.7 (PSReadLine 2.4.5) | 각 12건 통과 |
| pty (macos-15) | Bash 3.2.57 (2회), Zsh 5.9, PowerShell 7.6.5 (PSReadLine 2.4.5) | Bash 3.2 각 2건, Zsh와 PowerShell 각 12건 통과 |

CI 도입 중 발견과 수정: (1) Bash 5.2의 `bind -x` 안 `$(<file)` 치환 결함 (2) macOS Bash 3.2 전환 안내 출력 (3) Ubuntu `/etc/zsh/zshrc` compinit 확인 질문 (4) 고정 대기 경합 (5) Windows 설치 시험 HTTP 서버 기동 경합.

## 19. 실행 기록 (CR-005, CR-006, Windows 사용자 확인, 2026-10-23)

| TC | 결과 | 실행 수단 | 근거 |
| --- | --- | --- | --- |
| `TC-UI-014` | 통과 | 사용자 수동 확인 (Windows) | Windows Terminal과 PowerShell 창 화면 정상 (사용자 보고, 2026-10-23) |
| `TC-OUT-009` | 통과 | 사용자 수동 확인 (Windows) | 리디렉션과 캡처 명령어 정상 (사용자 보고). 한국어 코드 페이지 949 조건은 별도 확인 대기 (TC-SH-013) |
| `TC-SH-003` | 통과 | 사용자 수동 확인 (Windows) | Ctrl+G 입력란 교체와 직접 실행 결과 다음 입력 전달 정상 (사용자 보고) |
| `TC-UI-019` | 통과 | 단위 `cargo test`, PTY `tests/pty/run.sh` | settings_select_model_flow, settings_close_and_slash_command, settings_load_errors, settings_view_lists_models_and_click_targets, settings |
| `TC-UI-020` | 통과 | 단위 `cargo test`, PTY `tests/pty/run.sh` | settings_clicks_and_sign_out, settings (SGR 클릭 2회, 원시 출력 `?1000h`와 `?1000l`) |
| `TC-LLM-022` | 통과 | 단위 `cargo test` | model_choice_order_with_selection_and_default, settings_list_models_and_selection_persists |
| `TC-INS-012` | 통과 | `tests/install/run_unix.sh` | uninstall script |
| `TC-INS-013` | 통과 | CI installers (windows-2025 powershell 5.1.26100, pwsh 7.6.7) | run 39932086714: answer no keeps profile, default answer adds one profile block, uninstall script |
| `TC-REL-001` | 통과 | 태그 `v0.1.0` 릴리스 워크플로 | run 39932957686: 7개 대상 빌드, 초안 생성, 체크섬과 압축 파일 구성과 출처 증명(`gh attestation verify`) 확인 후 공개 (2026-10-23T09:42Z) |
| `TC-REL-008` | 통과 | `release-smoke.yml` | run 39934903418: ubuntu-24.04, ubuntu-24.04-arm, macos-15, windows-2025 (5.1, 7), windows-11-arm 공개 설치, `--version`, 제거 |
| `TC-SH-021` | 통과 | 단위 `cargo test`, 통합 `tests/cli.rs` | rc_path_rules, powershell_block_is_ascii_and_creates_profile_dir, init_install_edits_only_the_marked_block |
