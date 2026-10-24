//! API Key Provider 테스트 (TC-LLM-018). Mock HTTP 서버 사용.

use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{body_partial_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

use super::*;
use crate::llm::types::{EnvContext, OsKind};
use crate::shell::ShellKind;

const ANSWER: &str = r#"{"command":"ls -al","explanation":"Lists files.","notes":[]}"#;

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
        question: Some("list files".into()),
        command: None,
        history: Vec::new(),
        include_folder: false,
        folder: None,
    }
}

fn provider(kind: ProviderKind, server: &MockServer, key: Option<&str>) -> ApiProvider {
    let cfg = ApiProviderConfig {
        base_url: Some(format!("{}/v1", server.uri())),
        ..Default::default()
    };
    ApiProvider::new(
        kind,
        crate::llm::http::client(),
        &cfg,
        key.map(Secret::new),
        None,
    )
    .with_base(format!("{}/v1", server.uri()))
}

fn models(ids: &[&str]) -> Vec<ModelInfo> {
    ids.iter()
        .map(|s| ModelInfo {
            slug: s.to_string(),
            display_name: None,
        })
        .collect()
}

#[test]
fn model_choice_order() {
    let list = models(&["a", "b", "gpt-6-astra"]);
    let d = Some("gpt-6-astra");
    assert_eq!(
        choose_model(&list, Some("b"), Some("a"), d).as_deref(),
        Some("b")
    );
    assert_eq!(
        choose_model(&list, Some("gone"), None, d).as_deref(),
        Some("gpt-6-astra")
    );
    assert_eq!(
        choose_model(&list, None, Some("custom"), d).as_deref(),
        Some("custom")
    );
    assert_eq!(
        choose_model(&models(&["x"]), None, None, d).as_deref(),
        Some("x")
    );
    assert_eq!(
        choose_model(&[], None, None, d).as_deref(),
        Some("gpt-6-astra")
    );
    assert_eq!(choose_model(&[], None, None, None), None);
}

#[test]
fn request_bodies_per_provider() {
    let mut req = request(Operation::Revise);
    req.command = Some("ls".into());
    req.history.push(Turn {
        op: Operation::Generate,
        question: Some("list".into()),
        command_sent: None,
        response: LlmResponse {
            command: Some("ls".into()),
            explanation: "x".into(),
            notes: vec![],
            model: "m".into(),
        },
    });

    let b = body(ProviderKind::Openai, &req, "gpt-6-astra", true);
    assert_eq!(b["store"], false);
    assert_eq!(b["text"]["format"]["type"], "json_schema");
    assert_eq!(b["input"][1]["role"], "assistant");
    assert!(b.get("temperature").is_none() && b.get("stream").is_none());
    assert!(
        body(ProviderKind::Openai, &req, "m", false)
            .get("text")
            .is_none()
    );

    let b = body(ProviderKind::Anthropic, &req, "claude-opus-5-5", true);
    assert!(b["system"].as_str().unwrap().contains("howtui"));
    assert_eq!(b["messages"][0]["role"], "user");
    assert_eq!(b["output_config"]["effort"], "low");
    assert_eq!(b["output_config"]["format"]["type"], "json_schema");
    assert_eq!(b["fallbacks"], "default");
    assert!(b.get("temperature").is_none() && b.get("thinking").is_none());
    // 목록 밖 모델: effort, fallbacks 생략
    let b = body(ProviderKind::Anthropic, &req, "claude-haiku-4-5", false);
    assert!(b.get("output_config").is_none() && b.get("fallbacks").is_none());

    let b = body(ProviderKind::Gemini, &req, "gemini-3.8-flash", true);
    assert_eq!(b["contents"][1]["role"], "model");
    assert_eq!(
        b["generationConfig"]["responseMimeType"],
        "application/json"
    );
    assert!(b["systemInstruction"]["parts"][0]["text"].is_string());

    let b = body(ProviderKind::OpenaiCompatible, &req, "local", false);
    assert_eq!(b["messages"][0]["role"], "system");
    assert_eq!(b["stream"], false);
    assert!(b.get("response_format").is_none());
}

