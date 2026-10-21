# ADR-0001 단일 crate 구조와 모듈 경계

- 상태: 제안 (Phase 0, 사용자 승인 대기)
- 일자: 2026-10-22
- 관련: SRS 3절, PRD-FS-002, 지시서 14절

## 맥락

- 지시서 요구: 초기부터 다중 crate workspace 강제 금지, 코어 로직의 OS와 TUI 구현 분리.
- Inline과 Fullscreen 두 화면 모드가 같은 상태와 LLM 로직 공유 필요 (PRD-FS-002).

## 결정

- 저장소 루트에 단일 바이너리 crate `howtui` 구성 (`src/main.rs` + 하위 모듈).
- 모듈 의존 방향 고정: `main` → `cli` → (`app`, `ui`, `llm`, `auth`, `shell`, `output`, `config`, `safety`, `doctor`, `platform`).
- `app` 모듈은 `ui`, `platform`, 네트워크 구현에 의존하지 않음. 입력은 `app::Action`, 출력은 `app::Effect` 값으로만 교환.
- `llm`, `auth`의 네트워크 구현은 trait 뒤에 배치, `app` 테스트는 Mock 구현 사용.
- 실험 코드는 `spikes/` 아래 독립 crate(`[workspace]` 단독 선언)로 유지, 제품 crate와 분리.

## 근거

- 모듈 경계만으로 테스트 격리와 화면 모드 교체 가능. crate 분리는 독립 배포 대상이 생길 때 수행.
- 단일 crate는 빌드 설정, 버전 관리, 릴리스 스크립트 단순화.

## 결과

- 장점: 빌드 설정 1개, 의존성 버전 일원화, 리팩터링 비용 감소.
- 단점: 모듈 경계 위반을 컴파일러가 막지 않음. 대응: 리뷰 체크리스트와 `app` 모듈의 `use` 제한 규칙(SRS-MOD-002) 점검.

## 대안

- (a) workspace (`howtui-core`, `howtui-tui`, `howtui-cli`): 경계 강제 장점, 초기 비용과 지시서 조건 위배로 기각.
