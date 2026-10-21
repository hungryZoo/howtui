# ADR-0006 LLM Provider 인터페이스와 표준 응답

- 상태: 제안
- 일자: 2026-10-22
- 관련: PRD-LLM-001-005, PRD-LLM-010, SRS 5절

## 결정

- 작업 3종 `Operation::{Generate, Revise, Explain}` + 취소 토큰 기반 Cancel.
- Provider trait 단일 메서드: `complete(LlmRequest, CancelToken) -> Result<LlmResponse, LlmError>`.
- 표준 응답: `command: Option<String>`, `explanation: String`, `notes: Vec<String>`, `model: String`.
- 응답 형식: JSON 객체 `{"command", "explanation", "notes"}` 요구. 구조화 출력 기능(JSON schema) 지원 Provider는 해당 기능 사용, 미지원 또는 미확인 Provider는 지시문 기반 요구 후 단계적 파싱 (JSON, 코드 블록 내 JSON, 첫 코드 블록과 나머지 본문 분리).
- 형식 오류 시 자동 재시도 1회 (PRD-LLM-010).
- 오류는 `LlmError` 공통 분류로 정규화 (SRS 8절).

## 근거

- ChatGPT 로그인 경로의 구조화 출력 지원 미확인, `temperature` 미지원 (research/chatgpt-oauth.md 5.3절). 지시문과 파싱 폴백으로 형식 안정성 확보.
- API 키 Provider(P5), 로컬 서버(P6)도 같은 trait로 추가 가능. 로컬 서버 2종은 OpenAI 호환 채팅 API 공유 (research/local-llm.md).

## 대안

- (a) Provider별 함수 호출(tool calling) 기반 응답: ChatGPT 경로의 namespace 도구 실제 동작 미확인, Provider 간 형식 차이로 1차 채택 보류. 실측 후 ADR 갱신 가능.
