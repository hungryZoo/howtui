//! SIWC OAuth HTTP 요청 (SRS 11.1절, 11.3절). 공식 문서 기반 독자 구현 (ADR-0007).

use serde::Deserialize;
use serde_json::Value;

use super::state::Jwk;

pub const SCOPE: &str =
    "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";
pub const RESOURCE: &str = "https://api.openai.com/v1";
pub const DYNAMIC_CLIENT: &str = "dynamic_agent_client";
pub const AGENT_NAME: &str = "howtui";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoints {
    pub issuer: String,
    pub authorize: String,
    pub token: String,
    pub revoke: String,
    pub jwks: String,
    pub api_base: String,
}

impl Endpoints {
    pub fn openai() -> Self {
        Self {
            issuer: "https://auth.openai.com".into(),
            authorize: "https://auth.openai.com/api/accounts/authorize".into(),
            token: "https://auth.openai.com/api/accounts/oauth/token".into(),
            revoke: "https://auth.openai.com/api/accounts/oauth/revoke".into(),
            jwks: "https://auth.openai.com/.well-known/jwks.json".into(),
            api_base: "https://api.openai.com/v1".into(),
        }
    }

    /// 테스트 빌드 전용 재정의 (`mock-provider` 기능). 릴리스 빌드는 공식 주소 고정.
    pub fn resolve() -> Self {
        #[allow(unused_mut)]
        let mut e = Self::openai();
        #[cfg(feature = "mock-provider")]
        {
            if let Ok(base) = std::env::var("HOWTUI_TEST_AUTH_BASE") {
                e.issuer = base.clone();
                e.authorize = format!("{base}/api/accounts/authorize");
                e.token = format!("{base}/api/accounts/oauth/token");
                e.revoke = format!("{base}/api/accounts/oauth/revoke");
                e.jwks = format!("{base}/.well-known/jwks.json");
            }
            if let Ok(base) = std::env::var("HOWTUI_TEST_API_BASE") {
                e.api_base = base;
            }
        }
        e
    }
}

pub struct AuthorizeParams<'a> {
    pub client_id: &'a str,
    pub new_registration: bool,
    pub host_id: &'a str,
    pub redirect_uri: &'a str,
    pub state: &'a str,
    pub nonce: &'a str,
    pub challenge: &'a str,
    pub id_token_hint: Option<&'a str>,
    pub prompt_consent: bool,
}

pub fn authorize_url(endpoints: &Endpoints, p: &AuthorizeParams) -> String {
    let mut q = url::form_urlencoded::Serializer::new(String::new());
    q.append_pair("client_id", p.client_id);
    if p.new_registration {
        q.append_pair("agent_name_hint", AGENT_NAME);
    }
    q.append_pair("ext_agent_host_id", p.host_id);
    if let Some(hint) = p.id_token_hint {
        q.append_pair("id_token_hint", hint);
    }
    q.append_pair("response_type", "code");
    q.append_pair("redirect_uri", p.redirect_uri);
    q.append_pair("scope", SCOPE);
    q.append_pair("resource", RESOURCE);
    q.append_pair("state", p.state);
    q.append_pair("nonce", p.nonce);
    q.append_pair("code_challenge_method", "S256");
    q.append_pair("code_challenge", p.challenge);
    if p.prompt_consent {
        q.append_pair("prompt", "consent");
    }
    format!("{}?{}", endpoints.authorize, q.finish())
}

#[derive(Clone, Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub id_token: Option<String>,
    #[serde(default)]
    pub token_type: Option<String>,
    #[serde(default)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub earliest_refresh_at: Option<Value>,
}

impl TokenResponse {
    pub fn scopes(&self) -> Vec<String> {
        self.scope
            .as_deref()
            .unwrap_or("")
            .split_whitespace()
            .map(str::to_string)
            .collect()
    }

