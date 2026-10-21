# ADR-0009 쉘 통합 프로토콜: 환경변수 입력, 출력 파일 결과

- 상태: 제안 (입력란 교체 API는 SPIKE-03, 04, 05 실측. 출력 파일 방식은 미실측)
- 일자: 2026-10-22
- 관련: PRD-SH 계열, PRD-OUT-009, SRS 9절, research/shell-integration.md

## 맥락

- SPIKE-03, 04, 05: `$(...)` 캡처 방식으로 Bash, Zsh, PowerShell(Linux) 입력란 교체 성공 (fd 1 전환 적용 바이너리).
- PowerShell 위젯 핸들러 안 `& exe` 실행은 stdout 리디렉션 발생, 캡처 문자열은 원래 콘솔 인코딩(한국어 Windows CP949 추정)으로 해석되어 UTF-8 한글 손상 위험.
- PowerShell 5.1과 7.3 이상은 네이티브 인자 인용 규칙이 다름.
- 명령 치환은 끝 줄바꿈 제거.

## 결정

- 위젯 모드 `howtui --widget`: 입력은 환경변수(`HOWTUI_SHELL`, `HOWTUI_BUFFER`, `HOWTUI_LBUFFER`, `HOWTUI_OUTPUT_FILE`, 선택 `HOWTUI_SHELL_VERSION`), 결과는 쉘이 만든 임시 파일(UTF-8, BOM 없음, LF, 끝 줄바꿈 없음).
- 3개 쉘 공통 프로토콜. TUI는 터미널에 직접 연결 (Bash, Zsh `</dev/tty >/dev/tty 2>/dev/tty`, PowerShell 리디렉션 없는 Process).
- 판정: 종료 코드 0이고 파일이 비어 있지 않을 때만 입력란 교체.
- 기본 단축키 Ctrl+G, `--key`, `--no-bind` 제공.
- stdout 캡처는 직접 실행 모드(`cmd=$(howtui)`)에서만 사용.

## 근거

- 인코딩, 인자 인용, 끝 줄바꿈 문제를 3개 쉘에서 동일 방식으로 제거.
- 쉘 측 코드가 단순 (파일 읽기 1회).

## 결과

- 임시 파일 생성과 삭제 비용, 파일 경로 권한 관리 필요 (`mktemp`, `GetTempFileName` 사용자 전용 경로).
- Phase 2에서 출력 파일 방식 실측 필요 (TC-SH 계열).

## 대안

- (a) stdout 캡처 (SPIKE 방식): Unix는 동작 확인, PowerShell 인코딩 위험으로 공통 방식에서 기각.
- (b) stdout과 stderr 교환 (Atuin 방식): 진단 출력 혼입 위험, PowerShell 적용 불가로 기각.
- (c) fd 3: PowerShell 개념 부재로 기각.
