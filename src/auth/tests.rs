//! 인증 흐름 테스트 (TC-AUTH-001-007, 012-016, 020-023). Mock 인증 서버(wiremock) 사용.

use std::sync::Arc;
use std::sync::Mutex as StdMutex;

use serde_json::json;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::idtoken::test_support;
use super::*;

fn manager(dir: &Path, server: &MockServer) -> AuthManager {
    let s = StateFile {
        credential_store: Some("file".into()),
        ..Default::default()
    };
    state::save(dir, &s).unwrap();
    let mut m = AuthManager::new(dir.to_path_buf(), crate::llm::http::client());
    m.endpoints = Endpoints {
        issuer: "https://auth.openai.com".into(),
        authorize: format!("{}/authorize", server.uri()),
        token: format!("{}/token", server.uri()),
        revoke: format!("{}/revoke", server.uri()),
        jwks: format!("{}/jwks", server.uri()),
        api_base: format!("{}/v1", server.uri()),
    };
    m
}

fn id_token(nonce: &str, client_id: &str) -> String {
    test_support::sign(
        &json!({"iss": "https://auth.openai.com", "aud": client_id, "exp": now() + 3600, "iat": now(), "sub": "user-1", "nonce": nonce, "email": "kim@example.com"}),
        "k1",
    )
}

async fn mount_jwks(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"keys": [test_support::jwk()]})),
        )
        .mount(server)
        .await;
}

/// 브라우저 대신 인가 URL을 해석해 콜백(또는 붙여넣기 URL)을 만든다.
fn callback_from(
    url: &str,
    client_id: Option<&str>,
    override_state: Option<&str>,
) -> (String, String, u16) {
    let parsed = url::Url::parse(url).unwrap();
    let q: std::collections::HashMap<String, String> = parsed
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    let redirect = q["redirect_uri"].clone();
    let port = url::Url::parse(&redirect).unwrap().port().unwrap();
    let state = override_state.unwrap_or(&q["state"]).to_string();
    let mut cb = format!("{redirect}?code=code-1&state={state}");
    if let Some(id) = client_id {
        cb.push_str(&format!("&client_id={id}"));
    }
    (cb, q["nonce"].clone(), port)
}

type Captured = Arc<StdMutex<Option<String>>>;

fn progress_capture() -> (Captured, Box<dyn Fn(LoginProgress) + Send + Sync>) {
    let captured: Captured = Arc::new(StdMutex::new(None));
    let c = captured.clone();
    (
        captured,
        Box::new(move |p| {
            if let LoginProgress::Started(v) = p {
                *c.lock().unwrap() = v.url
            }
        }),
    )
}

async fn wait_url(captured: &Captured) -> String {
    for _ in 0..200 {
        if let Some(u) = captured.lock().unwrap().clone() {
            return u;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("authorize URL not reported");
}

async fn http_get(url: &str) {
    let resp = crate::llm::http::client().get(url).send().await.unwrap();
    assert!(resp.status().is_success());
}

#[tokio::test]
async fn first_login_registers_verifies_and_stores() {
    let server = MockServer::start().await;
    mount_jwks(&server).await;
    let dir = tempfile::tempdir().unwrap();
    let auth = Arc::new(manager(dir.path(), &server));
    let (captured, progress) = progress_capture();
    let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let a = auth.clone();
    let task = tokio::spawn(async move {
        a.login(
            LoginOptions {
                no_browser: true,
                port: None,
            },
            &*progress,
            rx,
            CancelToken::new(),
        )
        .await
    });
    let url = wait_url(&captured).await;
    assert!(url.contains("client_id=dynamic_agent_client"));
    assert!(url.contains("agent_name_hint=howtui"));
    let (cb, nonce, _) = callback_from(&url, Some("oaiapp_1"), None);
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("grant_type=authorization_code"))
        .and(body_string_contains("client_id=oaiapp_1"))
        .and(body_string_contains("code_verifier="))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "at-1", "refresh_token": "rt-1", "id_token": id_token(&nonce, "oaiapp_1"),
            "token_type": "Bearer", "expires_in": 3600,
            "scope": "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct"
        })))
        .expect(1)
        .mount(&server)
        .await;
    http_get(&cb).await;
    let info = task.await.unwrap().unwrap();
    assert!(info.plan_enabled);
    assert!(info.show_plan_notice);
    assert_eq!(info.email.as_deref(), Some("kim@example.com"));
    let s = auth.load_state();
    assert_eq!(s.registration.as_ref().unwrap().client_id, "oaiapp_1");
    assert!(s.host_id.as_deref().unwrap().starts_with("urn:uuid:"));
    assert_eq!(auth.access_token().await.unwrap().expose(), "at-1");
    // 두 번째 로그인: 발급 client_id 재사용, 플랜 안내 미표시
    let host = s.host_id.clone();
    let (captured, progress) = progress_capture();
    let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let a = auth.clone();
    let task = tokio::spawn(async move {
        a.login(
            LoginOptions {
                no_browser: true,
                port: None,
            },
            &*progress,
            rx,
            CancelToken::new(),
        )
        .await
    });
    let url = wait_url(&captured).await;
    assert!(url.contains("client_id=oaiapp_1") && !url.contains("agent_name_hint"));
    let (cb, nonce, _) = callback_from(&url, None, None);
    server.reset().await;
    mount_jwks(&server).await;
    Mock::given(method("POST")).and(path("/token")).respond_with(ResponseTemplate::new(200).set_body_json(json!({
        "access_token": "at-2", "refresh_token": "rt-2", "id_token": id_token(&nonce, "oaiapp_1"), "expires_in": 3600,
        "scope": "openid chatgpt.tokens.use.direct"
    }))).mount(&server).await;
    http_get(&cb).await;
    let info = task.await.unwrap().unwrap();
    assert!(!info.show_plan_notice);
    assert_eq!(auth.load_state().host_id, host, "host ID 유지");
}

