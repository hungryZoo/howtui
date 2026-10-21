# ADR-0015 자체 편집 버퍼 구현

- 상태: 제안
- 일자: 2026-10-22
- 관련: PRD-EDIT 계열, SRS 7.4절, SRS-TUI-011

## 맥락

- 지시서: ratatui-textarea 등 검토.
- ratatui-textarea 0.9.3: ratatui 0.30 호환, Undo/Redo 내장(기본 50단계, Ctrl+U/Ctrl+R), `input_without_shortcuts`로 키 재정의 가능, CJK 가로 스크롤 커서 소실 결함(#35) 미해결 (research/rust-crates.md 4절).
- tui-textarea 0.7: ratatui 0.29 의존으로 부적합.

## 결정

- `app::editor::TextBuffer` 자체 구현 (grapheme 단위 커서, Unicode 표시 폭, 여러 줄, Undo/Redo, 읽기 전용).
- 렌더링(줄바꿈, 스크롤, 실제 커서 배치)은 ui 계층 담당.

## 근거

- (a) `app` 순수 코어 규칙: 위젯 crate가 ratatui에 의존하므로 상태 계층에 포함 시 의존 규칙 위반.
- (b) CJK 결함 회피와 한국어 입력 정확성 직접 통제.
- (c) AI 응답 반영을 Undo 1단계로 기록하는 정책(PRD-EDIT-006) 직접 구현.

## 결과

- 구현과 테스트 비용 증가 (Phase 1 작업 1-06). 비용이 과도하면 ratatui-textarea를 ui 계층에 두고 상태 동기화하는 대안으로 ADR 갱신.

## 대안

- (a) ratatui-textarea 사용: 기능 풍부, 위 (a)-(c) 사유로 보류.
