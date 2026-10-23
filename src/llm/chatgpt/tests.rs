//! ChatGPT Provider 테스트 (TC-LLM-008, 014-016, TC-AUTH-024). Mock HTTP 서버 사용.

use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

use super::*;
use crate::auth::oauth::Endpoints;
use crate::auth::secret::Secret;
use crate::auth::store::{PLAN_SCOPE, TokenSet};
use crate::llm::types::{EnvContext, OsKind};
use crate::shell::ShellKind;

fn tokens(scope: &str) -> TokenSet {
    TokenSet {
        access_token: Secret::new("at-1"),
        refresh_token: Secret::new("rt-1"),
        id_token: None,
        token_type: "Bearer".into(),
        scope: scope.split_whitespace().map(str::to_string).collect(),
        expires_at: now() + 3600,
        earliest_refresh_at: None,
        saved_at: now(),
    }
}

fn setup(
    server: &MockServer,
    dir: &std::path::Path,
    scope: &str,
    pref: Option<&str>,
) -> ChatGptProvider {
    let mut auth = AuthManager::new(dir.to_path_buf(), crate::llm::http::client());
    auth.endpoints = Endpoints {
        issuer: "https://auth.openai.com".into(),
        authorize: format!("{}/authorize", server.uri()),
        token: format!("{}/token", server.uri()),
        revoke: format!("{}/revoke", server.uri()),
        jwks: format!("{}/jwks", server.uri()),
        api_base: format!("{}/v1", server.uri()),
    };
    auth.seed_for_tests("oaiapp_1", &tokens(scope));
    ChatGptProvider::new(
        crate::llm::http::client(),
        Arc::new(auth),
        pref.map(str::to_string),
    )
}

fn request(op: Operation) -> LlmRequest {
    LlmRequest {
        id: 1,
        op,
        env: EnvContext {
            os: OsKind::Linux,
            arch: "x86_64",
            shell: ShellKind::Bash,
            shell_version: None,
        },
        question: Some("find big files".into()),
        command: if op == Operation::Generate {
            None
        } else {
            Some("ls".into())
        },
        history: vec![],
        include_folder: false,
        folder: None,
    }
}

fn sse(events: &[Value]) -> String {
    events
        .iter()
        .map(|e| format!("event: {}\ndata: {}\n\n", e["type"].as_str().unwrap(), e))
        .collect()
}

fn completed(text: &str) -> Vec<Value> {
    let half = text.len() / 2;
    vec![
        json!({"type": "response.created", "response": {"id": "r1"}}),
        json!({"type": "response.output_text.delta", "delta": &text[..half]}),
        json!({"type": "response.output_text.delta", "delta": &text[half..]}),
        json!({"type": "response.completed", "response": {"output": [{"type": "message", "content": [{"type": "output_text", "text": text}]}]}}),
    ]
}

async fn mount_models(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .and(header("authorization", "Bearer at-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"models": [
            {"slug": "hidden-1", "display_name": "Hidden", "visibility": "hide"},
            {"slug": "model-a", "display_name": "Model A", "visibility": "list"},
            {"slug": "model-b", "display_name": "Model B", "visibility": "list"}
        ]})))
        .mount(server)
        .await;
}

const ANSWER: &str = r#"{"command":"find . -type f -size +100M","explanation":"Finds files over 100 MB.","notes":[]}"#;