#[test]
fn extract_text_and_refusals() {
    let openai = json!({"status":"completed","output":[{"type":"reasoning"},{"type":"message","content":[{"type":"output_text","text":ANSWER}]}]});
    assert_eq!(extract(ProviderKind::Openai, &openai).unwrap(), ANSWER);
    let refused =
        json!({"output":[{"type":"message","content":[{"type":"refusal","refusal":"no"}]}]});
    assert_eq!(
        extract(ProviderKind::Openai, &refused),
        Err(LlmError::Refused)
    );

    let anthropic = json!({"stop_reason":"end_turn","content":[{"type":"thinking","thinking":""},{"type":"text","text":ANSWER}]});
    assert_eq!(
        extract(ProviderKind::Anthropic, &anthropic).unwrap(),
        ANSWER
    );
    assert_eq!(
        extract(
            ProviderKind::Anthropic,
            &json!({"stop_reason":"refusal","content":[]})
        ),
        Err(LlmError::Refused)
    );

    let gemini = json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"text":"hidden","thought":true},{"text":ANSWER}]}}]});
    assert_eq!(extract(ProviderKind::Gemini, &gemini).unwrap(), ANSWER);
    assert_eq!(
        extract(
            ProviderKind::Gemini,
            &json!({"promptFeedback":{"blockReason":"SAFETY"}})
        ),
        Err(LlmError::Refused)
    );
    assert_eq!(
        extract(
            ProviderKind::Gemini,
            &json!({"candidates":[{"finishReason":"SAFETY"}]})
        ),
        Err(LlmError::Refused)
    );

    let compat = json!({"choices":[{"finish_reason":"stop","message":{"content":ANSWER}}]});
    assert_eq!(
        extract(ProviderKind::OpenaiCompatible, &compat).unwrap(),
        ANSWER
    );
    assert!(matches!(
        extract(ProviderKind::OpenaiCompatible, &json!({"choices":[]})),
        Err(LlmError::InvalidResponse { .. })
    ));
}

