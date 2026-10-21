# PRD: howtui 제품 요구사항 문서

- 문서 ID: PRD
- 버전: 0.1 (Phase 0 초안)
- 작성일: 2026-10-22
- 상태: 사용자 검토 대기
- 상위 문서: [MRD](MRD.md)
- 하위 문서: [SRS](SRS.md), [TC](TC.md), [TRACEABILITY](TRACEABILITY.md)

## 0. 표기 규칙

- 요구사항 ID: `PRD-<영역>-<번호>`. 영역 코드는 아래 표 기준.
- 단계: P1 (Inline TUI MVP), P2 (쉘 통합), P3 (Fullscreen), P4 (배포 안정화), P5 (API Key Provider), P6 (로컬 LLM)
- 우선순위: 필수 (해당 단계 완료 조건), 권장 (해당 단계 내 구현 목표, 미완료 시 다음 단계로 이월 기록)
- MRD 열: 근거가 되는 MRD 항목 ID
- 요구사항 변경 시 본 문서, SRS, TC, TRACEABILITY 동시 갱신

| 영역 코드 | 영역 |
| --- | --- |
| CLI | 명령줄 인터페이스 |
| UI | Inline TUI 화면 |
| KEY | 키 바인딩 |
| EDIT | 명령어 편집 |
| LLM | LLM 작업과 Provider |
| AUTH | ChatGPT 로그인 인증 |
| OUT | stdout 출력 계약 |
| SH | 선택적 쉘 통합 |
| FS | Fullscreen TUI |
| CFG | 사용자 설정 |
| DOC | howtui doctor |
| INS | 설치와 업데이트 |
| REL | 릴리스 |
| SAFE | 안전 |
| PRIV | 개인정보와 전송 데이터 |
| PLAT | 플랫폼 |
| ERR | 오류 처리 |

---

## 1. 사용자 워크플로

### 1.1 기본 흐름 (P1)

1. 사용자가 `howtui` 또는 `howtui <질문>` 실행
2. 기존 터미널 출력 아래에 Inline 영역 표시 (기본 10줄 이내)
3. 질문 입력 후 Enter: 생성 요청, 로딩 표시
4. 응답 수신: 명령어 편집 영역에 명령어, 설명 영역에 설명 표시, 포커스가 명령어 편집 영역으로 이동
5. 사용자 선택
   - (a) 직접 편집 후 Enter 확정
   - (b) Tab으로 질문 입력 영역 이동, 후속 요청 입력 후 Enter: 현재 편집 내용 기준 수정 요청
   - (c) Ctrl+E: 현재 명령어 설명 요청
   - (d) Esc 또는 Ctrl+C: 취소 종료
6. 확정 시 Inline 영역 삭제, 터미널 상태 복구, stdout에 명령어 출력 후 종료 코드 0
7. 실행은 사용자 판단 (howtui는 실행하지 않음)

### 1.2 쉘 통합 흐름 (P2)

1. 사용자가 `howtui init <쉘>` 출력을 rc 파일 또는 Profile에 직접 등록
2. 쉘 입력 중 단축키 입력
3. 기존 입력란 내용이 명령어 편집 영역 초기값으로 표시
4. 1.1의 3-5단계 동일
5. 확정 시 쉘 입력란이 최종 명령어로 교체, 커서는 끝 위치, 실행 없음
6. 취소 또는 실패 시 기존 입력란 무변경

### 1.3 비대화형 흐름 (P1)

1. `howtui --no-tui <질문>` 실행
2. TUI 없이 생성 요청 1회
3. stdout에 명령어만 출력, 설명과 경고는 stderr 평문 출력
4. 터미널 제어 시퀀스 출력 없음

---

## 2. 화면 구성 (Inline, P1)

기본 높이 10줄 기준 배치. 영역 우선순위: 상태줄 > 질문 > 명령어 > 경고 > 설명.

```text
질문 > 현재 폴더에서 100MB 넘는 파일 찾기
명령 > find . -type f -size +100M -print
설명   현재 디렉터리 이하에서 100MB 초과 일반 파일 경로 출력
       (긴 설명은 남은 줄 수만큼 줄바꿈 표시, 초과분은 "…" 표시)
경고   (위험 명령어 감지 시에만 표시)
상태   gpt-model  bash  Using ChatGPT plan   Enter 확정  Tab 질문  Ctrl+J 줄바꿈  Esc 종료
```

- 화면 문구는 예시. 최종 UI 문자열 언어는 미해결 사항 E-03([ROADMAP](ROADMAP.md) 6절)에서 결정.
- 요청 중 상태줄: 진행 표시와 "Esc 취소"
- 경고 영역: 위험 수준과 감지 사유 1줄

---

## 3. TUI 상태 전이 (P1)