#[tokio::test]
async fn generates_with_required_fields_and_streaming() {
    let server = MockServer::start().await;
    mount_models(&server).await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(header("authorization", "Bearer at-1"))
        .and(body_partial_json(
            json!({"model": "model-a", "store": false, "stream": true}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse(&completed(ANSWER))),
        )
        .expect(1)
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let p = setup(&server, dir.path(), &format!("openid {PLAN_SCOPE}"), None);
    let label = p.prepare().await.unwrap();
    assert_eq!(
        label,
        ProviderLabel {
            model: Some("model-a".into()),
            chatgpt_plan: true
        }
    );
    let r = p
        .complete(request(Operation::Generate), CancelToken::new())
        .await
        .unwrap();
    assert_eq!(r.command.as_deref(), Some("find . -type f -size +100M"));
    assert_eq!(r.model, "model-a");
    // 요청 본문 검사: 미지원 필드 부재, instructions 사용, system 역할 없음
    let reqs = server.received_requests().await.unwrap();
    let body: Value = reqs
        .iter()
        .find(|r| r.url.path() == "/v1/responses")
        .unwrap()
        .body_json()
        .unwrap();
    for field in [
        "temperature",
        "top_p",
        "max_output_tokens",
        "previous_response_id",
        "metadata",
        "user",
        "truncation",
    ] {
        assert!(body.get(field).is_none(), "{field} 포함 금지");
    }
    assert!(
        body["instructions"]
            .as_str()
            .unwrap()
            .contains("shell=Bash")
    );
    assert!(
        body["input"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["role"] != "system")
    );
    assert_eq!(body["text"]["format"]["type"], "json_schema");
}

#[tokio::test]
async fn model_preference_and_cache() {
    let models = vec![
        ModelInfo {
            slug: "a".into(),
            display_name: None,
        },
        ModelInfo {
            slug: "b".into(),
            display_name: None,
        },
    ];
    assert_eq!(
        ChatGptProvider::choose_model(&models, None, None).as_deref(),
        Some("a")
    );
    assert_eq!(
        ChatGptProvider::choose_model(&models, None, Some("b")).as_deref(),
        Some("b")
    );
    assert_eq!(
        ChatGptProvider::choose_model(&models, None, Some("zzz")).as_deref(),
        Some("a")
    );
    assert_eq!(ChatGptProvider::choose_model(&[], None, None), None);
}

/// CR-005: 설정 화면 선택 > 설정 파일 > 기본 모델(gpt-6-luna) > 첫 모델. 목록에 없는 선택은 무시.
#[test]
fn model_choice_order_with_selection_and_default() {
    let m = |s: &str| ModelInfo {
        slug: s.into(),
        display_name: None,
    };
    let models = vec![m("gpt-6.1-sol"), m("gpt-6-luna"), m("b")];
    let pick = |sel, pref| ChatGptProvider::choose_model(&models, sel, pref);
    assert_eq!(pick(None, None).as_deref(), Some("gpt-6-luna"));
    assert_eq!(pick(None, Some("b")).as_deref(), Some("b"));
    assert_eq!(
        pick(Some("gpt-6.1-sol"), Some("b")).as_deref(),
        Some("gpt-6.1-sol")
    );
    assert_eq!(pick(Some("gone"), None).as_deref(), Some("gpt-6-luna"));
    assert_eq!(
        ChatGptProvider::choose_model(&[m("x"), m("y")], None, None).as_deref(),
        Some("x")
    );
}

#[tokio::test]
async fn structured_output_fallback_is_remembered() {
    let server = MockServer::start().await;
    mount_models(&server).await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(|req: &Request| serde_json::from_slice::<Value>(&req.body).is_ok_and(|b| b.get("text").is_some()))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({"error": {"code": "subscription_sharing_unsupported_capability", "param": "text.format", "message": "x"}})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(|req: &Request| {
            serde_json::from_slice::<Value>(&req.body).is_ok_and(|b| b.get("text").is_none())
        })
        .respond_with(ResponseTemplate::new(200).set_body_string(sse(&completed(ANSWER))))
        .expect(2)
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let p = setup(&server, dir.path(), PLAN_SCOPE, None);
    assert!(
        p.complete(request(Operation::Generate), CancelToken::new())
            .await
            .is_ok()
    );
    assert_eq!(p.auth.load_state().structured_output, Some(false));
    assert!(
        p.complete(request(Operation::Revise), CancelToken::new())
            .await
            .is_ok(),
        "두 번째 요청은 바로 지시문 방식"
    );
}

#[tokio::test]
async fn unauthorized_refreshes_once_and_retries() {
    let server = MockServer::start().await;
    mount_models(&server).await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(header("authorization", "Bearer at-1"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({"detail": "expired"})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"access_token": "at-2", "refresh_token": "rt-2", "expires_in": 3600}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(header("authorization", "Bearer at-2"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sse(&completed(ANSWER))))
        .expect(1)
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let p = setup(&server, dir.path(), PLAN_SCOPE, None);
    // 다른 프로세스 갱신으로 판단하지 않도록 저장 시각을 과거로
    let mut t = tokens(PLAN_SCOPE);
    t.saved_at = now() - 100;
    p.auth.seed_for_tests("oaiapp_1", &t);
    assert!(
        p.complete(request(Operation::Generate), CancelToken::new())
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn plan_not_granted_sends_no_request() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let p = setup(&server, dir.path(), "openid profile", None);
    assert_eq!(
        p.complete(request(Operation::Generate), CancelToken::new())
            .await,
        Err(LlmError::PlanNotGranted)
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn stream_failures() {
    let server = MockServer::start().await;
    mount_models(&server).await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sse(&[
            json!({"type": "response.output_text.delta", "delta": "{"}),
            json!({"type": "response.failed", "response": {"error": {"code": "subscription_sharing_usage_limit_exceeded"}}}),
        ])))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let p = setup(&server, dir.path(), PLAN_SCOPE, None);
    assert_eq!(
        p.complete(request(Operation::Generate), CancelToken::new())
            .await,
        Err(LlmError::UsageLimit)
    );

    server.reset().await;
    mount_models(&server).await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sse(&[
            json!({"type": "response.output_text.delta", "delta": "{"}),
        ])))
        .mount(&server)
        .await;
    assert!(
        matches!(
            p.complete(request(Operation::Generate), CancelToken::new())
                .await,
            Err(LlmError::Network { .. })
        ),
        "completed 없이 종료"
    );
}

#[tokio::test]
async fn cancel_returns_promptly() {
    let server = MockServer::start().await;
    mount_models(&server).await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(5))
                .set_body_string(sse(&completed(ANSWER))),
        )
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let p = setup(&server, dir.path(), PLAN_SCOPE, None);
    let cancel = CancelToken::new();
    let c = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        c.cancel();
    });
    let start = std::time::Instant::now();
    assert_eq!(
        p.complete(request(Operation::Generate), cancel).await,
        Err(LlmError::Cancelled)
    );
    assert!(start.elapsed() < Duration::from_secs(1));
}

