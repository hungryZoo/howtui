# ADR-0007 최초 Provider: Sign in with ChatGPT

- 상태: 제안 (실계정 실측 전 조건부)
- 일자: 2026-10-22
- 관련: PRD-AUTH 계열, PRD-LLM-009, SRS 11절, research/chatgpt-oauth.md

## 결정

- 공식 SIWC 오픈소스 로컬 앱 경로 채택: 동적 등록(`dynamic_agent_client`), 공개 클라이언트, PKCE S256, `127.0.0.1` 루프백 콜백 `/auth/callback`, `resource=https://api.openai.com/v1`, scope `openid profile email offline_access resource.invoke chatgpt.tokens.use.direct`.
- 추론 경로: `POST https://api.openai.com/v1/responses` (`store: false`, `stream: true`, `instructions` 사용, 미지원 필드 미포함). 모델 목록: `GET https://api.openai.com/v1/models`.
- `agent_name_hint` 고정값 `howtui`.
- OpenAI DevKit 코드 복사와 번역 이식 금지. 공개 문서만으로 독자 구현.
- 헤드리스: 루프백 포트 지정과 SSH 포트 포워딩 안내, 리다이렉트 URL 붙여넣기 기본 제공 (사용자 결정 E-04), 공식 자격증명 이전 절차는 P4.

## 근거

- 개발자 측 등록, client secret, 파트너 API 키 불필요 (공식 문서 원문 인용, research/chatgpt-oauth.md 1.1절).
- 지시서의 비공식 인증 경로 금지 조건 충족.

## 위험과 조건

- "limited preview" 상태, 신규 동적 등록 앱 활성화 여부 미확인 (`subscription_sharing_v2_client_not_enabled` 가능성).
- 사용 자격(Plus, Pro 또는 전체 사용자), 허용 지역(한국 포함 여부) 미확인.
- 위 항목은 Phase 1 실계정 실측에서 확인. 불가 판정 시 대체 Provider 없이 사용 불가 상태 유지 (사용자 결정 E-01, 2026-10-22). API Key Provider는 계획대로 Phase 5.

## 대안

- (a) Codex CLI 인증 파일 재사용: 지시서 금지 항목, 기각.
- (b) Codex app-server 경유: Codex 설치 전제, 독립 실행 원칙 위배로 기각.