| 현재 상태 | 이벤트 | 다음 상태 | 동작 |
| --- | --- | --- | --- |
| 시작 | 인증 정보 없음 | 로그인 필요 | 로그인 안내 표시 |
| 시작 | 인증 정보 있음 | 입력 대기 | 질문 입력 영역 포커스 |
| 로그인 필요 | Enter | 로그인 진행 | 브라우저 실행, 인가 URL 표시 |
| 로그인 진행 | 콜백 성공 | 입력 대기 | 최초 1회 플랜 사용 안내 |
| 로그인 진행 | Esc, 실패 | 로그인 필요 | 실패 사유 표시 |
| 입력 대기 | 질문 Enter (명령어 없음) | 요청 중 (생성) | Generate 요청 |
| 편집 중 | 질문 Enter (명령어 있음) | 요청 중 (수정) | Revise 요청, 현재 편집 내용 포함 |
| 편집 중 | Ctrl+E | 요청 중 (설명) | Explain 요청 |
| 요청 중 | 응답 성공 (생성, 수정) | 편집 중 | 명령어 반영 (Undo 1단계), 설명 갱신, 위험 검사 |
| 요청 중 | 응답 성공 (설명) | 편집 중 | 설명만 갱신 |
| 요청 중 | Esc | 직전 상태 | 요청 취소, 늦게 도착한 응답 폐기 |
| 요청 중 | 오류 | 직전 상태 | 오류 유형별 메시지 표시 |
| 편집 중 | 명령어 Enter (위험 높음 아님) | 확정 | 종료 후 출력 |
| 편집 중 | 명령어 Enter (위험 높음) | 위험 확인 | 경고 강조, 재확인 안내 |
| 위험 확인 | Enter | 확정 | 종료 후 출력 |
| 위험 확인 | 그 외 키 | 편집 중 | 확인 해제 |
| 모든 상태 | Ctrl+C | 취소 | 진행 요청 취소, 무출력 종료 (130) |
| 요청 중, 로그인 진행 외 모든 상태 | Esc | 취소 | 무출력 종료 (130) |
| 모든 상태 | 창 크기 최소 미만 | 크기 부족 (중첩 상태) | 크기 부족 안내 1줄, 키 입력은 Esc와 Ctrl+C만 처리 |
| 크기 부족 | 창 크기 회복 | 직전 상태 | 화면 재구성 |

---

## 4. 키 바인딩 (Inline P1, Fullscreen P3 공통)

키 선택 근거: SPIKE-01 실측 (Alt+Enter, Ctrl+J 줄바꿈 수신 확인). Windows Terminal의 Alt+Enter 기본 전체화면 전환 동작 때문에 Ctrl+J를 1차 줄바꿈 키로 지정. Windows 실측 전까지 Windows 키 동작은 미검증.

| 키 | 질문 입력 영역 | 명령어 편집 영역 | 요청 중 |
| --- | --- | --- | --- |
| Enter | 요청 제출 (명령어 유무로 생성과 수정 구분) | 확정 (위험 높음 시 재확인) | 무시 |
| Ctrl+J, Alt+Enter | 무시 | 줄바꿈 삽입 | 무시 |
| Shift+Enter | 무시 | 줄바꿈 삽입 (키보드 확장 프로토콜 지원 터미널 한정, 권장) | 무시 |
| Tab, Shift+Tab | 포커스 전환 | 포커스 전환 | 포커스 전환 (편집 잠금 유지) |
| Esc | 취소 종료 | 취소 종료 | 요청 취소 |
| Ctrl+C | 취소 종료 | 취소 종료 | 요청 취소 후 종료 |
| Ctrl+E | 무시 | 설명 요청 | 무시 |
| Ctrl+Z, Ctrl+Y | 질문 Undo, Redo | 명령어 Undo, Redo | 무시 |
| 방향키, Home, End | 커서 이동 | 커서 이동 (여러 줄 상하 이동 포함) | 이동만 허용 |
| Ctrl+방향키, Alt+B, Alt+F | 단어 단위 이동 | 단어 단위 이동 | 이동만 허용 |
| Backspace, Delete | 문자 삭제 | 문자 삭제 | 무시 |
| Ctrl+W, Ctrl+U, Ctrl+K | 단어 삭제, 줄 앞 삭제, 줄 뒤 삭제 | 동일 | 무시 |
| 붙여넣기 (bracketed paste) | 한 줄로 결합 입력 | 원문 그대로 삽입 (여러 줄 유지, 확정 미발생) | 무시 |

---

## 5. 기능 요구사항

### 5.1 CLI

| ID | 요구사항 | 단계 | 우선순위 | MRD |
| --- | --- | --- | --- | --- |
| PRD-CLI-001 | `howtui` 실행 시 Inline TUI 시작. 위치 인자 `<질문>` 지정 시 질문 입력 영역에 채운 뒤 즉시 생성 요청 | P1 | 필수 | MRD-SCN-001, MRD-SCP-001 |
| PRD-CLI-002 | `howtui --version`: 버전 문자열(`howtui <semver>`)을 stdout에 출력, 종료 코드 0 | P1 | 필수 | MRD-SCP-007 |
| PRD-CLI-003 | `howtui login`, `howtui logout`, `howtui auth status` 하위 명령 제공 | P1 | 필수 | MRD-SCP-004 |
| PRD-CLI-004 | `howtui doctor [--json]` 하위 명령 제공 | P1 | 필수 | MRD-SCP-006 |
| PRD-CLI-005 | `howtui --no-tui <질문>`: TUI 없는 1회 생성 (1.3절 흐름) | P1 | 필수 | MRD-SCP-005 |
| PRD-CLI-006 | `--height <N>`: Inline 높이 일시 변경 | P1 | 권장 | MRD-SCP-001 |
| PRD-CLI-007 | `--shell <bash\|zsh\|powershell>`: 대상 쉘 명시. 미지정 시 쉘 통합 스크립트가 설정한 환경변수 사용, 없으면 OS 기본 조합(Linux Bash, macOS Zsh, Windows PowerShell) 사용 | P1 | 필수 | MRD-SCN-001 |
| PRD-CLI-008 | `howtui init <bash\|zsh\|powershell>`: 쉘 통합 스크립트를 stdout에 출력 | P2 | 필수 | MRD-SCP-101 |
| PRD-CLI-009 | `--mode <inline\|fullscreen>`: 화면 모드 일시 변경 | P3 | 필수 | MRD-SCP-102 |
| PRD-CLI-010 | 종료 코드 체계 고정 (SRS 8절 표) | P1 | 필수 | MRD-SCP-005 |
| PRD-CLI-011 | `--help` 출력에 하위 명령, 옵션, 종료 코드 요약 포함 | P1 | 필수 | MRD-SCN-001 |

