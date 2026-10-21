# howtui 개발 로드맵

- 버전: 0.1 (Phase 0 초안)
- 작성일: 2026-10-22
- 기준 문서: [MRD](MRD.md), [PRD](PRD.md), [SRS](SRS.md), [TC](TC.md)
- 원칙: 각 Phase는 독립 검증 가능 단위. 완료 조건 미충족 시 다음 Phase 착수 금지. 명세에 없는 기능은 요구사항 변경 기록 후 착수.

## 1. Phase 요약

| Phase | 목표 | 완료 조건 (게이트) | 상태 |
| --- | --- | --- | --- |
| 0 | SDD 문서와 기술 검증 | 요구사항 간 모순 없음, 기술 리스크와 미해결 항목 기록, 사용자 검토 승인 | 검토 대기 |
| 1 | Inline TUI MVP | 지정 주요 OS에서 생성, 편집, 확정, 출력 일관 동작. P1 필수 TC 통과 | 대기 |
| 2 | 선택적 쉘 통합 | 3개 쉘에서 자동 실행 없이 입력란 안전 전달. P2 필수 TC 통과 | 대기 |
| 3 | Fullscreen TUI | 두 모드 동일 기능, 설정 선택 가능. P3 필수 TC 통과 | 대기 |
| 4 | 배포와 1.x 안정화 | 지원 플랫폼에서 문서화된 절차로 설치, 진단, 업데이트 가능 | 대기 |
| 5 | API Key Provider | 복수 Provider를 동일 UI와 로직으로 사용 | 대기 |
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
| 1-02 | 1-01 결과 반영: ADR-0007 확정 또는 대체 Provider 결정 (E-01) | ADR-0007 | 1-01 | 문서 갱신 |
| 1-03 | 저장소 기반: `Cargo.toml`, 툴체인 고정, CI(fmt, clippy, test 3개 OS, 추적성 검사, grep 검사), LICENSE | SRS-BLD-007, SRS-TEST-005 | 라이선스 결정 (E-02) | TC-NFR-001, TC-NFR-010 |
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

## 4. Phase 2-6 작업 개요

| Phase | 작업 | 검증 TC |
| --- | --- | --- |
| 2 | `init` 스크립트 3종, 어댑터 프로토콜(`--adapter`, `HOWTUI_BUFFER`, 출력 파일), 단축키, 입력란 초기값, 대체 경로, doctor 통합 점검, 쉘별 지원 버전 표기 | TC-SH-001-014, TC-EDIT-009, TC-OUT-010, TC-CLI-007 |
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
| U-5 | 라이선스 결정 | Phase 1 작업 1-03 |
| U-6 | 공개 릴리스와 원격 push 승인 | 각 릴리스 |
| U-7 | Unsloth Gemma 4 모델 벤치마크 결과 제공 | Phase 6 |

## 6. 미해결 사항 (사용자 결정 필요)

| ID | 사항 | 선택지 | 권장안 | 결정 시점 |
| --- | --- | --- | --- | --- |
| E-01 | ChatGPT 로그인 경로 사용 불가 판정 시 Phase 1 Provider | (a) OpenAI API Key Provider를 Phase 1로 앞당김 (b) Phase 1을 Mock Provider 기준으로 완료하고 실연동은 보류 | (a). 요구사항 변경 기록 후 PRD-LLM-013을 P1로 이동, ChatGPT 로그인은 활성화 확인 시 추가 | 작업 1-01 직후 |
| E-02 | 저장소 라이선스 | (a) MIT (b) MIT OR Apache-2.0 (c) 기타 | (a) MIT (tu와 동일). SIWC DevKit 코드는 사용하지 않는 전제 | 작업 1-03 이전 |
| E-03 | TUI와 CLI 메시지 언어 | (a) 영어 단일 (b) 한국어 단일 (c) 다국어 | (a). 공개 저장소 대상 범용성. LLM 설명은 질문 언어를 따르므로 한국어 질문에는 한국어 설명 | 작업 1-07 이전 |
| E-04 | 리다이렉트 URL 붙여넣기 로그인 | (a) 실측 성공 시 기본 활성 (b) 실험 기능 옵션으로만 제공 (c) 미제공 | (b). SIWC 문서 미기재 경로 | 작업 1-01 직후 |
| E-05 | 쉘 통합 실패 시 "복사 방식" 범위 | (a) 터미널 표시 후 수동 복사 (b) OSC 52 클립보드 복사 추가 | (a). (b)는 터미널별 지원 차이와 원격 클립보드 쓰기 보안 고려 필요 | Phase 2 착수 전 |
| E-06 | Windows 코드 서명, macOS 공증 | (a) 1.x 미적용 (b) 인증서 구매 후 적용 | (a). curl, irm 경로는 공증 불필요 판단. Smart App Control 강제 모드 차단 사례 수집 후 재검토 | Phase 4 |
| E-07 | 위험 높음 명령어 확정 재확인(Enter 2회) | (a) 유지 (b) 경고 표시만 | (a). 확정과 실행이 분리되어 있어 부담 작음 | Phase 1 검토 시 |