#[test]
fn error_mapping() {
    use ProviderKind as K;
    let cases: Vec<(K, u16, serde_json::Value, LlmError)> = vec![
        (
            K::Openai,
            401,
            json!({"error":{"code":"invalid_api_key","message":"bad"}}),
            LlmError::KeyRejected {
                provider: "openai".into(),
            },
        ),
        (
            K::Openai,
            429,
            json!({"error":{"code":"insufficient_quota","message":"quota"}}),
            LlmError::QuotaExceeded,
        ),
        (
            K::Openai,
            429,
            json!({"error":{"code":"rate_limit_exceeded","message":"slow"}}),
            LlmError::Transient { status: Some(429) },
        ),
        (
            K::Openai,
            404,
            json!({"error":{"code":"model_not_found","message":"x"}}),
            LlmError::ModelNotFound,
        ),
        (
            K::Openai,
            403,
            json!({"error":{"code":"unsupported_country_region_territory","message":"region"}}),
            LlmError::RegionOrPolicy {
                detail: "region".into(),
            },
        ),
        (
            K::Anthropic,
            401,
            json!({"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}),
            LlmError::KeyRejected {
                provider: "anthropic".into(),
            },
        ),
        (
            K::Anthropic,
            400,
            json!({"type":"error","error":{"type":"invalid_request_error","message":"Your credit balance is too low"}}),
            LlmError::QuotaExceeded,
        ),
        (
            K::Anthropic,
            529,
            json!({"type":"error","error":{"type":"overloaded_error","message":"busy"}}),
            LlmError::Transient { status: Some(529) },
        ),
        (
            K::Anthropic,
            404,
            json!({"type":"error","error":{"type":"not_found_error","message":"model"}}),
            LlmError::ModelNotFound,
        ),
        (
            K::Gemini,
            400,
            json!({"error":{"code":400,"message":"API key not valid.","status":"INVALID_ARGUMENT","details":[{"reason":"API_KEY_INVALID"}]}}),
            LlmError::KeyRejected {
                provider: "gemini".into(),
            },
        ),
        (
            K::Gemini,
            503,
            json!({"error":{"code":503,"message":"unavailable","status":"UNAVAILABLE"}}),
            LlmError::Transient { status: Some(503) },
        ),
        (
            K::OpenaiCompatible,
            404,
            json!({}),
            LlmError::Unsupported {
                param: Some("http 404".into()),
            },
        ),
        (
            K::OpenaiCompatible,
            400,
            json!({"error":{"message":"bad field"}}),
            LlmError::Unsupported {
                param: Some("bad field".into()),
            },
        ),
    ];
    for (kind, status, body, want) in cases {
        assert_eq!(
            map_error(kind, status, &body.to_string()),
            want,
            "{kind:?} {status} {body}"
        );
    }
    assert!(structured_rejected(
        400,
        r#"{"error":{"message":"output_config.format: unsupported"}}"#
    ));
    assert!(!structured_rejected(
        400,
        r#"{"error":{"message":"messages: empty"}}"#
    ));
    assert!(!structured_rejected(500, "schema"));
}

#[tokio::test]
async fn openai_request_headers_and_answer() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .and(header("authorization", "Bearer sk-test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"data":[{"id":"text-embedding-3"},{"id":"gpt-6-astra"},{"id":"gpt-6-mini"}]}),
        ))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(header("authorization", "Bearer sk-test"))
        .and(body_partial_json(json!({"model":"gpt-6-astra","store":false})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":ANSWER}]}]})))
        .expect(1)
        .mount(&server)
        .await;
    let p = provider(ProviderKind::Openai, &server, Some("sk-test"));
    let r = p
        .complete(request(Operation::Generate), CancelToken::new())
        .await
        .unwrap();
    assert_eq!(r.command.as_deref(), Some("ls -al"));
    assert_eq!(r.model, "gpt-6-astra");
    let s = p.settings().await.unwrap();
    assert_eq!(
        s.models.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
        ["gpt-6-astra", "gpt-6-mini"],
        "임베딩 모델 제외"
    );
    assert_eq!(s.current.as_deref(), Some("gpt-6-astra"));
}

#[tokio::test]
async fn anthropic_headers_pagination_and_structured_fallback() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .and(query_param("after_id", "claude-b"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":[{"id":"claude-opus-5-5","display_name":"Claude Opus 5.5"}],"has_more":false})))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .and(header("x-api-key", "ak"))
        .and(header("anthropic-version", "2026-10-21"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":[{"id":"claude-a"},{"id":"claude-b"}],"has_more":true,"last_id":"claude-b"})))
        .mount(&server)
        .await;
    // 구조화 출력 포함 요청은 거부, 지시문 방식 재요청은 성공
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(body_partial_json(json!({"output_config":{"format":{"type":"json_schema"}}})))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({"type":"error","error":{"type":"invalid_request_error","message":"output_config.format: schema type not supported"}})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "ak"))
        .and(header("anthropic-beta", "server-side-fallback-2026-07-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"stop_reason":"end_turn","content":[{"type":"text","text":ANSWER}]}),
        ))
        .expect(2)
        .mount(&server)
        .await;
    let p = provider(ProviderKind::Anthropic, &server, Some("ak"));
    let r = p
        .complete(request(Operation::Generate), CancelToken::new())
        .await
        .unwrap();
    assert_eq!(r.model, "claude-opus-5-5", "두 쪽 목록에서 기본 모델 선택");
    // 두 번째 요청은 처음부터 지시문 방식
    p.complete(request(Operation::Generate), CancelToken::new())
        .await
        .unwrap();
    let posts: Vec<Request> = server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.method.as_str() == "POST")
        .collect();
    assert_eq!(posts.len(), 3);
    let last: serde_json::Value = serde_json::from_slice(&posts[2].body).unwrap();
    assert!(last["output_config"].get("format").is_none());
    assert_eq!(last["output_config"]["effort"], "low");
}

#[tokio::test]
async fn gemini_path_and_model_names() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .and(header("x-goog-api-key", "gk"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"models":[
            {"name":"models/embedding-001","supportedGenerationMethods":["embedContent"]},
            {"name":"models/gemini-3.8-flash","displayName":"Gemini 3.8 Flash","supportedGenerationMethods":["generateContent"]}
        ]})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/models/gemini-3.8-flash:generateContent"))
        .and(header("x-goog-api-key", "gk"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"text":ANSWER}]}}]}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let p = provider(ProviderKind::Gemini, &server, Some("gk"));
    let r = p
        .complete(request(Operation::Generate), CancelToken::new())
        .await
        .unwrap();
    assert_eq!(r.model, "gemini-3.8-flash");
    let s = p.settings().await.unwrap();
    assert_eq!(s.models.len(), 1);
    assert_eq!(s.models[0].name, "Gemini 3.8 Flash");
}