### 5.2 Inline TUI 화면

| ID | 요구사항 | 단계 | 우선순위 | MRD |
| --- | --- | --- | --- | --- |
| PRD-UI-001 | Inline 뷰포트 사용. 전체화면 전환과 Alternate Screen 미사용, 기존 터미널 출력 유지 | P1 | 필수 | MRD-SCP-001, MRD-DIF-006 |
| PRD-UI-002 | 설정 높이(`ui.inline_height`, 기본 10) 이내 렌더링. 터미널 높이보다 크면 터미널 높이로 축소 | P1 | 필수 | MRD-SCP-001 |
| PRD-UI-003 | 질문 입력 영역 제공 (긴 질문은 줄바꿈 표시) | P1 | 필수 | MRD-SCP-001 |
| PRD-UI-004 | 명령어 표시 및 편집 영역 제공 (여러 줄, 영역 초과 시 스크롤) | P1 | 필수 | MRD-SCP-001 |
| PRD-UI-005 | 설명 영역 제공 (남은 줄 수만큼 표시, 초과 시 생략 표시) | P1 | 필수 | MRD-SCP-001 |
| PRD-UI-006 | 상태줄: 모델, 쉘 종류, 플랜 사용 표시, 진행 상태, 주요 키 안내 | P1 | 필수 | MRD-SCP-001 |
| PRD-UI-007 | 요청 중 진행 표시와 Esc 취소 안내 | P1 | 필수 | MRD-SCP-003 |
| PRD-UI-008 | 터미널 크기 변경 시 재배치 (가로 축소 시 화면 정리 동작 포함) | P1 | 필수 | MRD-SCP-001 |
| PRD-UI-009 | 최소 크기(가로 30열, 세로 3행) 미만 시 크기 부족 안내 1줄 표시, Esc와 Ctrl+C만 처리, 회복 시 재구성 | P1 | 필수 | MRD-SCP-001 |
| PRD-UI-010 | 종료 시 Inline 영역 삭제, 커서 원위치, raw mode 해제, 커서 표시 복구. 정상 확정, 취소, 오류, panic, 종료 시그널 모두 적용 | P1 | 필수 | MRD-KPI-003 |
| PRD-UI-011 | 프롬프트 줄 중간에서 시작 시 다음 줄부터 Inline 영역 표시, 종료 시 원래 커서 위치 복귀 | P1 | 필수 | MRD-KPI-001 |
| PRD-UI-012 | 포커스 규칙: 시작 시 질문 영역, 생성 또는 수정 응답 수신 시 명령어 영역, Tab으로 전환 | P1 | 필수 | MRD-SCN-002 |
| PRD-UI-013 | 3절 상태 전이 표 준수 | P1 | 필수 | MRD-SCP-001 |
| PRD-UI-014 | GUI 팝업 오버레이와 동일한 표현을 주장하지 않음 (문서와 도움말 문구 기준) | P1 | 필수 | MRD-EXC-012 |

### 5.3 키 바인딩

| ID | 요구사항 | 단계 | 우선순위 | MRD |
| --- | --- | --- | --- | --- |
| PRD-KEY-001 | 4절 키 바인딩 표 구현 | P1 | 필수 | MRD-SCP-001, MRD-SCP-002 |
| PRD-KEY-002 | Shift+Enter 줄바꿈: 키보드 확장 프로토콜 지원 터미널에서만 활성화, 미지원 시 무시 | P1 | 권장 | MRD-SCP-002 |
| PRD-KEY-003 | 키 바인딩 사용자 재정의 기능 미제공 (1.x 범위 외). 필요 시 요구사항 변경 절차 | P1 | 필수 | MRD-SCP-001 |

### 5.4 명령어 편집

| ID | 요구사항 | 단계 | 우선순위 | MRD |
| --- | --- | --- | --- | --- |
| PRD-EDIT-001 | 문자 입력, 삭제, 커서 이동(문자, 단어, 줄 처음과 끝, 여러 줄 상하) | P1 | 필수 | MRD-DIF-001, MRD-SCP-002 |
| PRD-EDIT-002 | 여러 줄 편집: Ctrl+J, Alt+Enter 줄바꿈 | P1 | 필수 | MRD-SCP-002 |
| PRD-EDIT-003 | Undo, Redo (질문 영역과 명령어 영역 각각 독립 이력) | P1 | 필수 | MRD-SCP-002 |
| PRD-EDIT-004 | 붙여넣기: bracketed paste 수신 시 원문 삽입, 붙여넣기 내부 줄바꿈으로 확정 미발생 | P1 | 필수 | MRD-SCP-002 |
| PRD-EDIT-005 | Unicode: 한글, CJK, 이모지 등 표시 폭 2 문자와 결합 문자의 커서 위치 정확성 | P1 | 필수 | MRD-SCP-002 |
| PRD-EDIT-006 | AI 재생성 시 변경 내용 보존 정책: (a) 요청 시점의 편집 내용을 요청 컨텍스트로 전송, (b) 요청 중 명령어 편집 잠금, (c) 응답은 Undo 1단계로 되돌릴 수 있는 단일 변경으로 반영, (d) 취소 또는 오류 시 편집 내용 무변경 | P1 | 필수 | MRD-DIF-001, MRD-SCN-003 |
| PRD-EDIT-007 | 직접 편집 후 설명 영역에 "설명 갱신 필요" 표시, Ctrl+E로 재설명 | P1 | 필수 | MRD-SCN-004 |
| PRD-EDIT-008 | 쉘 통합 호출 시 기존 입력란 내용을 명령어 영역 초기값으로 사용 | P2 | 필수 | MRD-SCN-005 |