    /// 형식 미정 필드: 큰 수는 절대 시각(초), 작은 수는 상대 초로 해석.
    pub fn earliest_refresh(&self, now: u64) -> Option<u64> {
        let v = match self.earliest_refresh_at.as_ref()? {
            Value::Number(n) => n.as_f64()?,
            Value::String(s) => s.trim().parse::<f64>().ok()?,
            _ => return None,
        };
        let v = v as u64;
        Some(if v > 1_000_000_000 { v } else { now + v })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("rejected: {code}")]
    Rejected { code: String, status: u16 },
    #[error("http status {status}")]
    Http { status: u16 },
    #[error("network: {0}")]
    Network(String),
    #[error("parse: {0}")]
    Parse(String),
}

impl OAuthError {
    /// 갱신 종결 오류 (토큰 삭제 후 재로그인 대상, research 8.4절).
    pub fn is_terminal_refresh(&self) -> bool {
        matches!(self, OAuthError::Rejected { code, .. } if matches!(code.as_str(),
            "invalid_grant" | "invalid_refresh_token" | "token_expired" | "refresh_token_expired" | "refresh_token_invalidated" | "refresh_token_reused"))
    }

    pub fn is_invalid_client(&self) -> bool {
        matches!(self, OAuthError::Rejected { code, .. } if code == "invalid_client")
    }

    pub fn is_transient(&self) -> bool {
        matches!(self, OAuthError::Network(_))
            || matches!(self, OAuthError::Http { status } if *status >= 500 || *status == 429)
    }
}

fn form(pairs: &[(&str, &str)]) -> String {
    let mut s = url::form_urlencoded::Serializer::new(String::new());
    for (k, v) in pairs {
        s.append_pair(k, v);
    }
    s.finish()
}

async fn post_form(
    http: &reqwest::Client,
    url: &str,
    pairs: &[(&str, &str)],
) -> Result<(u16, String), OAuthError> {
    let resp = http
        .post(url)
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .header(reqwest::header::ACCEPT, "application/json")
        .body(form(pairs))
        .send()
        .await
        .map_err(|e| OAuthError::Network(e.without_url().to_string()))?;
    let status = resp.status().as_u16();
    let body = resp
        .text()
        .await
        .map_err(|e| OAuthError::Network(e.without_url().to_string()))?;
    Ok((status, body))
}

fn token_result(status: u16, body: &str) -> Result<TokenResponse, OAuthError> {
    if (200..300).contains(&status) {
        return serde_json::from_str(body).map_err(|e| OAuthError::Parse(e.to_string()));
    }
    let code = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v.get("error").and_then(|e| e.as_str().map(str::to_string)));
    match code {
        Some(code) => Err(OAuthError::Rejected { code, status }),
        None => Err(OAuthError::Http { status }),
    }
}

