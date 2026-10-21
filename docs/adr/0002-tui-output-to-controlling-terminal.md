# ADR-0002 TUI 출력 대상: 제어 터미널과 fd 1 임시 전환

- 상태: 제안 (Unix 실측 검증 완료, Windows 미검증)
- 일자: 2026-10-22
- 관련: PRD-OUT-001, PRD-OUT-002, PRD-OUT-006, SRS 6절, SPIKE-01

## 맥락

- stdout에는 확정 명령어만 출력해야 함.
- crossterm 0.29.0 `cursor::position()`은 커서 위치 질의 `\x1B[6n`을 항상 `io::stdout()`에 기록 (`src/cursor/sys/unix.rs:32-36`).
- ratatui 0.30 Inline 뷰포트는 초기화와 리사이즈 때 커서 위치 질의 수행 (ratatui-core 0.1.2 `inline.rs:396`, `resize.rs:24-37`).
- SPIKE-01 `--naive` 실측: stderr 계열 핸들로만 렌더링 시 stdout에 `\033[6n` 유입, 커서 위치 응답 시간 초과 오류.

## 결정

- Unix: 시작 시 `/dev/tty`를 읽기와 쓰기로 열어 ratatui 백엔드 출력 대상으로 사용. TUI 실행 동안 `dup`으로 원래 fd 1 보관, `dup2`로 fd 1을 `/dev/tty`에 연결. 종료 시 fd 1 복구 후 명령어 출력.
- Windows: stdout 핸들 무변경. 출력 대상은 stdout(콘솔일 때) → stderr(콘솔일 때) → `CONOUT$` + `WriteConsoleW` 래퍼 순서로 선택 (SRS 6.1절). crossterm Windows 커서 위치 질의는 `CONOUT$` 기반 콘솔 API 사용으로 stdout 무관 (소스 확인). 파일 핸들로 연 `CONOUT$`에 UTF-8을 쓰는 방식은 콘솔 코드 페이지 의존이므로 미사용. Phase 1 초기에 실측.
- 제어 터미널 열기 실패 시 TUI 미시작, 종료 코드 3.

## 근거

- SPIKE-01 12개 케이스 실측: 파이프, 명령 치환, 파일 리디렉션, stdin 파이프 모두 stdout 순수성 유지, Unicode 여러 줄 바이트 일치, 취소와 panic 후 터미널 모드 복구.
- fd 1 전환은 crossterm 내부 구현 변경 없이 적용 가능. TUI 실행 중 실수로 stdout에 쓰는 코드가 있어도 stdout 오염 방지.

## 결과

- 장점: crossterm과 ratatui 수정 없이 stdout 계약 보장.
- 단점: Unix 전용 `unsafe` 호출(`dup`, `dup2`) 필요. 대응: `platform::tty` 모듈 한 곳에 격리, 복구 경로 단위 테스트와 PTY 테스트.
- 위험: crossterm 향후 버전의 질의 출력 대상 변경 가능성. 대응: 의존성 갱신 시 SPIKE-01 대응 TC(TC-OUT 계열) 재실행.

## 대안

- (a) 커서 위치 질의를 직접 구현한 사용자 정의 Backend: crossterm 내부 이벤트 필터 비공개로 응답 수신 경로 확보 곤란, 기각.
- (b) `Viewport::Fixed`로 커서 질의 회피: 시작 위치 계산을 위해 결국 커서 위치 필요, 기각.
- (c) stdout 대신 출력 파일만 사용: 쉘 통합 없는 stdout 계약 미충족, 기각.