### 5.5 LLM 작업과 Provider

| ID | 요구사항 | 단계 | 우선순위 | MRD |
| --- | --- | --- | --- | --- |
| PRD-LLM-001 | Generate: 자연어 질문과 환경 컨텍스트로 명령어와 설명 생성 | P1 | 필수 | MRD-SCP-003 |
| PRD-LLM-002 | Revise: 후속 요청, 현재 명령어, 세션 이력으로 수정 명령어와 설명 생성 | P1 | 필수 | MRD-SCP-003, MRD-DIF-001 |
| PRD-LLM-003 | Explain: 현재 명령어 설명 생성 (명령어 무변경) | P1 | 필수 | MRD-SCP-003 |
| PRD-LLM-004 | Cancel: 진행 중 요청 중단, 연결 종료, 늦게 도착한 응답 폐기 | P1 | 필수 | MRD-SCP-003 |
| PRD-LLM-005 | Provider 무관 표준 응답: 명령어(선택), 설명, 모델 제시 주의 사항(선택) | P1 | 필수 | MRD-DIF-007 |
| PRD-LLM-006 | 대상 쉘(Bash, Zsh, PowerShell)과 OS에 맞는 문법 생성 지시 | P1 | 필수 | MRD-SCN-001 |
| PRD-LLM-007 | 세션 내 대화 이력 유지(질문, 명령어, 사용자 수정본), 종료 시 폐기, 디스크 미저장 | P1 | 필수 | MRD-SCN-003 |
| PRD-LLM-008 | 설명 언어는 사용자 질문 언어와 동일하게 지시 | P1 | 권장 | MRD-SCN-001 |
| PRD-LLM-009 | ChatGPT 계정 모델 목록 조회, `visibility == "list"` 모델 중 설정값 또는 서버 순서 첫 모델 사용 | P1 | 필수 | MRD-SCP-004 |
| PRD-LLM-010 | 응답 형식 오류 시 자동 재시도 1회, 재실패 시 형식 오류 표시 | P1 | 필수 | MRD-RSK-003 |
| PRD-LLM-011 | 요청 제한 시간: 연결 10초, 첫 응답 이벤트 60초, 전체 120초 (기본값, SRS에서 조정 가능) | P1 | 필수 | MRD-SCP-003 |
| PRD-LLM-012 | Mock Provider: 재현 가능한 고정 응답, 지연, 오류 주입. 테스트 빌드 전용 | P1 | 필수 | MRD-KPI-002 |
| PRD-LLM-013 | OpenAI API Key Provider | P5 | 필수 | MRD-SCP-104 |
| PRD-LLM-014 | Gemini API Key Provider | P5 | 필수 | MRD-SCP-104 |
| PRD-LLM-015 | Anthropic API Key Provider | P5 | 필수 | MRD-SCP-104 |
| PRD-LLM-016 | OpenAI 호환 엔드포인트 Provider (URL, 선택적 키) | P5 | 필수 | MRD-SCP-104 |
| PRD-LLM-017 | Provider 설정과 전환 (설정 파일, CLI 옵션) | P5 | 필수 | MRD-SCP-104 |
| PRD-LLM-018 | Ollama HTTP API 연동 (실행 중 서버 연결) | P6 | 필수 | MRD-SCP-105 |
| PRD-LLM-019 | llama.cpp llama-server HTTP API 연동 (실행 중 서버 연결) | P6 | 필수 | MRD-SCP-105 |
| PRD-LLM-020 | 로컬 모델 선택 설정, CPU-only 동작 확인, Gemma 4 E2B와 E4B 평가 결과 문서화 | P6 | 필수 | MRD-SCP-105 |

### 5.6 ChatGPT 로그인 인증

공식 문서 근거: [ChatGPT 로그인 조사](research/chatgpt-oauth.md). 비공식 경로 사용 금지.