#[tokio::test]
async fn pasted_redirect_completes_login_and_bad_paste_is_reported() {
    let server = MockServer::start().await;
    mount_jwks(&server).await;
    let dir = tempfile::tempdir().unwrap();
    let auth = Arc::new(manager(dir.path(), &server));
    let captured: Captured = Arc::new(StdMutex::new(None));
    let invalid = Arc::new(StdMutex::new(0));
    let (c, inv) = (captured.clone(), invalid.clone());
    let progress: Box<dyn Fn(LoginProgress) + Send + Sync> = Box::new(move |p| match p {
        LoginProgress::Started(v) => *c.lock().unwrap() = v.url,
        LoginProgress::InvalidPaste => *inv.lock().unwrap() += 1,
    });
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let a = auth.clone();
    let task = tokio::spawn(async move {
        a.login(
            LoginOptions {
                no_browser: true,
                port: None,
            },
            &*progress,
            rx,
            CancelToken::new(),
        )
        .await
    });
    let url = wait_url(&captured).await;
    let (cb, nonce, port) = callback_from(&url, Some("oaiapp_9"), None);
    Mock::given(method("POST")).and(path("/token")).respond_with(ResponseTemplate::new(200).set_body_json(json!({
        "access_token": "at", "refresh_token": "rt", "id_token": id_token(&nonce, "oaiapp_9"), "expires_in": 3600, "scope": "openid"
    }))).mount(&server).await;
    tx.send(format!("http://localhost:{port}/auth/callback?code=x"))
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(*invalid.lock().unwrap(), 1);
    tx.send(format!("  {cb}\n")).unwrap();
    let info = task.await.unwrap().unwrap();
    assert!(!info.plan_enabled, "scope 미부여");
    assert!(
        auth.load_state().plan_disabled_last,
        "다음 로그인 prompt=consent"
    );
}

