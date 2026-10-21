# ChatGPT 로그인(Sign in with ChatGPT) 기반 플랜 사용 경로 조사

- 조사일: 2026-10-22
- 조사 대상: OpenAI 공식 "Sign in with ChatGPT"(이하 SIWC)의 오픈소스 로컬 앱용 "ChatGPT plan usage"(토큰 공유) 경로
- 적용 대상: howtui (Rust CLI/TUI, 자연어 기반 셸 명령 생성, 첫 공급자로 ChatGPT 로그인 사용)
- 조사 범위: developers.openai.com/siwc 전체 문서(Markdown 원문), learn.chatgpt.com, help.openai.com, openai.com/policies, auth.openai.com 공개 discovery 문서, github.com/openai/sign-in-with-chatgpt-devkit
- 표기 규칙: 공식 출처에서 확인 불가 항목은 "미확인"으로 표기. 원문 정확성이 중요한 부분은 영문 인용 블록 유지.
- 조사 방식: 공개 문서 열람만 수행. 가입, 양식 제출, 자격 증명 전송, 실제 로그인 시도 없음.

---

## 0. 핵심 결론 요약

| 항목 | 결론 |
| --- | --- |
| 개발자 사전 등록 | 불필요. 사용자 로그인 시점의 동적 등록(`client_id=dynamic_agent_client`)으로 `client_id` 발급 |
| client secret, 파트너 API 키 | 불필요. 공개 클라이언트(public client) |
| 인가 엔드포인트 | `https://auth.openai.com/api/accounts/authorize` |
| 토큰 엔드포인트 | `https://auth.openai.com/api/accounts/oauth/token` |
| 폐기 엔드포인트 | `https://auth.openai.com/api/accounts/oauth/revoke` (discovery 문서 기준) |
| PKCE | `S256` 전용 |
| redirect URI | `http://127.0.0.1:<port>/auth/callback`, 포트만 가변, `localhost` 금지 |
| scope | `openid profile email offline_access resource.invoke chatgpt.tokens.use.direct` |
| resource | `https://api.openai.com/v1` |
| 추론 API | `POST https://api.openai.com/v1/responses`, `store: false`, `stream: true` 필수 |
| 모델 목록 | `GET https://api.openai.com/v1/models` (응답의 `models[]`, `slug`, `display_name`, `visibility`) |
| 토큰 수명 | access token 1시간, refresh token 30일(회전식, 갱신마다 30일 재부여) |
| device code 흐름 | SIWC에는 없음. discovery의 `grant_types_supported`는 `authorization_code`, `refresh_token` 2종 |
| 헤드리스 공식 경로 | 브라우저가 있는 로컬 기기에서 OAuth 완료 후 자격 증명 파일을 SSH로 VM에 전송 |
| 사용 자격 | 개발자 문서 기준 Plus, Pro. 도움말 센터는 상용 앱 기준 Go, Plus, Pro 및 "오픈소스 도구는 모든 ChatGPT 사용자" 표기. 출처 간 불일치 존재 |
| 상태 | "limited preview" (2026-09-29 ChatGPT 릴리스 노트), 문서에 "Preview limitations" 별도 페이지 |
| 공식 SDK | TypeScript(Node.js) DevKit 1종, 비상업 라이선스. 공식 Rust 구현 없음 |
| howtui 실현 가능성 | 조건부 가능. 무료 오픈소스 로컬 도구 요건 충족 시 개발자 측 등록 절차 없이 구현 가능. 헤드리스 흐름 부재, 사용 자격 불일치, 프리뷰 제약, 약관의 DevKit 라이선스 참조가 주요 위험 |

---

## 1. 오픈소스 앱 등록 절차

### 1.1 등록 주체와 발급 대상

- 등록 주체: 최종 사용자. 개발자 측 사전 등록 절차 없음.
- 첫 로그인 시 `client_id=dynamic_agent_client`로 인가 요청, 사용자가 에이전트 이름을 확인하고 승인하면 콜백으로 발급된 `client_id` 수신.