| ID | 요구사항 | 단계 | 우선순위 | MRD |
| --- | --- | --- | --- | --- |
| PRD-AUTH-001 | `howtui login`: Authorization Code + PKCE(S256) + state + nonce, `127.0.0.1` 루프백 콜백(`/auth/callback`), 시스템 브라우저 실행과 인가 URL 표시 | P1 | 필수 | MRD-SCP-004, MRD-USR-005 |
| PRD-AUTH-002 | 최초 등록 시 `dynamic_agent_client`와 고정 `agent_name_hint`, 발급 `client_id` 저장 후 재사용 | P1 | 필수 | MRD-SCP-004 |
| PRD-AUTH-003 | 설치별 host ID(`urn:uuid:<UUIDv4>`) 최초 1회 생성, 로그아웃과 재로그인 후에도 유지 | P1 | 필수 | MRD-SCP-004 |
| PRD-AUTH-004 | ID 토큰 서명(RS256, JWKS)과 `iss`, `aud`, `exp`, `nonce` 검증, 플랜 사용 scope 부여 확인 | P1 | 필수 | MRD-SCP-004 |
| PRD-AUTH-005 | GUI 브라우저 없는 환경: `--no-browser` URL 표시, `--port` 고정 포트와 SSH 포트 포워딩 안내 | P1 | 필수 | MRD-SCN-007, MRD-RSK-004 |
| PRD-AUTH-006 | 리다이렉트 URL 수동 붙여넣기 입력 (OpenAI 측 허용 여부 실측 후 활성화) | P1 | 권장 | MRD-SCN-007 |
| PRD-AUTH-007 | 공식 원격 장치 절차 대응: 자격증명 내보내기와 가져오기, 대상 장치 host ID 유지 | P4 | 권장 | MRD-SCN-007 |
| PRD-AUTH-008 | 토큰 저장: OS 보안 저장소 우선, 사용 불가 시 소유자 전용 권한 파일(Unix 0600) 사용과 사용자 고지 | P1 | 필수 | MRD-SCP-004 |
| PRD-AUTH-009 | 액세스 토큰 자동 갱신: 만료 60초 전, `earliest_refresh_at` 이전 갱신 보류, 프로세스 간 직렬화 | P1 | 필수 | MRD-SCP-004 |
| PRD-AUTH-010 | `howtui logout`: refresh token 폐기 요청, 로컬 토큰 삭제, client 매핑과 host ID 보존, 폐기 미확인 시 고지 | P1 | 필수 | MRD-SCP-004 |
| PRD-AUTH-011 | `howtui auth status`: 로그인 여부, 마스킹 이메일, 플랜 사용 활성 여부, 액세스 토큰 만료 시각. 토큰 값 미출력 | P1 | 필수 | MRD-SCP-004 |
| PRD-AUTH-012 | 플랜 사용 scope 미부여 시 안내: 재동의(`prompt=consent`) 재시도 경로, 대체 과금 경로 안내(P5) | P1 | 필수 | MRD-RSK-002 |
| PRD-AUTH-013 | TUI 시작 시 미로그인: TUI 내 로그인 안내, Enter로 로그인 시작, 진행 표시와 Esc 취소 | P1 | 필수 | MRD-SCN-001 |
| PRD-AUTH-014 | 브랜딩: 로그인 동작 문구 "Continue with ChatGPT", 최초 로그인 후 "You're using your ChatGPT plan" 1회 안내, 상태줄 "Using ChatGPT plan"과 사용량 관리 URL(`https://chatgpt.com/settings/usage`) 안내 | P1 | 필수 | MRD-SCP-004 |
| PRD-AUTH-015 | 로그인 전 개인정보 처리 고지: 수신 정보(이메일, 이름 클레임), 저장 위치, 전송 데이터 범위 | P1 | 필수 | MRD-SCP-004 |
| PRD-AUTH-016 | 금지: Codex 인증 파일 읽기, 브라우저 쿠키 추출, `backend-api` 등 비공식 엔드포인트 사용, 비밀번호 요구 | P1 | 필수 | MRD-EXC-007 |
| PRD-AUTH-017 | 인증 오류 구분: 인증 실패, 플랜 비대상, 권한 구성 오류, 지역 또는 정책 차단, 사용량 초과, 기능 또는 모델 미지원, 일시 장애, 네트워크 오류 | P1 | 필수 | MRD-SCP-004 |
| PRD-AUTH-018 | 일시 장애와 네트워크 오류 시 자격증명 보존 (삭제 금지) | P1 | 필수 | MRD-SCP-004 |

### 5.7 stdout 출력 계약

| ID | 요구사항 | 단계 | 우선순위 | MRD |
| --- | --- | --- | --- | --- |
| PRD-OUT-001 | 확정 시 stdout에 UTF-8 명령어 바이트와 LF 1개만 출력 | P1 | 필수 | MRD-SCP-005, MRD-DIF-005 |
| PRD-OUT-002 | TUI 렌더링과 터미널 질의는 제어 터미널 핸들로만 수행, stdout 미사용 | P1 | 필수 | MRD-SCP-005 |
| PRD-OUT-003 | 취소 시 stdout 0바이트, 종료 코드 130 | P1 | 필수 | MRD-SCP-005 |
| PRD-OUT-004 | 오류 시 stdout 0바이트, stderr 평문 메시지, 오류별 종료 코드 | P1 | 필수 | MRD-SCP-005 |
| PRD-OUT-005 | 여러 줄 명령어와 Unicode 바이트 보존 (정규화, 공백 변환, 끝 줄바꿈 추가 없음, 출력 끝 LF 1개 제외) | P1 | 필수 | MRD-SCP-005 |
| PRD-OUT-006 | 파이프, 파일 리디렉션, 명령 치환 시 오염 없음 | P1 | 필수 | MRD-KPI-002 |
| PRD-OUT-007 | 제어 터미널 없음: TUI 미시작, 종료 코드 3, stderr 평문 안내, 제어 시퀀스 미출력 | P1 | 필수 | MRD-SCP-005 |
| PRD-OUT-008 | 비대화형 모드: stdout 명령어만, stderr 설명과 경고 평문, 제어 시퀀스 미출력. 제어 터미널 유무 무관 동작 | P1 | 필수 | MRD-SCP-005 |
| PRD-OUT-009 | 직접 출력 모드와 위젯 모드 구분: 위젯 모드(쉘 어댑터 호출)는 `init` 스크립트가 지정한 출력 파일로만 결과 전달, stdout 미사용 | P2 | 필수 | MRD-SCP-101 |
| PRD-OUT-010 | 로그와 진단 출력은 stdout과 stderr에 기록하지 않음 (설정 시 로그 파일 전용) | P1 | 필수 | MRD-SCP-005 |

### 5.8 선택적 쉘 통합