#[tokio::test]
async fn compatible_without_key_and_missing_key_errors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"data":[{"id":"local-model"}]})),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"choices":[{"finish_reason":"stop","message":{"content":ANSWER}}]}),
        ))
        .mount(&server)
        .await;
    let p = provider(ProviderKind::OpenaiCompatible, &server, None);
    let r = p
        .complete(request(Operation::Generate), CancelToken::new())
        .await
        .unwrap();
    assert_eq!(r.model, "local-model");
    let reqs = server.received_requests().await.unwrap();
    assert!(
        reqs.iter()
            .all(|r| !r.headers.contains_key("authorization")),
        "키 없으면 인증 헤더 없음"
    );

    // 키 없는 OpenAI: 네트워크 요청 없이 NoApiKey
    let empty = MockServer::start().await;
    let p = provider(ProviderKind::Openai, &empty, None);
    assert_eq!(
        p.complete(request(Operation::Generate), CancelToken::new())
            .await,
        Err(LlmError::NoApiKey {
            provider: "openai".into(),
            env: Some("OPENAI_API_KEY".into())
        })
    );
    let s = p.settings().await.unwrap();
    assert!(matches!(s.problem, Some(LlmError::NoApiKey { .. })));
    assert!(empty.received_requests().await.unwrap().is_empty());
    assert_eq!(
        LlmError::NoApiKey {
            provider: "x".into(),
            env: None
        }
        .exit_code(),
        4
    );
}

#[tokio::test]
async fn selected_model_is_saved_per_provider() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"data":[{"id":"gpt-6-astra"},{"id":"gpt-6-mini"}]})),
        )
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let auth = Arc::new(AuthManager::new(
        dir.path().to_path_buf(),
        crate::llm::http::client(),
    ));
    let make = || {
        ApiProvider::new(
            ProviderKind::Openai,
            crate::llm::http::client(),
            &ApiProviderConfig::default(),
            Some(Secret::new("k")),
            Some(auth.clone()),
        )
        .with_base(format!("{}/v1", server.uri()))
    };
    let p = make();
    assert_eq!(
        p.select_model("nope".into()).await,
        Err(LlmError::ModelNotFound)
    );
    p.select_model("gpt-6-mini".into()).await.unwrap();
    assert_eq!(
        auth.load_state()
            .models_by_provider
            .get("openai")
            .map(String::as_str),
        Some("gpt-6-mini")
    );
    let again = make();
    assert_eq!(
        again.prepare().await.unwrap().model.as_deref(),
        Some("gpt-6-mini")
    );
}

#[tokio::test]
async fn timeout_and_cancel() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
        .mount(&server)
        .await;
    let cfg = ApiProviderConfig {
        model: Some("m".into()),
        ..Default::default()
    };
    let p = ApiProvider::new(
        ProviderKind::OpenaiCompatible,
        crate::llm::http::client(),
        &cfg,
        None,
        None,
    )
    .with_base(format!("{}/v1", server.uri()))
    .with_timeout(Duration::from_millis(300));
    assert_eq!(
        p.complete(request(Operation::Generate), CancelToken::new())
            .await,
        Err(LlmError::Timeout)
    );
    let cancel = CancelToken::new();
    cancel.cancel();
    assert_eq!(
        p.complete(request(Operation::Generate), cancel).await,
        Err(LlmError::Cancelled)
    );
}