#[tokio::test]
async fn state_mismatch_and_denied_and_cancel() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let auth = Arc::new(manager(dir.path(), &server));
    for (mutate, expect) in [
        ("state", LoginFailure::StateMismatch),
        ("deny", LoginFailure::Denied),
    ] {
        let (captured, progress) = progress_capture();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let a = auth.clone();
        let task = tokio::spawn(async move {
            a.login(
                LoginOptions {
                    no_browser: true,
                    port: None,
                },
                &*progress,
                rx,
                CancelToken::new(),
            )
            .await
        });
        let url = wait_url(&captured).await;
        let pasted = if mutate == "state" {
            callback_from(&url, Some("oaiapp_1"), Some("forged")).0
        } else {
            let (cb, _, _) = callback_from(&url, None, None);
            let state = url::Url::parse(&cb)
                .unwrap()
                .query_pairs()
                .find(|(k, _)| k == "state")
                .unwrap()
                .1
                .into_owned();
            format!(
                "{}?error=access_denied&state={state}",
                cb.split('?').next().unwrap()
            )
        };
        tx.send(pasted).unwrap();
        assert_eq!(task.await.unwrap().unwrap_err(), expect);
    }
    let (_captured, progress) = progress_capture();
    let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let cancel = CancelToken::new();
    let c = cancel.clone();
    let a = auth.clone();
    let task = tokio::spawn(async move {
        a.login(
            LoginOptions {
                no_browser: true,
                port: None,
            },
            &*progress,
            rx,
            c,
        )
        .await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    cancel.cancel();
    assert_eq!(task.await.unwrap().unwrap_err(), LoginFailure::Cancelled);
    assert!(
        auth.load_state().registration.is_none(),
        "토큰 교환 전 실패 시 등록 정보 없음"
    );
}

fn seed_tokens(auth: &AuthManager, expires_at: u64) {
    let mut s = auth.load_state();
    s.registration = Some(Registration {
        client_id: "oaiapp_1".into(),
        ..Default::default()
    });
    auth.save_state(&s);
    let t = TokenSet {
        access_token: Secret::new("old-at"),
        refresh_token: Secret::new("old-rt"),
        id_token: None,
        token_type: "Bearer".into(),
        scope: vec!["chatgpt.tokens.use.direct".into()],
        expires_at,
        earliest_refresh_at: None,
        saved_at: now() - 3000,
    };
    auth.save_tokens("oaiapp_1", &t).unwrap();
}

#[tokio::test]
async fn refresh_is_serialized_across_managers() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .and(body_string_contains("refresh_token=old-rt"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(200))
                .set_body_json(json!({
                    "access_token": "new-at", "refresh_token": "new-rt", "expires_in": 3600
                })),
        )
        .expect(1)
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let a = Arc::new(manager(dir.path(), &server));
    seed_tokens(&a, now() + 10);
    // 같은 데이터 디렉터리를 쓰는 두 번째 관리자 (다른 프로세스 대응)
    let b = Arc::new(AuthManager {
        endpoints: a.endpoints.clone(),
        ..AuthManager::new(dir.path().to_path_buf(), crate::llm::http::client())
    });
    let (ra, rb) = tokio::join!(a.access_token(), b.access_token());
    assert_eq!(ra.unwrap().expose(), "new-at");
    assert_eq!(rb.unwrap().expose(), "new-at");
    let stored = a.tokens().unwrap();
    assert_eq!(stored.refresh_token.expose(), "new-rt");
    assert_eq!(
        stored.scope,
        vec!["chatgpt.tokens.use.direct".to_string()],
        "scope 생략 응답은 기존 값 유지"
    );
}

#[tokio::test]
async fn terminal_refresh_error_deletes_tokens() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(json!({"error": "refresh_token_reused"})),
        )
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let auth = manager(dir.path(), &server);
    seed_tokens(&auth, now() + 5);
    assert!(matches!(
        auth.access_token().await,
        Err(AuthError::Rejected(_))
    ));
    assert!(!auth.has_credentials());
    assert!(auth.registration().is_some(), "client 매핑 보존");
}

#[tokio::test]
async fn transient_refresh_failure_keeps_credentials() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let auth = manager(dir.path(), &server);
    seed_tokens(&auth, now() + 30);
    // 만료 전이면 기존 토큰 사용
    assert_eq!(auth.access_token().await.unwrap().expose(), "old-at");
    assert!(auth.has_credentials());
}

#[tokio::test]
async fn logout_revokes_and_keeps_registration() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/revoke"))
        .and(body_string_contains("token_type_hint=refresh_token"))
        .and(body_string_contains("token=old-rt"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let auth = manager(dir.path(), &server);
    let host = state::ensure_host_id(dir.path()).unwrap();
    seed_tokens(&auth, now() + 3600);
    assert_eq!(auth.logout(false).await, LogoutOutcome::Revoked);
    assert!(!auth.has_credentials());
    let s = auth.load_state();
    assert!(s.registration.is_some());
    assert_eq!(s.host_id, Some(host.clone()));
    assert_eq!(auth.logout(false).await, LogoutOutcome::NotSignedIn);
}

#[tokio::test]
async fn logout_with_server_error_reports_unconfirmed_and_forget_clears_registration() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/revoke"))
        .respond_with(ResponseTemplate::new(502))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let auth = manager(dir.path(), &server);
    let host = state::ensure_host_id(dir.path()).unwrap();
    seed_tokens(&auth, now() + 3600);
    assert_eq!(auth.logout(true).await, LogoutOutcome::RevokeUnconfirmed);
    let s = auth.load_state();
    assert!(s.registration.is_none());
    assert_eq!(s.host_id, Some(host), "forget 후에도 host ID 유지");
}

#[test]
fn email_masking() {
    assert_eq!(mask_email("kim@example.com"), "k***@example.com");
    assert_eq!(mask_email("x"), "***");
}