| ID | 요구사항 | 단계 | 우선순위 | MRD |
| --- | --- | --- | --- | --- |
| PRD-SH-001 | Bash 어댑터: readline `bind -x`, `READLINE_LINE`, `READLINE_POINT` 기반 | P2 | 필수 | MRD-SCP-101, MRD-DIF-003 |
| PRD-SH-002 | Zsh 어댑터: ZLE 위젯, `BUFFER`, `CURSOR` 기반 (emacs, viins 키맵) | P2 | 필수 | MRD-SCP-101, MRD-DIF-003 |
| PRD-SH-003 | PowerShell 어댑터: PSReadLine `GetBufferState`, `Replace` 기반. Windows PowerShell 5.1과 PowerShell 7 지원 | P2 | 필수 | MRD-SCP-101, MRD-DIF-003 |
| PRD-SH-004 | 선택적 단축키 호출: 기본 키 Ctrl+G (3개 쉘 공통), `init --key`로 변경, `init --no-bind`로 바인딩 생략 | P2 | 필수 | MRD-SCN-005 |
| PRD-SH-005 | 기존 입력란 내용 보존, 명령어 편집 초기값 활용 (PRD-EDIT-008) | P2 | 필수 | MRD-SCN-005 |
| PRD-SH-006 | 확정 시 입력란 교체, 커서 끝 위치, 자동 실행 없음 | P2 | 필수 | MRD-DIF-002 |
| PRD-SH-007 | 취소, 오류 시 입력란 무변경. 오류 원인 1줄 표시 | P2 | 필수 | MRD-KPI-005 |
| PRD-SH-008 | 통합 실패 시 대체 경로: 최종 명령어를 터미널에 표시해 수동 복사 가능 상태 제공 | P2 | 필수 | MRD-SCN-006 |
| PRD-SH-009 | 쉘 초기화 등록과 제거 안내 (문서, `init` 출력 주석) | P2 | 필수 | MRD-SCP-101 |
| PRD-SH-010 | 설치 과정의 rc 파일과 Profile 자동 변경 금지 | P1 | 필수 | MRD-EXC-004 |
| PRD-SH-011 | 쉘 통합 사용 여부를 doctor가 판별할 수 있는 표식 제공 | P2 | 필수 | MRD-SCN-008 |
| PRD-SH-012 | 쉘별 지원 상태는 테스트 결과 기준 표기 (검증 버전 명시) | P2 | 필수 | MRD-EXC-003 |

### 5.9 Fullscreen TUI

| ID | 요구사항 | 단계 | 우선순위 | MRD |
| --- | --- | --- | --- | --- |
| PRD-FS-001 | Fullscreen 뷰포트와 Alternate Screen 기반 화면 | P3 | 필수 | MRD-SCP-102 |
| PRD-FS-002 | Inline과 동일한 상태, 편집, 후속 요청, 설명, 취소 기능 (동일 코어 사용) | P3 | 필수 | MRD-SCP-102 |
| PRD-FS-003 | 긴 질문과 답변용 배치: 세션 대화 이력 스크롤 영역 | P3 | 필수 | MRD-SCN-010 |
| PRD-FS-004 | 종료 후 기존 화면 복구 (Alternate Screen 해제) | P3 | 필수 | MRD-KPI-003 |
| PRD-FS-005 | 설정 `ui.mode`와 CLI `--mode`로 선택 | P3 | 필수 | MRD-SCP-102 |

### 5.10 사용자 설정

| ID | 요구사항 | 단계 | 우선순위 | MRD |
| --- | --- | --- | --- | --- |
| PRD-CFG-001 | TOML 설정 파일, OS별 기본 위치, `HOWTUI_CONFIG` 환경변수로 경로 재정의 | P1 | 필수 | MRD-SCP-001 |
| PRD-CFG-002 | 설정 파일 부재 시 기본값 동작, 자동 생성 없음 | P1 | 필수 | MRD-SCP-001 |
| PRD-CFG-003 | P1 설정 항목: `ui.mode`(`inline`), `ui.inline_height`(10), `provider.default`(`chatgpt`), `provider.chatgpt.model`(선택) | P1 | 필수 | MRD-SCP-001 |
| PRD-CFG-004 | `ui.mode` 값 `fullscreen` 활성화 | P3 | 필수 | MRD-SCP-102 |
| PRD-CFG-005 | 문법 오류와 값 오류: 파일 경로, 행, 키를 포함한 메시지와 종료 코드 8 | P1 | 필수 | MRD-SCN-008 |
| PRD-CFG-006 | 알 수 없는 키: 실행 계속, doctor 경고 표시 | P1 | 필수 | MRD-SCN-008 |
| PRD-CFG-007 | 설정 파일에 자격증명 저장 금지 (P5 API 키는 OS 보안 저장소 또는 환경변수 참조) | P1 | 필수 | MRD-SCP-104 |

### 5.11 howtui doctor

| ID | 요구사항 | 단계 | 우선순위 | MRD |
| --- | --- | --- | --- | --- |
| PRD-DOC-001 | 최소 점검 항목: 버전, OS와 CPU 아키텍처, 터미널(TERM, 크기), 대상 쉘, TTY 사용 가능 여부, UTF-8 로캘, 설정 파일 유효성, 인증 상태 | P1 | 필수 | MRD-SCP-006 |
| PRD-DOC-002 | TUI 렌더링 지원 여부: 제어 터미널 열기와 커서 위치 질의 응답 확인 | P1 | 필수 | MRD-SCP-006 |
| PRD-DOC-003 | LLM Provider 연결 가능 여부 (모델 목록 조회 성공 여부, 추론 요청 미발생) | P1 | 필수 | MRD-SCP-006 |
| PRD-DOC-004 | `--json` 출력 (안정 스키마, `schema_version` 포함) | P1 | 필수 | MRD-SCN-008 |
| PRD-DOC-005 | 민감정보 마스킹: 토큰 미출력, 이메일 마스킹, 홈 경로 `~` 치환, 환경변수 값 미출력(이름과 유무만) | P1 | 필수 | MRD-SCN-008 |
| PRD-DOC-006 | 쉘 통합 등록 상태 점검 | P2 | 필수 | MRD-SCP-101 |
| PRD-DOC-007 | 최신 버전 확인과 업데이트 방법 표시 | P4 | 필수 | MRD-SCP-103 |
| PRD-DOC-008 | Ollama, llama-server 연결 상태, 모델 식별과 로딩 상태 | P6 | 필수 | MRD-SCP-105 |
| PRD-DOC-009 | 종료 코드: 실패 항목 없음 0, 실패 항목 존재 1 (경고는 0) | P1 | 필수 | MRD-SCN-008 |

