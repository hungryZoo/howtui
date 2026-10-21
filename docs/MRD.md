# MRD: howtui 시장 요구사항 문서

- 문서 ID: MRD
- 버전: 0.1 (Phase 0 초안)
- 작성일: 2026-10-22
- 상태: 사용자 검토 대기
- 상위 원칙: "기존 터미널을 대체하지 않고, 어떤 환경에서도 AI 명령어 생성·편집을 안전하고 가볍게 제공한다."
- 근거 자료: [경쟁 도구 조사](research/competitors.md), [ChatGPT 로그인 조사](research/chatgpt-oauth.md), [기술 검증 결과](research/spikes.md)

> 원칙 문장은 지시서 원문 인용으로, 원문의 가운뎃점을 그대로 유지한다.

---

## 1. 문제 정의

| ID | 문제 | 근거 |
| --- | --- | --- |
| MRD-PRB-001 | 쉘 명령어의 옵션과 문법을 기억하지 못해 작업 흐름이 끊기고 검색과 문서 탐색에 시간이 소요됨 | 사용자 지시서 1.1 |
| MRD-PRB-002 | 조사한 AI 명령어 도구 3종 모두 생성 결과를 도구 안에서 편집하고 편집 결과를 다시 AI에 보내 수정하는 흐름을 지원하지 않음 | 경쟁 조사 4절 비교표 (howto, how-tui, Atuin AI 모두 TUI 내 직접 편집 미지원) |
| MRD-PRB-003 | 일부 도구는 Enter 한 번으로 생성 명령어를 즉시 실행하므로 검토 없는 실행 위험이 존재함 | 경쟁 조사 3.6절 (Atuin AI의 Enter 즉시 실행) |
| MRD-PRB-004 | PowerShell 사용자를 위한 AI 명령어 입력란 삽입 도구가 부재함 | 경쟁 조사 4절 (세 도구 모두 PowerShell AI 삽입 미지원) |
| MRD-PRB-005 | 별도 계정 서버, 히스토리 DB, 특정 커널 기능(TIOCSTI)에 의존하는 도구는 설치 환경 제약이 큼 | 경쟁 조사 1.4절, 3.4절 |
| MRD-PRB-006 | Raspberry Pi 같은 ARMv7 장치와 Windows ARM64용 배포 바이너리를 제공하는 도구가 부족함 | 경쟁 조사 4절 (ARMv7 릴리스 3개 도구 모두 없음) |

## 2. 타깃 사용자

| ID | 사용자군 | 주 환경 | 핵심 요구 |
| --- | --- | --- | --- |
| MRD-USR-001 | Linux 개발자와 운영자 | Bash, 로컬 또는 SSH 원격 터미널 | 긴 파이프라인과 옵션 조합 생성, 원격 서버에서도 동작 |
| MRD-USR-002 | macOS 개발자 | Zsh, iTerm2, Terminal.app, VS Code 터미널 | 기존 터미널 화면 유지, 빠른 호출 |
| MRD-USR-003 | Windows 사용자 | PowerShell 5.1, PowerShell 7, Windows Terminal | PowerShell 문법 명령어 생성, 입력란 삽입 |
| MRD-USR-004 | Raspberry Pi 사용자 | Raspberry Pi OS (32비트, 64비트), SSH 접속 | 저사양 장치에서 가벼운 실행, 원격 LLM 사용 |
| MRD-USR-005 | ChatGPT 유료 플랜 사용자 | 위 환경 전체 | 별도 API 키 발급 없이 기존 ChatGPT 플랜 사용 |

## 3. 기존 솔루션 및 경쟁 제품 분석

상세 내용과 출처는 [경쟁 도구 조사](research/competitors.md)에 기록. 조사 기준 커밋: howto `d190fa1`, how-tui 1.0.3, Atuin v18.24.1 (`92a8b9a`).