/// research/chatgpt-oauth.md 8.6절 매핑 표 (TC-LLM-015).
#[test]
fn error_mapping_table() {
    let e = |status: u16, body: Value| map_http_error(status, &body.to_string());
    let code = |c: &str, status: u16| {
        e(
            status,
            json!({"error": {"code": c, "message": "m", "param": "p"}}),
        )
    };
    assert_eq!(
        e(401, json!({"detail": "no"})),
        LlmError::AuthFailed {
            detail: "no".into()
        }
    );
    assert_eq!(
        e(403, json!({"detail": "region"})),
        LlmError::RegionOrPolicy {
            detail: "region".into()
        }
    );
    assert_eq!(
        e(503, json!({"detail": "x"})),
        LlmError::Transient { status: Some(503) }
    );
    assert_eq!(
        code("subscription_sharing_user_not_eligible", 403),
        LlmError::NotEligible
    );
    assert_eq!(
        code("subscription_sharing_usage_limit_exceeded", 429),
        LlmError::UsageLimit
    );
    assert_eq!(
        code("subscription_sharing_usage_unavailable", 503),
        LlmError::Transient { status: Some(503) }
    );
    assert_eq!(
        code("subscription_sharing_unsupported_capability", 400),
        LlmError::Unsupported {
            param: Some("p".into())
        }
    );
    assert_eq!(
        code("subscription_sharing_route_not_supported", 403),
        LlmError::PermissionConfig {
            code: "subscription_sharing_route_not_supported".into()
        }
    );
    assert_eq!(
        code("subscription_sharing_invalid_user", 401),
        LlmError::AuthFailed {
            detail: "subscription_sharing_invalid_user".into()
        }
    );
    assert_eq!(
        code("chatpass_v2_scope_not_authorized", 403),
        LlmError::PermissionConfig {
            code: "chatpass_v2_scope_not_authorized".into()
        }
    );
    assert_eq!(
        code("chatpass_v2_invalid_authorization_context", 403),
        LlmError::PermissionConfig {
            code: "chatpass_v2_invalid_authorization_context".into()
        }
    );
    assert_eq!(
        code("subscription_sharing_user_unavailable", 503),
        LlmError::Transient { status: Some(503) }
    );
    assert_eq!(
        code("subscription_sharing_v2_client_not_enabled", 403),
        LlmError::ClientNotEnabled
    );
    assert_eq!(code("model_not_found", 404), LlmError::ModelNotFound);
    assert_eq!(e(429, json!({})), LlmError::Transient { status: Some(429) });
    assert_eq!(
        e(502, json!("bad gateway")),
        LlmError::Transient { status: Some(502) }
    );
}

/// CR-005: 설정 화면 내용과 모델 선택 저장 (다음 실행에도 유지).
#[tokio::test]
async fn settings_list_models_and_selection_persists() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"models": [
            {"slug": "gpt-6.1-sol", "display_name": "GPT-6.1-Sol", "visibility": "list"},
            {"slug": "gpt-6-luna", "display_name": "GPT-6-Luna", "visibility": "list"},
            {"slug": "hidden", "visibility": "hide"}
        ]})))
        .expect(1)
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let p = setup(&server, dir.path(), PLAN_SCOPE, None);
    let s = p.settings().await.unwrap();
    assert_eq!(s.agent, "ChatGPT");
    assert_eq!(
        s.models.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
        ["gpt-6.1-sol", "gpt-6-luna"]
    );
    assert_eq!(s.current.as_deref(), Some("gpt-6-luna"), "기본 모델");
    assert!(matches!(
        p.select_model("hidden".into()).await,
        Err(LlmError::ModelNotFound)
    ));
    let label = p.select_model("gpt-6.1-sol".into()).await.unwrap();
    assert_eq!(label.model.as_deref(), Some("gpt-6.1-sol"));
    // 같은 데이터 디렉터리의 새 Provider (다음 실행): 저장된 선택 사용, 목록은 캐시
    let next = setup(&server, dir.path(), PLAN_SCOPE, Some("gpt-6-luna"));
    assert_eq!(
        next.prepare().await.unwrap().model.as_deref(),
        Some("gpt-6.1-sol")
    );
}
