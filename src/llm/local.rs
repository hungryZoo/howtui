//! 로컬 서버 식별과 상태 조회 (P6, SRS 5.6절, SRS-LLM-020). doctor와 시험에서 사용한다.
//!
//! 서버 관리 기능(모델 내려받기, 적재, 해제)은 호출하지 않는다.

use std::time::Duration;

use serde_json::Value;

use super::types::LlmError;
use crate::config::ProviderKind;

const PROBE_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerInfo {
    /// 응답한 주소 (`/v1` 포함)
    pub base: String,
    pub version: Option<String>,
    /// 요청 처리 가능 여부 (llama-server 모델 적재 중이면 false)
    pub ready: bool,
}

/// `/v1` 앞부분 (서버 고유 경로용)
fn root(base: &str) -> &str {
    base.trim_end_matches('/').trim_end_matches("/v1")
}

async fn get(http: &reqwest::Client, url: String) -> Result<(u16, Value), reqwest::Error> {
    let resp = http.get(url).timeout(PROBE_TIMEOUT).send().await?;
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    Ok((status, serde_json::from_str(&text).unwrap_or(Value::Null)))
}

/// 주소 후보를 순서대로 식별. 모두 연결 실패면 `LocalUnreachable`(첫 후보 주소).
pub async fn probe(
    http: &reqwest::Client,
    kind: ProviderKind,
    bases: &[String],
) -> Result<ServerInfo, LlmError> {
    for base in bases {
        let r = match kind {
            ProviderKind::Ollama => {
                get(http, format!("{}/api/version", root(base)))
                    .await
                    .map(|(status, v)| ServerInfo {
                        base: base.clone(),
                        version: v.get("version").and_then(Value::as_str).map(str::to_string),
                        ready: status == 200,
                    })
            }
            _ => match get(http, format!("{}/health", root(base))).await {
                Ok((status, _)) => {
                    let version = get(http, format!("{}/props", root(base)))
                        .await
                        .ok()
                        .and_then(|(_, v)| {
                            v.get("build_info")
                                .and_then(Value::as_str)
                                .map(str::to_string)
                        });
                    Ok(ServerInfo {
                        base: base.clone(),
                        version,
                        ready: status == 200,
                    })
                }
                Err(e) => Err(e),
            },
        };
        match r {
            Ok(info) => return Ok(info),
            Err(e) if e.is_connect() || e.is_timeout() => continue,
            Err(e) => {
                return Err(LlmError::Network {
                    detail: e.without_url().to_string(),
                });
            }
        }
    }
    Err(LlmError::LocalUnreachable {
        provider: kind.display().into(),
        url: bases.first().cloned().unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// 연결 거부 주소 (바인딩 후 즉시 해제한 포트)
    fn closed() -> String {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        drop(l);
        format!("http://127.0.0.1:{port}/v1")
    }

    #[tokio::test]
    async fn ollama_and_llama_server_identification() {
        let http = crate::llm::http::client();
        let ollama = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/version"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"version":"0.40.7"})))
            .mount(&ollama)
            .await;
        let info = probe(
            &http,
            ProviderKind::Ollama,
            &[format!("{}/v1", ollama.uri())],
        )
        .await
        .unwrap();
        assert_eq!(info.version.as_deref(), Some("0.40.7"));
        assert!(info.ready);

        // llama-server: 첫 후보 연결 거부, 둘째 후보 적재 중(503)
        let llama = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(503).set_body_json(
                json!({"error":{"code":503,"message":"Loading model","type":"unavailable_error"}}),
            ))
            .mount(&llama)
            .await;
        Mock::given(method("GET"))
            .and(path("/props"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"build_info":"b11799-abc"})),
            )
            .mount(&llama)
            .await;
        let second = format!("{}/v1", llama.uri());
        let info = probe(
            &http,
            ProviderKind::LlamaServer,
            &[closed(), second.clone()],
        )
        .await
        .unwrap();
        assert_eq!(info.base, second);
        assert!(!info.ready);
        assert_eq!(info.version.as_deref(), Some("b11799-abc"));

        let first = closed();
        assert_eq!(
            probe(&http, ProviderKind::Ollama, std::slice::from_ref(&first)).await,
            Err(LlmError::LocalUnreachable {
                provider: "Ollama".into(),
                url: first
            })
        );
    }
}