pub async fn exchange_code(
    http: &reqwest::Client,
    e: &Endpoints,
    client_id: &str,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, OAuthError> {
    let (status, body) = post_form(
        http,
        &e.token,
        &[
            ("grant_type", "authorization_code"),
            ("client_id", client_id),
            ("code", code),
            ("code_verifier", verifier),
            ("redirect_uri", redirect_uri),
            ("resource", RESOURCE),
        ],
    )
    .await?;
    token_result(status, &body)
}

pub async fn refresh(
    http: &reqwest::Client,
    e: &Endpoints,
    client_id: &str,
    refresh_token: &str,
) -> Result<TokenResponse, OAuthError> {
    let (status, body) = post_form(
        http,
        &e.token,
        &[
            ("grant_type", "refresh_token"),
            ("client_id", client_id),
            ("refresh_token", refresh_token),
            ("resource", RESOURCE),
        ],
    )
    .await?;
    token_result(status, &body)
}

pub async fn revoke(
    http: &reqwest::Client,
    e: &Endpoints,
    client_id: &str,
    refresh_token: &str,
) -> Result<(), OAuthError> {
    let (status, _) = post_form(
        http,
        &e.revoke,
        &[
            ("token", refresh_token),
            ("token_type_hint", "refresh_token"),
            ("client_id", client_id),
        ],
    )
    .await?;
    if (200..300).contains(&status) {
        Ok(())
    } else {
        Err(OAuthError::Http { status })
    }
}

pub async fn fetch_jwks(http: &reqwest::Client, e: &Endpoints) -> Result<Vec<Jwk>, OAuthError> {
    #[derive(Deserialize)]
    struct Set {
        keys: Vec<Jwk>,
    }
    let resp = http
        .get(&e.jwks)
        .send()
        .await
        .map_err(|e| OAuthError::Network(e.without_url().to_string()))?;
    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(OAuthError::Http { status });
    }
    let set: Set = resp
        .json()
        .await
        .map_err(|e| OAuthError::Parse(e.to_string()))?;
    Ok(set.keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(new_registration: bool) -> AuthorizeParams<'static> {
        AuthorizeParams {
            client_id: if new_registration {
                DYNAMIC_CLIENT
            } else {
                "oaiapp_1"
            },
            new_registration,
            host_id: "urn:uuid:1",
            redirect_uri: "http://127.0.0.1:1455/auth/callback",
            state: "st",
            nonce: "no",
            challenge: "ch",
            id_token_hint: None,
            prompt_consent: false,
        }
    }

    fn query(url: &str) -> Vec<(String, String)> {
        url::Url::parse(url)
            .unwrap()
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect()
    }

    #[test]
    fn first_registration_url() {
        let q = query(&authorize_url(&Endpoints::openai(), &params(true)));
        let get = |k: &str| q.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
        assert_eq!(get("client_id"), Some(DYNAMIC_CLIENT));
        assert_eq!(get("agent_name_hint"), Some("howtui"));
        assert_eq!(get("ext_agent_host_id"), Some("urn:uuid:1"));
        assert_eq!(get("response_type"), Some("code"));
        assert_eq!(
            get("redirect_uri"),
            Some("http://127.0.0.1:1455/auth/callback")
        );
        assert_eq!(get("scope"), Some(SCOPE));
        assert_eq!(get("resource"), Some(RESOURCE));
        assert_eq!(get("code_challenge_method"), Some("S256"));
        assert_eq!(get("prompt"), None);
    }

    #[test]
    fn reauthorization_url_omits_name_hint() {
        let mut p = params(false);
        p.id_token_hint = Some("idt");
        p.prompt_consent = true;
        let q = query(&authorize_url(&Endpoints::openai(), &p));
        assert!(!q.iter().any(|(k, _)| k == "agent_name_hint"));
        assert!(q.contains(&("client_id".into(), "oaiapp_1".into())));
        assert!(q.contains(&("id_token_hint".into(), "idt".into())));
        assert!(q.contains(&("prompt".into(), "consent".into())));
    }

    #[test]
    fn token_errors_are_classified() {
        let e = token_result(400, r#"{"error":"refresh_token_reused"}"#).unwrap_err();
        assert!(e.is_terminal_refresh());
        let e = token_result(401, r#"{"error":"invalid_client"}"#).unwrap_err();
        assert!(e.is_invalid_client());
        let e = token_result(503, "oops").unwrap_err();
        assert!(e.is_transient());
    }

    #[test]
    fn earliest_refresh_parsing() {
        let mut t: TokenResponse =
            serde_json::from_str(r#"{"access_token":"a","earliest_refresh_at":1800000000}"#)
                .unwrap();
        assert_eq!(t.earliest_refresh(100), Some(1_800_000_000));
        t.earliest_refresh_at = Some(serde_json::json!(30));
        assert_eq!(t.earliest_refresh(100), Some(130));
        t.earliest_refresh_at = Some(serde_json::json!("x"));
        assert_eq!(t.earliest_refresh(100), None);
    }
}