### 5.12 설치와 업데이트

| ID | 요구사항 | 단계 | 우선순위 | MRD |
| --- | --- | --- | --- | --- |
| PRD-INS-001 | `scripts/install.sh` (POSIX sh): OS와 아키텍처 감지, 릴리스 자산 선택, 버전 고정 옵션, SHA-256 검증, 실패 시 임시 파일 정리, `~/.local/bin` 기본 설치, 설치 경로 옵션, PATH 안내, 재설치 덮어쓰기, 설치 후 `howtui --version` 확인 | P1 | 필수 | MRD-SCP-007, MRD-SCN-009 |
| PRD-INS-002 | `scripts/install.ps1`: Windows PowerShell 5.1과 PowerShell 7 호환, 아키텍처 감지(ARM64 에뮬레이션 고려), SHA-256 검증, 사용자 범위 설치, 사용자 PATH 등록(해제 옵션 제공), 설치 후 버전 확인 | P1 | 필수 | MRD-SCP-007 |
| PRD-INS-003 | ARMv7 판별: `armv6l` 거부, 64비트 커널과 32비트 사용자 공간 조합(Raspberry Pi 4, 400, CM4, 5의 32비트 OS)은 사용자 공간 기준 armv7 선택 | P1 | 필수 | MRD-PLT-004 |
| PRD-INS-004 | 미지원 시스템: 설치 없이 명확한 오류로 종료 | P1 | 필수 | MRD-SCP-007 |
| PRD-INS-005 | 관리자 권한 불필요 (사용자 범위 설치 우선) | P1 | 필수 | MRD-EXC-005 |
| PRD-INS-006 | 대안 경로 문서화: 스크립트 다운로드 후 검토 실행, 릴리스 바이너리 직접 다운로드, 체크섬 독립 검증 절차, 버전 고정 URL | P1 | 필수 | MRD-SCN-009 |
| PRD-INS-007 | 업데이트: 같은 설치 방식 재실행 (기존 설치 경로 유지) | P4 | 필수 | MRD-SCN-009 |
| PRD-INS-008 | 제거 방법 문서화 (바이너리, 설정, 자격증명, 쉘 통합 줄) | P4 | 필수 | MRD-SCN-009 |
| PRD-INS-009 | 설치 스크립트가 검증되지 않은 원격 코드를 실행하지 않음 (다운로드 대상은 릴리스 자산과 체크섬 파일만) | P1 | 필수 | MRD-SCN-009 |

### 5.13 릴리스

| ID | 요구사항 | 단계 | 우선순위 | MRD |
| --- | --- | --- | --- | --- |
| PRD-REL-001 | `vX.Y.Z` 태그 푸시 시 GitHub Actions 빌드와 GitHub Release 생성 | P1 | 필수 | MRD-SCP-007 |
| PRD-REL-002 | 릴리스 자산 명명 규칙과 `SHA256SUMS` 고정 (SRS 13절) | P1 | 필수 | MRD-SCP-007 |
| PRD-REL-003 | 7개 타깃 빌드 자동화 | P4 | 필수 | MRD-SCP-103, MRD-DIF-008 |
| PRD-REL-004 | P1 릴리스 최소 타깃: Linux x64, Linux ARM64, macOS ARM64, Windows x64 (나머지 3종 ARMv7, macOS x64, Windows ARM64는 P1 권장, P4 필수) | P1 | 필수 | MRD-SCP-007 |
| PRD-REL-005 | 릴리스 노트 (변경 사항, 설치 명령, 체크섬 검증 방법) | P1 | 필수 | MRD-SCN-009 |
| PRD-REL-006 | 빌드 출처 증명(artifact attestation) 제공 | P4 | 권장 | MRD-SCN-009 |
| PRD-REL-007 | 공개 릴리스 생성은 사용자 승인 후 수행 | P1 | 필수 | MRD-SCP-007 |

### 5.14 안전

| ID | 요구사항 | 단계 | 우선순위 | MRD |
| --- | --- | --- | --- | --- |
| PRD-SAFE-001 | AI 생성 명령어와 사용자 명령어 자동 실행 금지 (모든 모드, 모든 단계) | P1 | 필수 | MRD-EXC-001, MRD-DIF-002 |
| PRD-SAFE-002 | 규칙 기반 위험 감지: 삭제, 덮어쓰기, 권한 변경, 시스템 변경, 대량 프로세스 종료, 디스크 초기화, 원격 스크립트 실행, 권한 상승 | P1 | 필수 | MRD-SCP-008, MRD-DIF-009 |
| PRD-SAFE-003 | 위험 수준 2단계(주의, 높음). 높음 확정 시 재확인 1회 | P1 | 필수 | MRD-SCP-008 |
| PRD-SAFE-004 | 위험 감지 한계 고지: 완전한 보안 방어 수단 아님 (도움말, 문서) | P1 | 필수 | MRD-SCP-008 |
| PRD-SAFE-005 | 모델 제시 주의 사항은 보조 정보로 설명 영역에 표시, 위험 수준 판정은 규칙 결과 기준 | P1 | 필수 | MRD-SCP-008 |
| PRD-SAFE-006 | howtui 자체 기능(doctor, 인증, 설치, 모델 서버 연결)의 내부 동작과 AI 생성 명령어 실행의 구분 명시 (howtui는 AI 생성 명령어를 어떤 경로로도 실행하지 않음) | P1 | 필수 | MRD-EXC-001 |
| PRD-SAFE-007 | 위험 명령어 테스트는 실제 파일과 시스템을 변경하지 않음 (문자열 판정 테스트만 수행) | P1 | 필수 | MRD-SCP-008 |

