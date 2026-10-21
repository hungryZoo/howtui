# ADR-0004 상태 갱신 구조와 렌더러 분리

- 상태: 제안
- 일자: 2026-10-22
- 관련: PRD-UI-013, PRD-FS-002, SRS 7절

## 결정

- 단방향 3단계 구조: (1) 입력 변환 `keymap(Event, &State) -> Option<Action>`, (2) 상태 갱신 `update(&mut State, Action) -> Vec<Effect>`, (3) 효과 실행 `runtime`이 Effect 처리 후 결과를 Action으로 재주입.
- `State`와 `update`는 순수 코드. 터미널, 네트워크, 시간 접근 없음 (시간은 Action으로 주입).
- 렌더러는 `fn render(&State, &mut Frame)` 형태의 모드별 함수 (`ui::inline`, `ui::fullscreen`). 렌더러는 상태 변경 금지.
- 화면 모드 전환은 렌더러와 터미널 세션 설정만 교체, `State`, `update`, LLM, 편집 로직 재사용.

## 근거

- 상태 전이 표(PRD 3절)를 `update` 단위 테스트로 직접 검증 가능.
- Fullscreen 추가 시 렌더러와 레이아웃만 신규 작성 (PRD-FS-002).

## 대안

- (a) 위젯 내부 상태 중심 구성: 모드 추가 시 로직 중복, 테스트 곤란으로 기각.
