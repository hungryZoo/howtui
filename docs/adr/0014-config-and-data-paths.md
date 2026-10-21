# ADR-0014 설정과 데이터 경로

- 상태: 제안
- 일자: 2026-10-22
- 관련: PRD-CFG-001, SRS 12.1절

## 결정

- Linux와 macOS: XDG 형식 경로 (`~/.config/howtui/config.toml`, `~/.local/share/howtui/`, `~/.cache/howtui/`), XDG 환경변수 존중.
- Windows: `%APPDATA%\howtui\config.toml`, `%LOCALAPPDATA%\howtui\`.
- 재정의: `HOWTUI_CONFIG`, `HOWTUI_DATA_DIR`.
- 설정(사용자 편집)과 데이터(상태, 자격증명 파일, 잠금) 분리.

## 근거

- CLI 도구의 macOS XDG 경로 관례 (gh, starship 등), 사용자가 설정 파일을 찾기 쉬운 경로.
- SIWC 문서 예시 경로(`~/.config/YOUR_TOOL/`)와 같은 계열.
- 설정 파일 공유 시 비밀값 노출 방지 목적의 분리.

## 대안

- (a) macOS `~/Library/Application Support/howtui`: OS 관례 부합, CLI 사용자 접근성 낮음으로 기각.