### 5.15 개인정보와 전송 데이터

| ID | 요구사항 | 단계 | 우선순위 | MRD |
| --- | --- | --- | --- | --- |
| PRD-PRIV-001 | LLM 전송 컨텍스트 한정: OS 종류, CPU 아키텍처, 대상 쉘 종류와 버전(알려진 경우), 사용자 질문, 현재 명령어, 세션 이력 | P1 | 필수 | MRD-EXC-006 |
| PRD-PRIV-002 | 자동 수집 금지: 환경변수 전체, 토큰, 비밀번호, 파일 내용, 쉘 히스토리, 작업 디렉터리 경로와 파일 목록 | P1 | 필수 | MRD-EXC-006 |
| PRD-PRIV-003 | 전송 데이터 범위 고지: 문서, 로그인 전 고지, `--help` | P1 | 필수 | MRD-EXC-006 |
| PRD-PRIV-004 | 사용자가 수정한 명령어와 쉘 입력란 내용은 요청 제출 시에만 원격 Provider로 전송됨을 고지 | P1 | 필수 | MRD-EXC-006 |
| PRD-PRIV-005 | 로그 파일 기본 비활성. 활성화 시에도 토큰 기록 금지, 질문과 명령어 본문 기록 금지 | P1 | 필수 | MRD-EXC-006 |
| PRD-PRIV-006 | 자격증명과 상태 파일 접근 권한: Unix 0600 (디렉터리 0700), Windows 사용자 프로필 하위 경로 | P1 | 필수 | MRD-SCP-004 |

### 5.16 플랫폼

| ID | 요구사항 | 단계 | 우선순위 | MRD |
| --- | --- | --- | --- | --- |
| PRD-PLAT-001 | 1차 공식 지원 조합: Linux + Bash, macOS + Zsh, Windows + PowerShell | P1 | 필수 | MRD-PLT-001, MRD-PLT-002, MRD-PLT-003 |
| PRD-PLAT-002 | 배포 타깃 7종 (SRS 13절 표) | P4 | 필수 | MRD-DIF-008 |
| PRD-PLAT-003 | Raspberry Pi 2-5 지원 목표 (TUI와 원격 LLM), ARMv6와 Pi 1 미지원 | P4 | 필수 | MRD-PLT-004 |
| PRD-PLAT-004 | 지원 상태 표기 규칙: 실기기 검증, 에뮬레이션 검증, 빌드만 확인, 미검증 구분 | P1 | 필수 | MRD-PLT-004 |

### 5.17 오류 처리

| ID | 요구사항 | 단계 | 우선순위 | MRD |
| --- | --- | --- | --- | --- |
| PRD-ERR-001 | 오류 메시지 구성: 원인 1줄 + 조치 1줄 (예: "로그인 필요" + "howtui login 실행") | P1 | 필수 | MRD-SCN-008 |
| PRD-ERR-002 | TUI 내 LLM 오류는 종료 없이 표시, 재시도 가능 상태 유지 | P1 | 필수 | MRD-SCP-003 |
| PRD-ERR-003 | 사용량 한도 초과 시 리셋 시각 추정 금지, 사용량 관리 URL 안내 | P1 | 필수 | MRD-SCP-004 |
| PRD-ERR-004 | 종료 시그널(SIGTERM, SIGHUP) 수신 시 터미널 복구 후 무출력 종료 | P1 | 필수 | MRD-KPI-003 |
| PRD-ERR-005 | 내부 오류(panic) 시 터미널 복구, stdout 무출력, 종료 코드 101 | P1 | 필수 | MRD-KPI-003 |

---

## 6. 단계별 기능 우선순위 요약

| 단계 | 필수 요구사항 영역 | 비고 |
| --- | --- | --- |
| P1 | CLI(001-007, 010, 011), UI 전체, KEY, EDIT(001-007), LLM(001-012), AUTH(001-006, 008-018), OUT(001-008, 010), SH-010, CFG(001-003, 005-007), DOC(001-005, 009), INS(001-006, 009), REL(001, 002, 004, 005, 007), SAFE 전체, PRIV 전체, PLAT(001, 004), ERR 전체 | MVP |
| P2 | CLI-008, EDIT-008, OUT-009, SH(001-009, 011, 012), DOC-006 | |
| P3 | CLI-009, FS 전체, CFG-004 | |
| P4 | AUTH-007, DOC-007, INS(007, 008), REL(003, 006), PLAT(002, 003) | |
| P5 | LLM(013-017) | |
| P6 | LLM(018-020), DOC-008 | 1.x 마지막 단계 |
| 2.0 이후 | 내장 llama.cpp, 모델 다운로드와 관리, 자동 로컬 모델 선택, 하드웨어별 추론 최적화 | 1.x 범위 외 |