| 도구 | 형태 | 명령어 반환 방식 | 편집과 후속 수정 | 주요 제약 |
| --- | --- | --- | --- | --- |
| antonmedv/howto | Go CLI, TUI 없음 | Zsh 매크로와 TIOCSTI 입력 주입, Windows는 stdout | 쉘 입력란 편집만, 후속 수정 없음, 설명 없음 | Linux 6.2 이상에서 TIOCSTI 비활성화 시 동작 불가 (이슈 #2) |
| noahhefner/how-tui | Python CLI, 실제 TUI 없음 | stdout 목록 출력 | 편집, 후속 수정, 설명 모두 없음 | Python 3.14 이상 필요 |
| Atuin AI | Rust, 인라인 TUI | Bash `bind -x`, Zsh ZLE 위젯, 결과 토큰 방식 | 후속 수정과 설명 지원, TUI 내 직접 편집 미지원 | Hub 계정 또는 자체 Elixir 서버 필요, Enter 즉시 실행, PowerShell AI 미지원 |

경쟁 조사 결과 중 계획 차별화와 상충하는 사실:

- Atuin AI가 인라인 TUI, 후속 수정, 설명, Bash와 Zsh 버퍼 삽입, 단일 Rust 바이너리를 이미 제공함. 해당 항목 단독으로는 차별화 근거 부족.
- Atuin AI가 LLM 판정 기반 위험 명령어 2단계 확인을 제공함. howtui의 위험 경고는 차별화가 아닌 동등 수준 확보 항목.
- crates.io의 `howtui`, `how-tui`, `how_tui` 이름은 2026-10-22 기준 미등록. PyPI에 유사 이름 `how-tui` 존재.

## 4. 핵심 사용 시나리오

| ID | 시나리오 | 흐름 |
| --- | --- | --- |
| MRD-SCN-001 | 자연어 명령어 생성 | `howtui` 실행, 자연어 질문 입력, 명령어와 설명 확인, Enter 확정, stdout 출력 또는 입력란 삽입 |
| MRD-SCN-002 | 생성 명령어 직접 편집 | 생성 결과의 경로나 옵션을 TUI 편집 영역에서 수정 후 확정 |
| MRD-SCN-003 | AI 후속 수정 | "압축도 같이 해줘" 같은 후속 요청으로 현재 편집 내용 기반 수정 |
| MRD-SCN-004 | 명령어 설명 | 직접 수정한 명령어의 동작 설명 요청 |
| MRD-SCN-005 | 쉘 단축키 호출 | 쉘 입력 중 단축키로 호출, 기존 입력을 편집 대상으로 사용, 확정 시 입력란 교체, 실행은 사용자 판단 |
| MRD-SCN-006 | 쉘 통합 없는 사용 | `cmd=$(howtui)` 또는 일반 실행 후 stdout 결과 복사 |
| MRD-SCN-007 | 원격 SSH와 Raspberry Pi 사용 | GUI 브라우저 없는 원격 장치에서 로그인과 명령어 생성 |
| MRD-SCN-008 | 문제 진단 | `howtui doctor`로 환경, 인증, 연결 상태 확인, 민감정보 없는 결과를 버그 보고에 첨부 |
| MRD-SCN-009 | 설치와 업데이트 | 한 줄 설치 명령 또는 검토 후 수동 설치, 같은 방식으로 재설치와 업데이트 |
| MRD-SCN-010 | 긴 대화형 수정 | Fullscreen 모드에서 긴 질문과 설명을 보며 여러 차례 수정 (Phase 3) |

## 5. 차별화 요소

근거 수준 표기: (a) 경쟁 조사로 확인된 차별점, (b) 동등 수준 확보 항목, (c) 구현 후 검증 필요 항목.

| ID | 차별화 요소 | 근거 수준 | 근거 |
| --- | --- | --- | --- |
| MRD-DIF-001 | TUI 내 직접 편집과 편집 결과 기반 AI 후속 수정의 결합 | (a) | 세 도구 모두 TUI 내 편집 미지원 |
| MRD-DIF-002 | 자동 실행 없는 설계 (모든 경로에서 실행은 사용자 판단) | (a) | Atuin AI는 Enter 즉시 실행과 `Shell` 도구 실행 보유 |
| MRD-DIF-003 | Bash, Zsh, PowerShell 3종 입력란 삽입 | (a) | PowerShell AI 삽입 지원 도구 없음 |
| MRD-DIF-004 | 별도 서버, 계정 서버, 히스토리 DB 없는 독립 실행 | (a) | Atuin AI는 Hub 또는 자체 서버 필요 |
| MRD-DIF-005 | 쉘 통합 없이도 순수 stdout 계약으로 사용 가능 | (b) | Atuin `ai inline` stdout 출력 존재(문서 미기재, TUI도 stdout 사용) |
| MRD-DIF-006 | 전체화면을 강제하지 않는 Inline UI | (b) | Atuin AI도 인라인 기본 |
| MRD-DIF-007 | 다중 LLM Provider (ChatGPT 로그인, API 키, 로컬 서버) | (c) | Phase 1은 ChatGPT 로그인 1종, Phase 5와 6에서 확장 |
| MRD-DIF-008 | 7개 타깃 단일 실행 파일 배포 (ARMv7, Windows ARM64 포함) | (a) | ARMv7 릴리스 제공 도구 없음, Windows ARM64는 howto 구판만 제공 |
| MRD-DIF-009 | 규칙 기반 위험 명령어 경고 | (b) | Atuin AI의 위험 확인 기능과 동등 수준 확보 목적 |

## 6. 플랫폼 지원의 시장적 필요성

| ID | 플랫폼 | 필요성 |
| --- | --- | --- |
| MRD-PLT-001 | Linux x64 + Bash | 서버와 개발 환경의 기본 조합, MRD-USR-001 |
| MRD-PLT-002 | macOS ARM64, x64 + Zsh | macOS 기본 쉘 Zsh, Apple Silicon과 Intel 장비 공존, MRD-USR-002 |
| MRD-PLT-003 | Windows x64, ARM64 + PowerShell | PowerShell 5.1 기본 탑재, PowerShell 7 병행 사용, ARM64 Windows 장비 증가, MRD-USR-003 |
| MRD-PLT-004 | Linux ARMv7, ARM64 (Raspberry Pi 2-5) | 32비트 Raspberry Pi OS 사용자와 64비트 사용자 공존, 경쟁 도구 미지원, MRD-USR-004 |

Raspberry Pi 1과 ARMv6는 지원 제외. 실기기 검증 전에는 지원 확정 표기 금지.

## 7. MVP 범위

MVP = Phase 1 (Inline TUI MVP). 상세 단계는 [ROADMAP](ROADMAP.md) 참조.

| ID | MVP 포함 항목 |
| --- | --- |
| MRD-SCP-001 | Inline TUI 기반 질문 입력, 명령어 표시, 설명 표시 |
| MRD-SCP-002 | 명령어 직접 편집 (여러 줄, Undo/Redo, Unicode) |
| MRD-SCP-003 | AI 생성, 후속 수정, 설명, 요청 취소 |
| MRD-SCP-004 | ChatGPT 로그인(Sign in with ChatGPT) 기반 Provider 1종 |
| MRD-SCP-005 | stdout 계약 (확정 명령어만 출력, 취소 시 무출력, TTY 없는 환경 정의) |
| MRD-SCP-006 | 최소 doctor (환경, TTY, 설정, 인증 상태) |
| MRD-SCP-007 | 기본 릴리스 자동화와 설치 스크립트 (Linux, macOS, Windows) |
| MRD-SCP-008 | 규칙 기반 위험 명령어 경고 |

후속 범위 (MVP 이후):

| ID | 후속 항목 | 단계 |
| --- | --- | --- |
| MRD-SCP-101 | Bash, Zsh, PowerShell 선택적 쉘 통합 | Phase 2 |
| MRD-SCP-102 | Fullscreen TUI | Phase 3 |
| MRD-SCP-103 | 7개 타깃 정식 배포, Raspberry Pi 실기기 검증, doctor 고도화 | Phase 4 |
| MRD-SCP-104 | API Key Provider (OpenAI, Gemini, Anthropic, OpenAI 호환) | Phase 5 |
| MRD-SCP-105 | 로컬 LLM 서버 연동 (Ollama, llama-server) | Phase 6 |

## 8. 제외 범위

| ID | 제외 항목 | 비고 |
| --- | --- | --- |
| MRD-EXC-001 | 생성 명령어 자동 실행 | 모든 단계에서 제외 |
| MRD-EXC-002 | 쉘 또는 터미널 에뮬레이터 구현 | 기존 터미널 위에서 동작 |
| MRD-EXC-003 | 공식 지원 3개 조합 외 쉘의 자동 지원 선언 | Fish, Nushell 등 |
| MRD-EXC-004 | 설치 시 rc 파일과 PowerShell Profile 자동 변경 | 사용자 동의 기반 수동 등록 |
| MRD-EXC-005 | 관리자 권한, 쉘 교체, 상시 실행 데몬 요구 | |
| MRD-EXC-006 | 쉘 히스토리, 환경변수 전체, 파일 내용 자동 수집 | |
| MRD-EXC-007 | 비공식 인증 경로 (Codex 인증 파일 읽기, 브라우저 쿠키 추출, 비공식 엔드포인트) | |
| MRD-EXC-008 | llama.cpp 내장, 모델 다운로드와 서버 관리 | 2.0 이후 장기 과제 |
| MRD-EXC-009 | Homebrew, Cargo, Scoop, winget 등록 | 초기 범위 제외 |
| MRD-EXC-010 | Raspberry Pi 1, ARMv6 | |
| MRD-EXC-011 | Raspberry Pi에서의 로컬 LLM 추론 필수 지원 | |
| MRD-EXC-012 | GUI 팝업 오버레이와 동일한 화면 표현 주장 | 터미널 특성상 불가 |

## 9. 사용자 가치 및 성공 기준

| ID | 성공 기준 | 측정 방법 | 목표 단계 |
| --- | --- | --- | --- |
| MRD-KPI-001 | 질문 입력부터 확정까지 터미널 작업 화면 유지 | Inline 모드 화면 캡처 비교, 종료 후 기존 출력 보존 확인 | Phase 1 |
| MRD-KPI-002 | stdout 오염 0건 | 파이프, 리디렉션, 명령 치환 자동 테스트 | Phase 1 |
| MRD-KPI-003 | 비정상 종료 후 터미널 모드 복구 100% | 취소, 오류, panic, 시그널 시나리오 PTY 테스트 | Phase 1 |
| MRD-KPI-004 | 기동 후 첫 화면 표시 시간 측정 기준 확보 | 릴리스 빌드 측정, 목표치는 Phase 1 측정 후 확정 | Phase 1 |
| MRD-KPI-005 | 3개 쉘 입력란 삽입 시 기존 입력 손상 0건 | 쉘별 PTY 테스트 | Phase 2 |
| MRD-KPI-006 | 7개 타깃 설치 스크립트 성공 | 릴리스 후 플랫폼별 설치 테스트, 실기기 결과 기록 | Phase 4 |
| MRD-KPI-007 | 명령어 정확도 측정 체계 확보 | 평가 질문 세트와 채점 기준 수립, Provider별 측정 | Phase 4 이후 |

## 10. 주요 리스크

| ID | 리스크 | 영향 | 대응 | 상태 |
| --- | --- | --- | --- | --- |
| MRD-RSK-001 | ChatGPT 로그인 경로가 "limited preview" 상태, 신규 동적 등록 앱 활성화 여부 미확인 | Phase 1 Provider 부재 가능성 | 실계정 실측을 Phase 1 첫 작업으로 배치, 불가 시 API Key Provider 선행 여부 사용자 결정 | 미해결 |
| MRD-RSK-002 | ChatGPT 로그인 사용 자격 문구 출처 간 불일치 (Plus, Pro 한정 또는 전체 사용자) | 사용자 범위 축소 | Plus 계정 실측, 문서에 확인된 범위만 표기 | 미해결 |
| MRD-RSK-003 | ChatGPT 로그인 경로의 구조화 출력 지원 미확인, `temperature` 등 미지원 | 응답 형식 불안정 | 지시문 기반 JSON 응답과 단계적 파싱 폴백 | 설계 반영 |
| MRD-RSK-004 | device code 흐름 부재 | SSH 전용 장치의 첫 로그인 불편 | SSH 포트 포워딩 안내, 리다이렉트 URL 붙여넣기(실측 필요), 공식 자격증명 이전 절차 | 설계 반영 |
| MRD-RSK-005 | Atuin AI와의 기능 중첩 | 차별성 약화 | 직접 편집, 자동 실행 없음, PowerShell, 독립 실행 중심 포지셔닝 | 설계 반영 |
| MRD-RSK-006 | crossterm 커서 위치 질의의 stdout 출력 | stdout 오염 | fd 1 임시 전환 기법 검증 완료 (SPIKE-01) | Linux 검증 완료, macOS와 Windows 미검증 |
| MRD-RSK-007 | Windows 콘솔에서 stdout 리디렉션과 TUI 렌더링 병행 동작 미검증 | Windows stdout 계약 미충족 가능성 | Phase 1 초기 Windows 실환경 검증 | 미해결 |
| MRD-RSK-008 | PowerShell 위젯 핸들러의 네이티브 출력 캡처 인코딩 (한국어 Windows 코드 페이지) | 한글과 이모지 깨짐 | 출력 파일 기반 전달 방식 채택 (ADR-0009), Phase 2 실측 | 설계 반영 |
| MRD-RSK-009 | Raspberry Pi 실기기 미보유 시 검증 불가 | 지원 표기 지연 | qemu 에뮬레이션 결과와 실기기 결과 구분 표기 | 부분 검증 (qemu) |
| MRD-RSK-010 | SIWC 약관의 DevKit 비상업 라이선스 참조 조항 해석 불명확 | 라이선스 위험 | DevKit 코드 미사용, 공개 문서 기반 독자 구현, 사용자 법률 검토 | 미해결 |