> Start first-time registration with `client_id=dynamic_agent_client`, include the host's `ext_agent_host_id`, and set `agent_name_hint` to your app's actual name, used consistently across installations. After the user names and authorizes the agent, the callback returns its issued `client_id`. Save it with the registration for that ChatGPT account and reuse it for later sign-ins to the same account. Send the same `ext_agent_host_id` for the same host. `dynamic_agent_client` is the first-time registration entrypoint, not the client ID to save or use for token exchange. This direct flow needs neither a client secret nor a partner API key.
>
> (출처: https://developers.openai.com/siwc/token-sharing-open-source/sign-in)

- 발급 형식: `oaiapp_...` 형태의 `client_id`.

> For new registration, retain the issued `client_id`, such as `oaiapp_...`. Do not save `dynamic_agent_client` as the connection's issued ID. If a new-registration callback does not include an issued ID, treat registration as incomplete.

### 1.2 발급 단위 (앱 단위, 설치 단위 구분)

- `client_id`: 사용자와 워크스페이스 조합 단위. 앱 전체 공용 ID 아님. 설치(호스트) 단위도 아님.
- host ID(`ext_agent_host_id`): 설치(호스트) 단위, 앱이 직접 생성.

> In this dynamic-registration flow, each issued `client_id` is bound to the authenticated user and the workspace selected during registration. Its issued `client_id` identifies that registration, its security boundaries, and its ChatGPT plan usage settings.
>
> An issued `client_id` can be reused across different agent hosts for the same user and workspace, but each host must have its own distinct host identifier.
>
> (출처: https://developers.openai.com/siwc/token-sharing-open-source)

- 쿡북 표현: 앱이 host ID 생성, OpenAI가 로그인 중 별도 client ID 발급.

> **The app generates the host ID; OpenAI issues a separate client ID during sign-in.**
>
> (출처: https://developers.openai.com/cookbook/articles/sign-in-with-chatgpt)

### 1.3 개발자 계정, 심사, 대기자 명단

- 오픈소스 앱: 심사 또는 대기자 명단 절차 언급 없음. 동적 등록으로 바로 사용.
- 상용(유료) 앱, 원격 호스팅 앱: 관심 양식(interest form) 제출 필요.

> Sign in with ChatGPT is currently available to selected commercial partners through a limited trial. ChatGPT plan usage is available to all open-source partners and selected private clients.
>
> (출처: https://developers.openai.com/siwc/quickstart)

> _These docs explain ChatGPT plan usage for open-source and locally hosted apps. If you're interested in offering it in a paid or remotely hosted app, complete the [interest form](https://openai.com/form/sign-in-with-chatgpt-interest/)._
>
> (출처: https://developers.openai.com/siwc/token-sharing-open-source)

> The ChatGPT plan usage integration described here is available for open-source tools and personal projects that run locally. If you're building a paid or remotely hosted app, [join the waitlist](https://openai.com/form/sign-in-with-chatgpt-interest/) to request access before offering it to users.
>
> (출처: https://developers.openai.com/cookbook/articles/sign-in-with-chatgpt)

- "Request a client ID" 페이지: 상용 파트너 대상 대기자 명단 안내만 존재. 오픈소스 동적 등록과 별개.

> Sign in with ChatGPT is currently offered to a select group of commercial partners. To join the waitlist, complete the [Sign in with ChatGPT interest form](https://openai.com/form/sign-in-with-chatgpt-interest/).
>
> (출처: https://developers.openai.com/siwc/request-client-id)

### 1.4 개발자 비용

- 개발자 측 수수료, 등록비: 공식 문서 내 언급 없음 (미확인, 단 비용 언급 자체가 없음).
- 사용자 과금 금지 조건: SIWC 약관의 "No charge" 조항.

> **No charge.** Users must be able to use their ChatGPT plan through SIWC without paying you or upgrading to a paid version of your application.
>
> (출처: https://openai.com/policies/sign-in-with-chatgpt-terms/)

---

## 2. OAuth 흐름 상세

### 2.1 엔드포인트 (discovery 문서)

`https://auth.openai.com/.well-known/openid-configuration` 실측 결과(2026-10-22 조회) 주요 값:

```json
{
  "issuer": "https://auth.openai.com",
  "authorization_endpoint": "https://auth.openai.com/api/accounts/authorize",
  "token_endpoint": "https://auth.openai.com/api/accounts/oauth/token",
  "revocation_endpoint": "https://auth.openai.com/api/accounts/oauth/revoke",
  "jwks_uri": "https://auth.openai.com/.well-known/jwks.json",
  "userinfo_endpoint": "https://auth.openai.com/api/accounts/oauth/userinfo",
  "scopes_supported": ["openid", "profile", "email", "offline_access"],
  "response_types_supported": ["code"],
  "code_challenge_methods_supported": ["S256"],
  "response_modes_supported": ["query"],
  "grant_types_supported": ["authorization_code", "refresh_token"],
  "id_token_signing_alg_values_supported": ["RS256"],
  "token_endpoint_auth_methods_supported": ["client_secret_basic", "client_secret_post", "none"],
  "revocation_endpoint_auth_methods_supported": ["client_secret_basic", "client_secret_post", "none"]
}
```

- 참고: `scopes_supported`에 `resource.invoke`, `chatgpt.tokens.use.direct` 미포함. 문서상 해당 scope 요청은 명시되어 있으므로 discovery 목록은 전체 목록 아님으로 해석.
- `device_authorization_endpoint` 항목 없음.

### 2.2 인가 요청 파라미터 (원문 표)

> Open the system browser at `https://auth.openai.com/api/accounts/authorize` and URL-encode each query parameter value.
>
> (출처: https://developers.openai.com/siwc/token-sharing-open-source/sign-in)

| 파라미터 | 원문 요지 |
| --- | --- |
| `client_id` | 최초 등록 시 `dynamic_agent_client`, 이후 발급된 client ID 재사용 |
| `agent_name_hint` | 앱 실제 이름, 설치 간 일관 사용, 최초 동적 등록 요청에만 포함 |
| `ext_agent_host_id` | 필수, 호스트별 고유 값 |
| `id_token_hint` | 재인가 시 이전 성공 로그인의 ID 토큰(만료 허용) |
| `login_hint` | 재인가 시 선택 사항, 검증된 ID 토큰의 이메일 |
| `response_type` | `code` |
| `redirect_uri` | `http://127.0.0.1:<port>/auth/callback` |
| `scope` | `openid profile email offline_access resource.invoke chatgpt.tokens.use.direct` |
| `resource` | `https://api.openai.com/v1` |
| `state` | 시도별 새 난수 |
| `nonce` | 시도별 새 난수 |
| `code_challenge_method` | `S256` |
| `code_challenge` | PKCE verifier의 SHA-256 다이제스트, base64url, 패딩 없음 |

주요 원문:

> `agent_name_hint`: Your app's actual name, such as `OpenClaw`. Use it consistently across installations and include it only on initial dynamic-registration requests. Omit it from reauthorization with an issued client ID. The user can edit the name before approval; it is display metadata, not identity.

> `ext_agent_host_id`: Required host identifier, distinct for each host.

> `id_token_hint`: For reauthorization, the ID token retained from this account's previous successful sign-in. The hint may be expired; it identifies the account, not a valid authenticated session. Omit it after signing out or if none was retained.

### 2.3 PKCE, state, nonce 요구 사항

> Start your callback listener and generate a fresh random `state`, OIDC `nonce`, and PKCE verifier for each attempt. Keep those values and the exact callback URI until the callback completes.

> `code_challenge`: The base64url-encoded SHA-256 digest of the PKCE verifier, without padding.

- PKCE 방식: `S256`만 지원(discovery `code_challenge_methods_supported: ["S256"]`).
- state: 시도별 새 난수, 콜백에서 검증 필수. `error=access_denied` 콜백도 state 검증 후 처리.
- nonce: ID 토큰 검증용. ID 토큰의 `nonce` 클레임과 비교.
- 참조 구현(DevKit) 동작: state, nonce, verifier 모두 32바이트 난수의 base64url 값. state 비교는 상수 시간 비교(`timingSafeEqual`). 로그인 전체 제한 시간 10분.

### 2.4 redirect URI 규칙

> Use an HTTP loopback callback on `127.0.0.1` from initial registration onward, for example `http://127.0.0.1:1455/auth/callback`. Start the listener before opening the browser. Later sign-ins may use another available port, such as `http://127.0.0.1:54321/auth/callback`; only the port may vary. Keep the scheme, host, and path unchanged: `/callback` does not match `/auth/callback`. Within each authorization attempt, send the exact same URI, including its selected port, in the authorization request and code exchange. Do not substitute with `localhost`.
>
> (출처: https://developers.openai.com/siwc/token-sharing-open-source/sign-in)

정리:

- 스킴: `http`
- 호스트: `127.0.0.1` 고정. `localhost` 대체 금지.
- 경로: `/auth/callback` 고정.
- 포트: 가변 허용. 한 시도 안에서는 인가 요청과 코드 교환에 동일 URI 사용.
- 고정 포트 필요 여부: 불필요. DevKit은 포트 `0`(가용 포트 자동 선택) 설정 허용, 예제 앱 기본값 8787.
- 사용자 지정 스킴(custom scheme): 문서 언급 없음 (미확인). 문서상 허용 형식은 HTTP 루프백뿐.
- DevKit 메시지 기준 등록 시점 콜백 호스트가 등록에 고정되는 동작 존재:

> This connection was registered with the older localhost callback. Add an account to register the new 127.0.0.1 callback. Your existing connection is preserved.
>
> (출처: https://github.com/openai/sign-in-with-chatgpt-devkit/blob/main/packages/local/src/index.ts)

### 2.5 scope 의미

| scope | 의미 (원문 근거) |
| --- | --- |
| `openid` | OpenID Connect ID 토큰 요청 |
| `profile` | 이름, 프로필 사진 등 프로필 클레임 |
| `email` | 이메일 및 이메일 검증 클레임 |
| `offline_access` | refresh token 발급 |
| `resource.invoke` | `chatgpt.tokens.use.direct`와 함께 사용자 플랜 기반 API 접근 요청 |
| `chatgpt.tokens.use.direct` | ChatGPT 플랜 사용 권한. 부여 여부로 플랜 사용 활성 판단 |

> These scopes apply to identity sign-in: `openid` Request an OpenID Connect identity token. `profile` Receive available profile claims, such as a name or profile picture. `email` Receive available email and email-verification claims.
>
> (출처: https://developers.openai.com/siwc/website)

> The request includes identity scopes to identify the account, `offline_access` to request a refresh token, and `resource.invoke` with `chatgpt.tokens.use.direct` to request API access using the user's ChatGPT plan. The resource is `https://api.openai.com/v1`.
>
> (출처: https://developers.openai.com/cookbook/articles/sign-in-with-chatgpt)

> Using a ChatGPT plan requires separate Responses API scopes that the user must authorize. It doesn't grant access to the user's API key or conversations.
>
> (출처: https://developers.openai.com/siwc/quickstart)

- `resource.invoke` 단독 의미 정의: 문서 내 별도 정의 없음 (미확인).

### 2.6 동의 화면

- 신규 등록: 로그인, 워크스페이스 선택, 에이전트 이름 확인(수정 가능), 권한 검토.

> The user signs in, selects a workspace, and reviews the requested permissions. OpenAI registers the client during this flow, so you don't need a client secret or an API key.
>
> (출처: https://developers.openai.com/cookbook/articles/sign-in-with-chatgpt)

- 사용자 측 설명: "Use your ChatGPT plan" 권한 별도 토글.

> When an app asks to use your plan, review the **Use your ChatGPT plan** permission and choose whether to allow it. Select **Continue** to proceed.
>
> (출처: https://learn.chatgpt.com/docs/sign-in-with-chatgpt)

- 재인가: 동의 화면 없음. `id_token_hint` 미포함 시 계정 선택 화면만 표시, 포함 시 계정 선택도 생략.

> **Expected returning sign-in:** For routine reauthorization with an issued dynamic-client ID, without `id_token_hint` the user sees the account selector screen, then redirects, with no workspace selection or consent screen. With `id_token_hint`, the flow skips account selection and redirects. The client stays bound to its registered workspace.

- 재동의(플랜 사용을 나중에 켜는 경우):

> Request consent with `force_reconsent=true` only after OpenAI confirms deployment for your integration; the existing OAuth `prompt=consent` mechanism remains supported before that rollout. Your app should not force consent on every ordinary sign-in.
>
> (출처: https://developers.openai.com/siwc/token-sharing-open-source/errors-and-recovery)

### 2.7 콜백 처리와 코드 교환

콜백 예시 원문:

```text
YOUR_CALLBACK
  ?code=<AUTHORIZATION_CODE>
  &scope=chatgpt.tokens.use.direct+email+offline_access+openid+profile+resource.invoke
  &state=<ORIGINAL_STATE>
  &client_id=<ISSUED_CLIENT_ID>
```

처리 규칙:

- `error=access_denied`: state 검증 후 시도 중단, 코드 교환 금지.
- 신규 등록 성공: `code`, `state`, 발급 `client_id` 수신.
- 재인가: 콜백에 `client_id` 생략 가능. 다른 `client_id`가 오면 결과 거부.

> If the callback supplies a different client ID, reject the result rather than replacing the selected account's registration. Use the token response's granted scopes when deciding whether ChatGPT plan usage is enabled.

코드 교환:

> **Exchange the code.** POST a form-encoded `authorization_code` grant to `https://auth.openai.com/api/accounts/oauth/token` with the issued `client_id`, `code`, `code_verifier`, the same `redirect_uri`, and the same `resource`. No client secret is required.

> If code exchange returns `invalid_grant`, discard that code and start a fresh authorization with the issued client ID retained for this attempt.

- DevKit 동작: 코드 교환 전에 발급 `client_id`를 먼저 저장. 코드 만료 시 재등록 없이 같은 `client_id`로 재시도하기 위한 처리.

### 2.8 ID 토큰 검증

> **Validate the ID token.** Verify its signature against OpenAI's published JWKS. Check the issuer, audience against the issued client ID, expiration, and the nonce saved for this attempt. Use its validated `sub` as the account identity. Email and `sub` are not workspace identifiers; retain the issued client ID with the verified identity to keep registrations separate.

> Check the token response's granted scopes for `chatgpt.tokens.use.direct` before proceeding to inference. A valid ID token alone does not authorize ChatGPT plan usage.

- 서명 알고리즘: RS256 (discovery 기준).
- DevKit 검증 항목: `iss`, `aud`, `exp`, `iat`, `sub` 필수, 시계 오차 허용 5초, `azp` 존재 시 client ID 일치 확인.

---

## 3. 헤드리스 및 SSH 환경

### 3.1 device code 흐름

- SIWC 문서 내 device code 흐름: 없음.
- discovery 문서: `device_authorization_endpoint` 없음, `grant_types_supported`는 `authorization_code`, `refresh_token` 2종.
- Codex CLI의 device code 로그인(`codex login --device-auth`, beta)은 Codex CLI 자체 로그인 문서에만 존재. 제3자 SIWC 클라이언트 적용 근거 없음.

> In these situations, prefer device code authentication (beta). In the interactive login UI, choose **Sign in with Device Code**, or run `codex login --device-auth` directly.
>
> (출처: https://learn.chatgpt.com/docs/auth, Codex CLI 대상)

### 3.2 공식 헤드리스 경로 (Self-hosted VMs 문서)

4단계 절차:

- (1) VM용 host ID 생성 및 저장
- (2) 브라우저가 있는 로컬 기기에서 같은 도구, 같은 사용자, 같은 워크스페이스로 OAuth 완료
- (3) 보호된 자격 증명 파일을 SSH 등 보안 채널로 VM에 전송, VM 고유 host ID 유지
- (4) 이후 갱신은 VM 담당

> A `127.0.0.1` callback reaches the computer running the browser, not the remote VM. Complete OAuth locally with the same tool/client for the same user and workspace you will use on the VM.

> Transfer the selected registration's protected credential file to the tool's documented VM path over a secure channel such as SSH. Include its issued `client_id`, `access_token`, `refresh_token`, and other saved metadata such as a retained `id_token`.

> Preserve the host ID assigned to the VM when importing the credentials so a copied laptop ID does not overwrite it.

> Let the VM own later refreshes. Use its own host ID for its next authorization.

> This procedure transfers an existing credential session. Host-specific usage attribution and revocation of ChatGPT plan access for transferred sessions are not yet available.
>
> (출처: https://developers.openai.com/siwc/token-sharing-open-source/self-hosted-vms)

- 주의: 전송 후 로컬 기기와 VM이 같은 refresh token을 동시에 쓰면 회전식 토큰 충돌 발생 가능. 문서의 "Let the VM own later refreshes" 지침에 따라 로컬 측 사본 폐기 또는 미사용 처리 필요.

### 3.3 비공식 대안 (SIWC 문서 미기재)

| 대안 | 상태 |
| --- | --- |
| SSH 로컬 포트 포워딩(`ssh -L <port>:127.0.0.1:<port>`) | Codex CLI 문서에 Codex 전용 대안으로만 기재. SIWC 클라이언트 적용 공식 근거 없음 (미확인) |
| 리다이렉트 URL 수동 붙여넣기 | SIWC 문서 언급 없음 (미확인) |
| 사용자 지정 스킴, 원격 콜백 | SIWC 문서 언급 없음. 문서상 허용 형식은 127.0.0.1 루프백뿐 |

### 3.4 원격 실행 관련 약관 조항

> **User control.** Requests must originate from the user's local runtime or a remote runtime only that user controls.

> Any persistent storage of Authentication Tokens must be local and under the user's control, not in a remote or managed environment.
>
> (출처: https://openai.com/policies/sign-in-with-chatgpt-terms/)

- 해석: 사용자 단독 통제 VM은 개발자 문서에서 명시 지원. 관리형(managed) 원격 환경의 토큰 저장 금지. 경계 사례(공유 서버, 회사 관리 서버) 판단 기준은 미확인.

---

## 4. 사용 자격

### 4.1 플랜

| 출처 | 원문 |
| --- | --- |
| 개발자 quickstart | "Eligible ChatGPT Plus and Pro users can use their ChatGPT plan for AI requests in participating apps" |
| 쿡북 | "Eligible ChatGPT Plus and Pro users can try your tool with the account and plan they already have" |
| ChatGPT 릴리스 노트 (2026-09-29) | "Eligible Plus and Pro users can authorize participating apps to use their ChatGPT model allowance without creating an API key." |
| learn.chatgpt.com | "eligible ChatGPT Plus and Pro subscribers can also choose to use their ChatGPT plan for AI requests" |
| 도움말 센터 20001542 | "the option to use your ChatGPT plan in participating commercial apps and sites is only available with Go, Plus, and Pro" |
| 도움말 센터 20001542 | "Supported open-source tools remain available to all ChatGPT users." |

- 출처 간 불일치: 개발자 문서는 Plus, Pro만 명시. 도움말 센터는 상용 앱에 Go 포함, 오픈소스 도구는 "모든 ChatGPT 사용자" 표기. "Supported open-source tools"가 파트너 목록 등재 도구만 지칭하는지, 동적 등록한 모든 오픈소스 도구를 지칭하는지 미확인.
- Free 플랜: 오픈소스 도구 경로 사용 가능 여부 미확인. 확인 방법: Free 계정으로 실제 로그인 후 `chatgpt.tokens.use.direct` 부여 여부와 추론 성공 여부 측정, 또는 OpenAI 지원 문의.
- Business, Enterprise, Edu: 명시적 허용 또는 불허 문구 없음 (미확인). 관련 사실:
  - 등록 시 워크스페이스 선택 단계 존재, `client_id`는 워크스페이스에 고정.
  - 오류 코드 `subscription_sharing_user_not_eligible` 설명: "ChatGPT plan usage is unavailable for the selected user, workspace, or policy."
  - 신원 로그인은 조직 관리자 설정 영향: "Availability for members of an organization also depends on the organization's admin settings." (도움말 센터 20001410)

### 4.2 사용량 산정

> When you turn this on, eligible AI requests count toward the ChatGPT Work and Codex usage included in your plan. You can set a weekly usage limit for each app and review app usage in ChatGPT settings.
>
> (출처: https://help.openai.com/en/articles/20001542)

> For ChatGPT Plus users, the five-hour usage limit is shared across all apps where they use their ChatGPT plan, including private and open-source clients. Usage in one app contributes to the same five-hour usage total, and no app receives a separate allowance. The five-hour usage limit does not apply for Pro users.
>
> (출처: https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions)

- 앱별 주간 한도: 사용자가 ChatGPT 설정에서 전체 주간 사용량 대비 10-100% 범위로 지정 (문서 삽화 설명 기준).
- 크레딧: 플랜 한도 소진 후 사용자 명시 동의 시에만 사용, 기본값 꺼짐, 앱 한도 100% 설정 필요.

### 4.3 지역

- 지역 제한 존재 근거: 403 설명의 "permitted serving region".

> `403`: A policy or permission check, such as the permitted serving region, prevented admission. Surface the restriction and verify the integration.

- 허용 지역 목록: 미확인. 한국 사용자 허용 여부도 미확인. 확인 방법: 한국 소재 Plus 계정으로 실제 추론 요청 시험.
- 신원 로그인 범위: "Sign in with ChatGPT is available globally to authenticated ChatGPT users, including users in Enterprise organizations." (도움말 센터 20001410, 신원 로그인 한정)

### 4.4 연령

- SIWC 전용 연령 제한: 문서 언급 없음 (미확인).
- OpenAI 이용약관 일반 조항:

> You must be at least 13 years old or the minimum age required in your country to consent to use the Services. If you are under 18 you must have your parent or legal guardian's permission to use the Services.
>
> (출처: https://openai.com/policies/row-terms-of-use/)

---

## 5. 토큰으로 허용되는 API 범위

### 5.1 모델 목록

> Before showing a model picker, request the selected ChatGPT account's model catalog with the same access token you will use for inference:

```bash
curl --fail-with-body --silent --show-error \
  https://api.openai.com/v1/models \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  | jq '[.models[] | select(.visibility == "list") | {slug, display_name}]'
```

> The model-list response contains a `models` array. This example keeps models intended for display (`visibility: "list"`) and preserves the server's ordering. Show `display_name` in your UI and pass the selected `slug` as `model` in the next step. Refresh these choices when the user switches ChatGPT accounts.
>
> (출처: https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference)

- 응답 형식: 일반 API의 `data[]`가 아닌 `models[]` 배열, 필드 `slug`, `display_name`, `visibility`.
- 모델 목록: 계정별 가변. 문서 예시 모델 `gpt-6.1-sol`. 고정 허용 목록 문서 없음.

### 5.2 Responses API 요청 규칙

> To perform inference on behalf of the user, send the OAuth access token as `Authorization: Bearer <ACCESS_TOKEN>` to `POST https://api.openai.com/v1/responses`.

> **Supported endpoint:** Use the public Responses API endpoint above for this ChatGPT plan usage flow; do not point it at ChatGPT's `backend-api` endpoints. Use a model available to the signed-in account.

> **Set `store` to `false` and `stream` to `true` on each HTTP inference request in this flow.** Consume the response as a stream of events.

> Treat inference as successful only after receiving `response.completed`. Read through the terminal event. A usage-limit error can arrive as `response.failed` with `subscription_sharing_usage_limit_exceeded` or `subscription_sharing_usage_unavailable` after streaming has begun.

curl 예시 원문:

```bash
curl --no-buffer https://api.openai.com/v1/responses \
  -H "Authorization: Bearer ${ACCESS_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-6.1-sol",
    "input": [
      {
        "role": "user",
        "content": "Say exactly: Hello, world!"
      }
    ],
    "store": false,
    "stream": true
  }'
```

### 5.3 프리뷰 제약 (Preview limitations 원문)

> **HTTP requests:** Set `store: false` and `stream: true`. Send `input` as an array containing the context needed for each request. Use `instructions` or developer messages; explicit `{type: "message", role: "system"}` items are rejected.

> **Unsupported fields:** Omit `background`, `conversation`, `max_output_tokens`, `max_tool_calls`, `metadata`, `moderation`, `multi_agent`, `prompt`, `prompt_cache_retention`, `safety_identifier`, `temperature`, `top_logprobs`, `top_p`, `truncation`, and `user`.

> **Conversation state:** Omit `previous_response_id` over HTTP and send the required history in `input`. WebSocket continuation can reference only responses from the same authenticated connection; it does not provide persistent conversation storage.

> **Supported tools:** Group function/custom tools in namespaces or supply them through `additional_tools` input items. Web search remains subject to model and account/workspace policy.

> **Unsupported tools:** Image generation, file search, Code Interpreter, native computer use, hosted MCP/connectors, and Responses `tool_search`. Client-side execution does not make `tool_search` supported. `programmatic_tool_calling` is not accepted in top-level `tools` on this route.

> **Inputs:** Text, images, and files are supported when the selected model accepts them. Audio/video input, the Files upload API, and the transcription API are not supported by this flow.
>
> (출처: https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations)

howtui 관점 기능별 정리:

| 기능 | 상태 |
| --- | --- |
| 스트리밍 | 필수(`stream: true`) |
| 비스트리밍 응답 | 불가 (HTTP 요청 기준 `stream: true` 필수) |
| `store` | `false` 필수 |
| system 역할 메시지 | 거부. `instructions` 또는 developer 메시지 사용 |
| `temperature`, `top_p` | 미지원 |
| `max_output_tokens` | 미지원. 출력 길이는 지시문으로만 제어 |
| `previous_response_id` | HTTP에서 미지원. 대화 이력은 `input`에 직접 포함 |
| 함수 도구(function calling) | namespace로 묶거나 `additional_tools` 입력 항목으로 제공 시 지원 |
| 구조화 출력(`text.format`, JSON schema) | 지원 또는 미지원 명시 없음 (미확인) |
| `reasoning` 파라미터 | 지원 또는 미지원 명시 없음 (미확인) |
| 웹 검색 | 모델 및 계정/워크스페이스 정책에 종속 |
| WebSocket 모드 | 언급 존재(같은 연결 내 continuation). HTTP 외 사용 조건 상세 미확인 |
| 다른 엔드포인트(Chat Completions 등) | `subscription_sharing_route_not_supported` 대상. 문서상 지원 경로는 `POST /v1/responses`와 모델 목록 |

- namespace 도구 정의 형식: 일반 API 문서의 `{"type": "namespace", "name": ..., "description": ..., "tools": [{"type": "function", ...}]}` 구조 (https://developers.openai.com/api/docs/guides/tools-tool-search). SIWC 경로에서의 실제 동작은 미확인.

### 5.4 요청 빈도 한도

- 플랜 단위 한도(5시간, 주간, 앱별 주간 한도): 4.2절 참조.
- RPM, TPM 같은 요청 빈도 한도: 문서 언급 없음 (미확인).
- `Retry-After` 헤더 제공 여부: 미확인.

---

## 6. host ID 개념과 관리

### 6.1 정의

> An agent host is the environment where an instance of your tool runs, such as a laptop installation or a self-hosted VM. Your app defines what counts as one host and when its lifecycle begins and ends. Restarting the tool or signing in again does not by itself create a new host.

> **Host-ID:** Generate and persist a stable `ext_agent_host_id` for each host separately. Choose and persist the host ID before its first sign-in. Use an opaque value, not an email, user ID, or other user-identifying value. A host ID is not an authentication credential.
>
> (출처: https://developers.openai.com/siwc/token-sharing-open-source)

### 6.2 허용 형식

| 형식 | 원문 지침 |
| --- | --- |
| `urn:ietf:params:oauth:jwk-thumbprint:…` | 권장. RFC 9278 기반 공개키 썸프린트 URI |
| `urn:uuid:…` | 지원. UUIDv4를 1회 생성 후 호스트 수명 동안 유지 |
| `did:key:<key>` | `did:key`만 허용, 다른 DID 방식 불가 |

> A public-key-derived host ID is currently an identifier only. OpenAI does not verify possession of the private key as part of this flow.

> For a UUID-based host ID, generate a UUIDv4 once per host, prefix its hyphenated string form with `urn:uuid:`, and persist the value for reuse across sign-ins.

### 6.3 클라이언트 관리 규칙

- (a) 첫 로그인 전 생성 및 저장. 로그인 취소 시에도 유지.
- (b) 같은 호스트의 모든 로그인과 모든 계정에 같은 값 전송.
- (c) 계정 전환, 로그아웃, 재시작 시 재생성 금지.
- (d) VM 이전 시 VM 고유 값 유지, 노트북 값으로 덮어쓰기 금지.
- (e) 비밀 값 아님. DevKit은 `chatgpt-host.json`에 암호화 없이 저장.

> The installation's host ID stays the same across profiles.
>
> (출처: https://developers.openai.com/cookbook/articles/sign-in-with-chatgpt)

### 6.4 문서와 DevKit 간 불일치

- 개발자 문서: `ext_agent_host_id`를 "Required"로 표기.
- DevKit `sendHostId` 옵션: 기본값 `false`, 주석 "Enable only when the authorization provider supports ext_agent_host_id."
- 쿡북: `sendHostId: true` 설정 지시, "the complete example already includes this option" 표기. 실제 예제 `examples/paste-perfect/electron/main.ts`(2026-10-07 커밋 기준)에는 해당 옵션 없음.
- howtui 권장: 개발자 문서 기준으로 항상 전송.

---

## 7. 토큰 수명, 갱신, 폐기, 연결 해제

### 7.1 토큰 응답 필드와 수명

> The token response includes `access_token`, `refresh_token`, `id_token`, `token_type`, `expires_in`, `scope`, and `earliest_refresh_at`. A successful code exchange returns HTTP `200`. A successful `grant_type=refresh_token` exchange also returns HTTP `200`, with a new access token and a replacement refresh token.

> Access tokens are valid for one hour (`expires_in: 3600`).

> Refresh tokens are valid for 30 days. Each successful refresh returns a replacement refresh token with a fresh 30-day lifetime. Successive replacements have no fixed limit while each token remains valid.
>
> (출처: https://developers.openai.com/siwc/token-sharing-open-source/token-reference)

- `earliest_refresh_at`: 문서에 필드명만 존재, 의미 정의 없음 (미확인). DevKit 동작: 만료 60초 전부터 갱신 시도, 단 `earliest_refresh_at` 이전에는 갱신 보류.
- access token 클레임: `sub`, `aud`(`https://api.openai.com/v1`), `client_id`, `scope`, `https://api.openai.com/auth`(불투명 메타데이터), `iss`, `iat`, `exp`, `jti`, `nbf`. 앱 측 해석 불필요.

### 7.2 갱신

> Use standard OAuth refresh near access-token expiry. POST form-encoded `grant_type=refresh_token`, the issued `client_id` saved with this token set (not `dynamic_agent_client`), the saved `refresh_token`, and `resource=https://api.openai.com/v1` to `https://auth.openai.com/api/accounts/oauth/token`; omit `scope` to retain the grant.

> Store and use the latest replacement, and serialize refreshes for the same session so two processes do not race a rotating token.
>
> (출처: https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions)

- 회전(rotation): 갱신마다 새 refresh token 발급. 이전 토큰 재사용 시 `refresh_token_reused` 오류 가능.
- 저장 규칙: access token, 만료 시각, 부여 scope, 새 refresh token 일괄 교체.
- howtui 영향: CLI 동시 실행 시 프로세스 간 파일 잠금 필수.

### 7.3 로그아웃과 폐기

> When signing out, revoke the renewable session using the `revocation_endpoint` from `https://auth.openai.com/.well-known/openid-configuration`. Send a form-encoded `POST` with `token=<REFRESH_TOKEN>`, `token_type_hint=refresh_token`, and the issued `client_id`. An empty HTTP `200` is the revocation success response, including for an already-invalid token. Stop using the tokens and clear them locally after revocation.

> For a network failure or `5xx`, retry with backoff while the refresh token is still available. If you finish signing out locally without confirming revocation, clear the local tokens and tell the user that remote revocation was not confirmed. The user can disconnect the app in ChatGPT Settings. Revoking this session does not delete the registered client.

- 폐기 엔드포인트 실측값: `https://auth.openai.com/api/accounts/oauth/revoke`.
- 로그아웃 후 보존 항목: 계정과 client 매핑, host ID. 삭제 항목: access token, refresh token, ID token.
- 앱 로그아웃만으로는 플랜 사용 승인 해제 안 됨:

> Signing out of the app ends your session there, but does not disconnect it from ChatGPT or stop its authorized use of your plan.
>
> (출처: https://help.openai.com/en/articles/20001542)

### 7.4 사용자 측 연결 해제

> To completely disconnect access for an external partner tool, go to **ChatGPT Settings > Security and login > Sign in with ChatGPT**, select the app, and select **Disconnect**.
>
> (출처: https://learn.chatgpt.com/docs/sign-in-with-chatgpt)

> OpenAI does not currently notify your tool when a user disconnects the app in ChatGPT settings. When a request or refresh confirms the disconnection, stop using the invalid token set and ask the user to sign in again. Do not erase credentials solely because of a temporary network or infrastructure failure.
>
> (출처: https://developers.openai.com/siwc/token-sharing-open-source/errors-and-recovery)

---

## 8. 오류 응답 구분

### 8.1 일반 원칙

> ChatGPT plan usage errors stop inference. OpenAI does not silently switch the request to another billing path. Preserve the actual HTTP status, response body shape, error code when present, and request ID.

> Do not assume every failure has the standard API `error` object. Direct-route admission can return a body such as `{"detail":"..."}` before a Responses request starts. Treat the message as diagnostic text, not a stable machine-readable code.

### 8.2 스트림 시작 전 진입(admission) 오류

| 상태 | 원문 복구 지침 |
| --- | --- |
| `401` | The required signed identity or direct permission was not accepted. Check the selected ChatGPT account and granted scopes. |
| `403` | A policy or permission check, such as the permitted serving region, prevented admission. Surface the restriction and verify the integration. |
| `503` | Direct routing is unavailable or not enabled. Preserve credentials and use bounded backoff for temporary failures. |

### 8.3 Responses 계층 구조화 오류 (`error.code`)

| 코드 | HTTP | 의미와 조치 (원문 요지) |
| --- | --- | --- |
| `subscription_sharing_user_not_eligible` | 403 | 사용자, 워크스페이스, 정책상 플랜 사용 불가. 동일 요청 반복과 OAuth 반복 금지 |
| `subscription_sharing_usage_limit_exceeded` | 429 | 플랜 사용 요청 중지, ChatGPT 설정 Usage 링크 안내. 리셋 시각 추정 금지, 앱별 한도 가능성 |
| `subscription_sharing_usage_unavailable` | 503 | 사용량 확인 불가. 자격 증명 유지, 제한된 백오프 재시도 |
| `subscription_sharing_unsupported_capability` | 400 | `error.param` 확인 후 미지원 입력, 도구, 실행 기능, 모델, service tier 제거. 동일 본문 재시도 금지 |
| `subscription_sharing_route_not_supported` | 403 | HTTP 메서드와 엔드포인트 확인. 지원 경로 `POST /v1/responses` |
| `subscription_sharing_invalid_user` | 401 | 구독자 컨텍스트 검증 실패. request ID 보존, 폐기 확인 또는 갱신 종결 오류 후 재로그인 |
| `chatpass_v2_scope_not_authorized`, `chatpass_v2_invalid_authorization_context` | 403 | 서명된 권한 컨텍스트 불일치. client 및 grant 구성 점검, 재시도 금지 |
| `subscription_sharing_user_unavailable` | 503 | 사용자 또는 워크스페이스 정보 일시 불가. 자격 증명 유지, 백오프 재시도 |

> The following codes have specific recovery actions; they are not an exhaustive list of authentication, routing, or transport failures.
>
> (출처: https://developers.openai.com/siwc/token-sharing-open-source/errors-and-recovery)

### 8.4 갱신 오류

> **Unusable refresh token:** `invalid_grant` on refresh, `invalid_refresh_token`, `token_expired`, `refresh_token_expired`, `refresh_token_invalidated`, or `refresh_token_reused`. Clear unusable tokens and repeat OAuth with the saved issued client ID.
>
> **Invalid client:** `invalid_client`. Fix the client configuration.

### 8.5 OAuth 단계 오류

- 콜백 `error=access_denied`: 동의 거부. 코드 교환 금지.
- 코드 교환 `invalid_grant`: 코드 폐기 후 같은 발급 client ID로 새 인가.
- 토큰 응답에 `chatgpt.tokens.use.direct` 없음: 로그인 유지, 플랜 사용 비활성 처리, 다른 과금 경로(사용자 API 키 등) 선택지 제공.

> A coding harness still needs a way to pay for inference, so offer a clear choice: enable ChatGPT plan usage or configure another supported billing path, such as the user's own API key.

### 8.6 howtui용 상태 구분 매핑

| howtui 상태 | 판별 근거 |
| --- | --- |
| 인증 실패 | 진입 401, `subscription_sharing_invalid_user`(401), 갱신 종결 오류 6종 |
| 플랜 비대상 | `subscription_sharing_user_not_eligible`(403), 토큰 응답 scope에 `chatgpt.tokens.use.direct` 부재 |
| 권한 구성 오류 | `chatpass_v2_*`(403), `invalid_client` |
| 지역 또는 정책 차단 | 진입 403(`{"detail": ...}` 본문) |
| 사용량 한도 초과 | `subscription_sharing_usage_limit_exceeded`(429, 스트림 중 `response.failed`로도 도착) |
| 모델 또는 기능 미지원 | `subscription_sharing_unsupported_capability`(400, `error.param`), DevKit 매핑상 `model_not_found` |
| 경로 미지원 | `subscription_sharing_route_not_supported`(403) |
| 일시 장애 | 진입 503, `subscription_sharing_usage_unavailable`(503), `subscription_sharing_user_unavailable`(503), 5xx |
| 네트워크 오류 | 연결 실패, 타임아웃, `response.completed` 없이 스트림 종료. 자격 증명 유지 |
| 불완전 응답 | `response.incomplete` 이벤트 |

- 코드 없는 일반 429: 문서 처리 지침 없음 (미확인). DevKit은 "Too many requests" 재시도 가능 오류로 분류.
- DevKit 전용 코드(공식 문서 미기재): `subscription_sharing_v2_client_not_enabled`("This app is not enabled for token sharing. Contact the app developer."), `subscription_sharing_v2_*` 구버전 코드, `model_not_found`, `invalid_token`, `invalid_api_key`.
- request ID 헤더: DevKit은 `x-request-id`, 웹사이트 문서 예시는 `openai-request-id` 또는 `x-request-id` 사용.

---

## 9. 보안 요구 사항, 약관, 브랜딩

### 9.1 클라이언트 유형

> this OSS flow remains a public client without a secret.
>
> (출처: https://developers.openai.com/siwc/token-sharing-open-source/sign-in)

### 9.2 토큰 저장

개발자 문서 기준:

> Store credential files under an app-defined path such as `~/.config/YOUR_TOOL/`. Write files atomically with owner-only permissions (`0600` on Unix), and never commit or log them. Replace the access token, expiry, granted scopes, and rotating refresh token together after a successful refresh.

> Keep access, refresh, and retained ID tokens in protected local or self-hosted runtime storage. Keep tokens out of browser storage, source control, logs, analytics, and support transcripts.

> Never put access or refresh tokens in URLs. Send an ID token as `id_token_hint` only to the OpenAI authorization endpoint, and redact authorization URLs containing that hint from logs and diagnostics.

DevKit 기준(더 엄격):

> The local SDK requires an application-supplied credential encryption provider. There is no plaintext fallback. Paste Perfect supplies Electron's main-process `safeStorage` implementation after Electron is ready. On macOS, `safeStorage` protects its encryption key with Keychain; the encrypted connection state remains in the app's local data directory.
>
> (출처: https://github.com/openai/sign-in-with-chatgpt-devkit/blob/main/docs/security.md)

- OS 키체인 사용 의무 여부: 개발자 문서상 의무 아님(0600 파일 허용). DevKit은 OS 기반 암호화 필수, 평문 금지.
- 저장 레코드 필드(문서 예시): `email`, `issuer`, `subject`, `client_id`, `ext_agent_host_id`, `id_token`, `access_token`, `refresh_token`, `token_type`, `expires_in`, `scopes`, `saved_at`.

### 9.3 SIWC 약관 주요 조항

> Use your app's own name during sign-in and activation. Do not impersonate OpenAI, another application, or another open-source project.

> Obtain and use SIWC access and refresh tokens ("Authentication Tokens") only through OpenAI's supported sign-in flow and as authorized by the user. Do not ask users for their ChatGPT passwords or session cookies.

> **Connected application only.** Use the user's plan only for the application they connected. Do not provide general-purpose API access for other tools or unrelated requests.

> **User authorization.** Requests must be for the authenticated user and arise from their activity or expressly authorized automations or background processes. Obtain express consent before background use. Another user's activity must not trigger requests to the authenticated user's account.

> You must not enable or encourage: Creating multiple accounts, splitting usage, rotating accounts, or otherwise bypassing usage limits. Pooling, transferring, reselling, gifting, or sharing ChatGPT plan usage or Authentication Tokens, except for token handling expressly permitted by these Terms. Using one user's subscription to fulfill another user's requests.

> Your SIWC integration and use must comply with applicable law, these Terms, [the applicable license](https://github.com/openai/sign-in-with-chatgpt-devkit/blob/main/LICENSE), and our Usage Policies. You may not: ... Alter, reverse engineer, or otherwise amend the SIWC software. Sublicense SIWC or any underlying technology.

> We may suspend or disable your application's access to SIWC if it violates these Terms or presents a security, safety, or abuse risk.
>
> (출처: https://openai.com/policies/sign-in-with-chatgpt-terms/)

- 약관 결합 구조: SIWC 약관은 Terms of Use와 Service Terms를 포함. Service Terms 15절에서 SIWC 사용 서비스에 SIWC 약관 적용 명시.
- 개인정보 조항: 사용자에게 처리 전 개인정보 처리 고지 제시 의무, 필요 범위 외 개인정보 수집 금지.

### 9.4 금지 또는 비지원 행위 정리

| 행위 | 근거 |
| --- | --- |
| ChatGPT `backend-api` 엔드포인트 사용 | "do not point it at ChatGPT's `backend-api` endpoints" |
| 다른 클라이언트 유형의 경로 차용 | "support for another client type or route is not permission to use it here" |
| ChatGPT 비밀번호, 세션 쿠키 요청 | SIWC 약관 1절 |
| 다른 앱 또는 OpenAI 사칭 | SIWC 약관 1절 |
| 범용 API 프록시 제공 | SIWC 약관 2절 "Connected application only" |
| 토큰 공유, 양도, 계정 회전 | SIWC 약관 4절 |
| Codex CLI 인증 파일(`~/.codex/auth.json`) 읽기, Codex client ID 사용 | SIWC 문서 내 직접 금지 문구 미발견. 다만 "supported sign-in flow" 한정 조항, 사칭 금지 조항, 토큰 양도 금지 조항과 충돌 소지. 문서화된 SIWC 경로 외부 행위로 판단, 사용 금지 권장 |

### 9.5 브랜딩과 UI 요구 사항

- 버튼 문구: **Continue with ChatGPT**.

> When the user first opens your tool, show a Sign in with ChatGPT button labeled **Continue with ChatGPT**, following the UI/UX Guidelines for approved OpenAI branding.

- 승인된 버튼 형식 4종: "Continue with ChatGPT on black or white, and Sign in with ChatGPT on black or white. Each button includes the ChatGPT logo." (https://developers.openai.com/siwc/website)
- 첫 로그인 확인 모달: **You're using your ChatGPT plan** 문구와 **Got it** 동작, 최초 1회만 표시.
- 사용 중 표시: 입력창 또는 모델 선택기 근처 **Using ChatGPT plan** 표시와 **Manage usage** 링크(`https://chatgpt.com/settings/usage`).
- 한도 도달 시: **Manage usage**를 주 동작으로 제시.
- 상표: OpenAI 이름, 로고, 버튼은 허가 범위 내 사용, 후원 또는 보증 암시 금지(SIWC 약관 5절, https://openai.com/brand/).
- CLI/TUI 전용 지침: 문서 없음 (미확인). 로고 표시 불가 환경의 텍스트 전용 버튼 허용 여부 미확인.

---

## 10. 공식 SDK와 참조 구현

### 10.1 Sign in with ChatGPT DevKit

- 저장소: https://github.com/openai/sign-in-with-chatgpt-devkit (2026-09-29 생성, 최종 푸시 2026-10-07)
- 언어: TypeScript, JavaScript (Node.js 22.12 이상)
- 구성:

| 경로 | 내용 |
| --- | --- |
| `packages/local` (`@siwc/local`) | OAuth, 자격 증명 저장, 계정 프로필, 모델 조회, Responses 스트리밍 |
| `packages/react` (`@siwc/react`) | 로그인 버튼, 연결 상태, 사용량 UI 컴포넌트 |
| `assets` | ChatGPT 마크, 아이콘, 로그인 버튼 SVG, 글꼴 |
| `examples/paste-perfect` | macOS 데스크톱 예제(Electron + Swift) |

- 배포 형태: "Both packages are local workspaces in this repository" (README). npm 공개 배포 여부 미확인.
- 외부 기여: "This repository does not accept external pull requests or code contributions."
- 라이선스: "Sign-in with ChatGPT DevKit Noncommercial License v1.0". 핵심 조항:

> "Noncommercial Purpose" means personal learning, experimentation, or development without an intended or reasonably anticipated commercial application.

> "Modified Work" means a work containing modifications to the Work. Independently authored software does not become a Modified Work merely by calling, linking to, or communicating with the Work through an interface.

> You may distribute the Work or a Modified Work only for a Noncommercial Purpose ... license every modification You own or have authority to license in a distributed Modified Work under this same License

> This License grants no rights to OpenAI names, trademarks, logos, or branded visual assets ... It grants no access to ChatGPT, the OpenAI API, or any other OpenAI service.
>
> (출처: https://github.com/openai/sign-in-with-chatgpt-devkit/blob/main/LICENSE)

- howtui 영향: DevKit 코드의 복사 또는 번역 이식 시 Modified Work 해당 가능성. 허용적 라이선스(MIT, Apache-2.0) 유지 목적이면 공개 문서 기반 독자 구현 필요.

### 10.2 기타 공식 예시

- 쿡북: https://developers.openai.com/cookbook/articles/sign-in-with-chatgpt (Paste Perfect 통합 절차)
- Python 예시: 공식 `openai` Python SDK의 `api_key`에 OAuth access token 전달, `base_url="https://api.openai.com/v1"`, `max_retries=0`.
- Codex app-server 경유 방식: `codex app-server`에 `model_providers.<id>.env_key="ACCESS_TOKEN"`, `wire_api="responses"`, `requires_openai_auth=false`, `supports_websockets=false` 설정. 토큰 갱신은 앱 책임, 갱신 시 app-server 재시작 필요. Codex 설치 전제.

### 10.3 Rust 구현

- OpenAI 공식 Rust 구현: 없음. GitHub 코드 검색(`dynamic_agent_client`, `ext_agent_host_id`, `chatgpt.tokens.use.direct`, owner `openai`) 결과 DevKit 저장소만 검출.
- 제3자 Rust 구현 존재(예: `TabularisDB/tabularis`의 `src-tauri/src/chatgpt/`, `jlcodes99/cockpit-tools`의 `src-tauri/src/modules/chatgpt_plan_oauth.rs`). 공식 아님, 정확성 미검증.
- 제3자 구현의 문서 불일치 사례: `openclaw/openclaw`의 `extensions/openai/token-sharing.ts`는 `http://localhost:8080/auth/callback` 사용(문서의 `localhost` 금지 지침과 상이). 제3자 코드 참고 시 공식 문서 우선 필요.

---

## 11. 프리뷰 상태와 출시 시점

| 일자 | 내용 | 출처 |
| --- | --- | --- |
| 2026-07-29 | 신원 로그인(Sign in with ChatGPT) 단계적 출시, Airtable, GitLab, HubSpot, Notion, Supabase, Vercel 대상 | ChatGPT 릴리스 노트 |
| 2026-09-29 | "Use your ChatGPT allowance in participating apps" 출시, "This is a limited preview with participating integrations." | ChatGPT 릴리스 노트 |
| 2026-09-29 | DevKit 저장소 생성 | GitHub API |
| 2026-10-22 | 개발자 문서에 "Preview limitations" 페이지 유지 | developers.openai.com |

- 상용 파트너용 신원 로그인: "limited trial", 대기자 명단.
- 오픈소스 플랜 사용: "available to all open-source partners and selected private clients".
- `force_reconsent=true`: "only after OpenAI confirms deployment for your integration" 조건부 기능.
- 미출시 기능: 전송 세션의 호스트별 사용량 귀속과 플랜 접근 폐기("not yet available").
- 정식 출시(GA) 일정: 미확인.

---

## 12. howtui 적용 실현 가능성

### 12.1 판정

- 판정: 조건부 실현 가능.
- 근거:
  - (a) 오픈소스 로컬 도구 대상 경로가 공식 문서화됨.
  - (b) 개발자 측 등록, client secret, 파트너 API 키 불필요. 사용자 로그인 시 동적 등록.
  - (c) 표준 OAuth 2.0 Authorization Code + PKCE(S256) + OIDC 구성, Rust 생태계로 구현 가능한 범위.
  - (d) 추론 경로가 공개 Responses API(`POST /v1/responses`)로 고정, ChatGPT 내부 API 불필요.

### 12.2 차단 요소와 위험

| 구분 | 내용 | 영향 |
| --- | --- | --- |
| 헤드리스 | device code 흐름 없음. 공식 대안은 로컬 OAuth 후 자격 증명 파일 전송뿐 | SSH 전용 서버 사용자의 첫 로그인 경험 저하 |
| 사용 자격 불일치 | 개발자 문서 Plus, Pro 한정. 도움말 센터는 Go 포함 및 오픈소스 도구 "모든 사용자" 표기 | Free, Go, Business, Enterprise, Edu 사용자 지원 범위 불확정 |
| 프리뷰 제약 | `temperature`, `max_output_tokens` 미지원, `stream: true`와 `store: false` 필수, 구조화 출력 지원 미확인 | 명령 생성 결과 형식 제어 수단 제한. 지시문 기반 형식 강제 또는 namespace 함수 도구 활용 필요 |
| 프리뷰 변동성 | limited preview 상태, 문서 내 롤아웃 조건부 기능 존재 | 요청 규칙, 오류 코드 변경 가능성 |
| 약관의 라이선스 참조 | SIWC 약관이 "the applicable license"로 DevKit 비상업 라이선스 링크 | DevKit 코드 미사용 시 적용 범위 불명확. 법적 해석 필요 |
| "SIWC software" 개조 금지 조항 | "Alter, reverse engineer, or otherwise amend the SIWC software" | 공개 문서 기반 독자 구현의 해당 여부 미확인 |
| 클라이언트 활성화 여부 | DevKit에 `subscription_sharing_v2_client_not_enabled` 처리 코드 존재, 문서의 503 "not enabled" 표현 | 신규 동적 등록 앱의 자동 활성화 여부 실측 필요 |
| 지역 제한 | 403 "permitted serving region" 존재, 목록 미공개 | 한국 사용자 사용 가능 여부 실측 필요 |
| 동시 실행 | 회전식 refresh token, 갱신 직렬화 요구 | CLI 다중 실행 시 파일 잠금 필수 |
| 연결 해제 통지 부재 | OpenAI가 앱에 연결 해제 통지 안 함 | 401 계열 응답 기반 재로그인 유도 필요 |

### 12.3 프로젝트 소유자 조치 항목

- (1) OpenAI 측 앱 등록: 불필요. 다만 howtui가 무료 오픈소스 로컬 도구 요건(유료화 없음, 원격 호스팅 없음)을 유지해야 하며, 유료 기능 또는 호스팅 서비스 추가 시 관심 양식 제출 필요.
- (2) 실측 검증용 계정 준비: ChatGPT Plus 또는 Pro 계정 1개 이상(가능하면 Free, Go 계정 추가)으로 등록, 모델 목록, 스트리밍 추론, 갱신, 폐기 전 과정 시험.
- (3) 약관 검토: SIWC 약관, Terms of Use, Service Terms, DevKit 라이선스 참조 조항의 howtui 적용 범위 판단. howtui 라이선스 선택과 연동.
- (4) `agent_name_hint` 고정값 결정: 예 `howtui`. 모든 설치에서 동일 값 사용.
- (5) 대체 과금 경로 결정: 플랜 비대상 사용자용 API 키 입력 경로 제공 여부 결정(문서 권장 사항).
- (6) 브랜딩 결정: TUI 텍스트 버튼 "Continue with ChatGPT" 사용, 로고 사용 여부는 OpenAI 브랜드 지침 범위 내 결정.
- (7) 개인정보 처리 고지 작성: SIWC 약관 3절의 사전 고지 의무 대응(이메일, 이름 등 수신 정보 처리 범위 명시).

### 12.4 구현 권장 사항 (공식 문서 기반 설계 지침)

- (a) 호스트 준비: 최초 실행 시 `urn:uuid:<UUIDv4>` 생성 후 비밀 아닌 설정 파일에 저장, 재생성 금지.
- (b) 콜백 리스너: `127.0.0.1`에 바인딩, 경로 `/auth/callback` 고정, 포트 0(자동 선택) 또는 설정값, 리스너 기동 후 브라우저 실행, Host 헤더와 경로 검사, state 상수 시간 비교.
- (c) 인가 요청: 최초 `client_id=dynamic_agent_client` + `agent_name_hint` + `ext_agent_host_id`, 재인가 시 발급 `client_id` + `ext_agent_host_id` + 선택적 `id_token_hint`/`login_hint`.
- (d) 코드 교환 전 발급 `client_id` 선저장, 교환 후 ID 토큰 RS256 서명과 `iss`, `aud`, `exp`, `nonce` 검증, scope에 `chatgpt.tokens.use.direct` 포함 여부 확인.
- (e) 자격 증명 저장: 계정별 레코드 분리, OS 키체인 우선, 불가 시 0600 권한 원자적 쓰기 파일. 로그와 진단 출력에서 토큰과 `id_token_hint` URL 제거.
- (f) 갱신: 만료 임박 시 갱신, `earliest_refresh_at` 존중, 프로세스 간 잠금으로 직렬화, 응답 수신 즉시 새 refresh token 저장.
- (g) 추론: `GET /v1/models`로 `visibility == "list"` 모델 표시, `POST /v1/responses`에 `store: false`, `stream: true`, `instructions` 사용, 미지원 필드 미포함, `response.completed` 수신 시에만 성공 처리.
- (h) 오류: 8.6절 매핑 기준 분기, 일시 장애 시 자격 증명 보존과 제한된 백오프, `subscription_sharing_usage_limit_exceeded` 시 `https://chatgpt.com/settings/usage` 안내.
- (i) 로그아웃: 폐기 엔드포인트 호출 후 토큰 삭제, client 매핑과 host ID 보존, 폐기 미확인 시 사용자 고지.
- (j) 헤드리스: 공식 경로(로컬 로그인 후 자격 증명 내보내기와 가져오기 명령, VM 고유 host ID 유지)를 기본 지원. 포트 포워딩과 URL 붙여넣기 방식은 비공식으로 표기.

---

## 13. 미확인 항목과 확인 방법

| 항목 | 확인 방법 |
| --- | --- |
| Free, Go 플랜의 오픈소스 도구 경로 사용 가능 여부 | 해당 플랜 계정으로 실측, OpenAI 지원 문의 |
| Business, Enterprise, Edu 워크스페이스 허용 여부 | 해당 워크스페이스 계정으로 실측, 관리자 설정 문서 확인 |
| 허용 지역 목록, 한국 허용 여부 | 한국 소재 계정 실측, OpenAI 지원 문의 |
| SIWC 전용 연령 제한 | OpenAI 지원 문의 |
| 신규 동적 등록 앱의 자동 활성화 여부(`client_not_enabled` 가능성) | 신규 `agent_name_hint`로 등록 후 추론 실측 |
| 구조화 출력(`text.format` JSON schema) 지원 여부 | 실측 요청, 오류 시 `error.param` 확인 |
| `reasoning` 파라미터 지원 여부 | 실측 요청 |
| namespace 함수 도구 실제 동작 | 실측 요청 |
| 요청 빈도 한도(RPM, TPM), `Retry-After` 제공 여부 | 실측, 응답 헤더 관찰 |
| `earliest_refresh_at` 의미와 형식(숫자 또는 문자열) | 실제 토큰 응답 관찰, 문서 갱신 확인 |
| SSH 포트 포워딩 및 URL 붙여넣기 방식의 공식 허용 여부 | OpenAI 문의, 문서 갱신 확인 |
| 사용자 지정 스킴 redirect URI 허용 여부 | OpenAI 문의 |
| TUI 텍스트 전용 버튼의 브랜딩 적합성 | OpenAI 브랜드 지침 문의 |
| SIWC 약관의 DevKit 라이선스 참조 적용 범위 | 법률 검토, OpenAI 문의 |
| DevKit 패키지 npm 공개 배포 여부 | npm 레지스트리 조회 |
| 정식 출시 일정 | 릴리스 노트, 개발자 문서 추적 |

---

## 출처

공식 개발자 문서 (developers.openai.com):

- https://developers.openai.com/llms.txt
- https://developers.openai.com/siwc/llms.txt
- https://developers.openai.com/siwc/llms-full.txt
- https://developers.openai.com/siwc/quickstart
- https://developers.openai.com/siwc/request-client-id
- https://developers.openai.com/siwc/website
- https://developers.openai.com/siwc/chatgpt-plugin
- https://developers.openai.com/siwc/ui-ux-guidelines
- https://developers.openai.com/siwc/token-sharing-open-source
- https://developers.openai.com/siwc/token-sharing-open-source/sign-in
- https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions
- https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference
- https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server
- https://developers.openai.com/siwc/token-sharing-open-source/self-hosted-vms
- https://developers.openai.com/siwc/token-sharing-open-source/token-reference
- https://developers.openai.com/siwc/token-sharing-open-source/errors-and-recovery
- https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations
- https://developers.openai.com/cookbook/articles/sign-in-with-chatgpt
- https://developers.openai.com/api/docs/guides/tools-tool-search

인증 서버 공개 메타데이터:

- https://auth.openai.com/.well-known/openid-configuration

ChatGPT 문서 및 도움말 센터:

- https://learn.chatgpt.com/docs/sign-in-with-chatgpt
- https://learn.chatgpt.com/docs/auth
- https://help.openai.com/en/articles/20001542-using-your-chatgpt-plan-in-other-apps-and-sites (직접 접근 시 403, 공개 리더 프록시 r.jina.ai 경유 열람)
- https://help.openai.com/en/articles/20001410 (동일 방식 열람)
- https://help.openai.com/en/articles/6825453-chatgpt-release-notes (동일 방식 열람)

약관 및 정책 (openai.com):

- https://openai.com/policies/sign-in-with-chatgpt-terms/ (동일 방식 열람)
- https://openai.com/policies/service-terms/
- https://openai.com/policies/row-terms-of-use/
- https://openai.com/brand/
- https://openai.com/form/sign-in-with-chatgpt-interest/ (열람만, 제출 없음)

공식 GitHub 저장소:

- https://github.com/openai/sign-in-with-chatgpt-devkit
- https://github.com/openai/sign-in-with-chatgpt-devkit/blob/main/LICENSE
- https://github.com/openai/sign-in-with-chatgpt-devkit/blob/main/docs/security.md
- https://github.com/openai/sign-in-with-chatgpt-devkit/blob/main/packages/local/src/oauth.ts
- https://github.com/openai/sign-in-with-chatgpt-devkit/blob/main/packages/local/src/index.ts
- https://github.com/openai/sign-in-with-chatgpt-devkit/blob/main/packages/local/src/errors.ts
- https://github.com/openai/sign-in-with-chatgpt-devkit/blob/main/packages/local/src/models.ts
- https://github.com/openai/sign-in-with-chatgpt-devkit/blob/main/packages/local/src/responses.ts
- https://github.com/openai/sign-in-with-chatgpt-devkit/blob/main/packages/local/src/types.ts
- https://github.com/openai/sign-in-with-chatgpt-devkit/blob/main/examples/paste-perfect/electron/main.ts

참고용 제3자 구현 (비공식):

- https://github.com/openclaw/openclaw/blob/main/extensions/openai/token-sharing.ts
- https://github.com/TabularisDB/tabularis
- https://github.com/jlcodes99/cockpit-tools
