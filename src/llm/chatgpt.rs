//! ChatGPT 로그인 기반 Provider (SRS 5.4절, research/chatgpt-oauth.md 5절, 8절).
//!
//! 공식 경로만 사용: `GET /v1/models`, `POST /v1/responses` (`store: false`, `stream: true`).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::StreamExt;
use serde_json::{Value, json};

use super::parse::parse_response;
use super::prompt;
use super::sse::{SseEvent, SseParser};
use super::types::*;
use crate::auth::state::{ModelCache, ModelInfo};
use crate::auth::{AuthError, AuthManager, now};

const MODELS_MAX_AGE: u64 = 24 * 3600;
const FIRST_EVENT_TIMEOUT: Duration = Duration::from_secs(60);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(120);

pub struct ChatGptProvider {
    http: reqwest::Client,
    auth: Arc<AuthManager>,
    model_pref: Option<String>,
    model: Mutex<Option<String>>,
}

impl ChatGptProvider {
    pub fn new(http: reqwest::Client, auth: Arc<AuthManager>, model_pref: Option<String>) -> Self {
        Self {
            http,
            auth,
            model_pref,
            model: Mutex::new(None),
        }
    }

    fn api(&self, path: &str) -> String {
        format!(
            "{}{path}",
            self.auth.endpoints.api_base.trim_end_matches('/')
        )
    }

    async fn token(&self) -> Result<String, LlmError> {
        self.auth
            .access_token()
            .await
            .map(|s| s.expose().to_string())
            .map_err(map_auth_error)
    }

    pub async fn list_models(&self, token: &str) -> Result<Vec<ModelInfo>, LlmError> {
        let resp = self
            .http
            .get(self.api("/models"))
            .bearer_auth(token)
            .send()
            .await
            .map_err(network)?;
        let status = resp.status().as_u16();
        let body = resp.text().await.map_err(network)?;
        if !(200..300).contains(&status) {
            return Err(map_http_error(status, &body));
        }
        let v: Value = serde_json::from_str(&body).map_err(|e| LlmError::InvalidResponse {
            detail: e.to_string(),
        })?;
        let models = v
            .get("models")
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .filter(|m| {
                        m.get("visibility")
                            .and_then(Value::as_str)
                            .is_none_or(|vis| vis == "list")
                    })
                    .filter_map(|m| {
                        let slug = m.get("slug").and_then(Value::as_str)?.to_string();
                        let display_name = m
                            .get("display_name")
                            .and_then(Value::as_str)
                            .map(str::to_string);
                        Some(ModelInfo { slug, display_name })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(models)
    }

    /// 설정 모델이 목록에 있으면 사용, 아니면 서버 순서 첫 모델 (PRD-LLM-009).
    pub fn choose_model(models: &[ModelInfo], preference: Option<&str>) -> Option<String> {
        if let Some(p) = preference
            && (models.is_empty() || models.iter().any(|m| m.slug == p))
        {
            return Some(p.to_string());
        }
        models.first().map(|m| m.slug.clone())
    }

    async fn ensure_model(&self, token: &str, refresh: bool) -> Result<String, LlmError> {
        if !refresh && let Some(m) = self.model.lock().unwrap_or_else(|e| e.into_inner()).clone() {
            return Ok(m);
        }
        let mut state = self.auth.load_state();
        let cached = state
            .models
            .as_ref()
            .filter(|c| !refresh && c.fetched_at + MODELS_MAX_AGE > now())
            .map(|c| c.models.clone());
        let models = match cached {
            Some(m) => m,
            None => {
                let m = self.list_models(token).await?;
                state.models = Some(ModelCache {
                    fetched_at: now(),
                    models: m.clone(),
                });
                self.auth.save_state(&state);
                m
            }
        };
        let chosen = Self::choose_model(&models, self.model_pref.as_deref())
            .ok_or(LlmError::ModelNotFound)?;
        *self.model.lock().unwrap_or_else(|e| e.into_inner()) = Some(chosen.clone());
        Ok(chosen)
    }

    fn body(req: &LlmRequest, model: &str, structured: bool) -> Value {
        let p = prompt::build(req);
        let input: Vec<Value> = p
            .messages
            .iter()
            .map(|(role, text)| json!({ "role": role.as_str(), "content": text }))
            .collect();
        let mut body = json!({
            "model": model,
            "instructions": p.instructions,
            "input": input,
            "store": false,
            "stream": true,
        });
        if structured {
            body["text"] = json!({ "format": { "type": "json_schema", "name": "howtui_response", "strict": true, "schema": prompt::response_schema() } });
        }
        body
    }

    async fn responses(
        &self,
        req: &LlmRequest,
        token: &str,
        model: &str,
        structured: bool,
    ) -> Result<String, LlmError> {
        let resp = self
            .http
            .post(self.api("/responses"))
            .bearer_auth(token)
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .json(&Self::body(req, model, structured))
            .send()
            .await
            .map_err(network)?;
        let status = resp.status().as_u16();
        if !(200..300).contains(&status) {
            let body = resp.text().await.unwrap_or_default();
            return Err(map_http_error(status, &body));
        }
        let mut stream = resp.bytes_stream();
        let mut parser = SseParser::new();
        let mut collector = Collector::default();
        let mut first = true;
        loop {
            let next = if first {
                tokio::time::timeout(FIRST_EVENT_TIMEOUT, stream.next())
                    .await
                    .map_err(|_| LlmError::Timeout)?
            } else {
                stream.next().await
            };
            first = false;
            match next {
                Some(Ok(bytes)) => {
                    for ev in parser.feed(&bytes) {
                        if let Some(done) = collector.event(&ev)? {
                            return Ok(done);
                        }
                    }
                }
                Some(Err(e)) => return Err(network(e)),
                None => {
                    for ev in parser.finish() {
                        if let Some(done) = collector.event(&ev)? {
                            return Ok(done);
                        }
                    }
                    // response.completed 없이 종료: 실패 처리 (research 5.2절)
                    return Err(LlmError::Network {
                        detail: "stream ended before response.completed".into(),
                    });
                }
            }
        }
    }

    async fn complete_inner(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        if self.auth.tokens().is_none() {
            return Err(LlmError::AuthRequired);
        }
        if !self.auth.plan_enabled() {
            return Err(LlmError::PlanNotGranted);
        }
        let mut token = self.token().await?;
        let mut model = self.ensure_model(&token, false).await?;
        let mut structured = self.auth.load_state().structured_output != Some(false);
        let mut refreshed = false;
        let mut model_retry = false;
        loop {
            match self.responses(&req, &token, &model, structured).await {
                Ok(text) => {
                    if structured {
                        let mut s = self.auth.load_state();
                        if s.structured_output != Some(true) {
                            s.structured_output = Some(true);
                            self.auth.save_state(&s);
                        }
                    }
                    return parse_response(&text, req.op, &model);
                }
                // 구조화 출력 미지원 시 지시문 방식으로 전환 후 기록
                Err(LlmError::Unsupported { param: Some(p) })
                    if structured && p.starts_with("text") =>
                {
                    structured = false;
                    let mut s = self.auth.load_state();
                    s.structured_output = Some(false);
                    self.auth.save_state(&s);
                }
                Err(LlmError::AuthFailed { .. }) if !refreshed => {
                    refreshed = true;
                    token = self
                        .auth
                        .force_refresh()
                        .await
                        .map(|s| s.expose().to_string())
                        .map_err(map_auth_error)?;
                }
                Err(LlmError::ModelNotFound) if !model_retry => {
                    model_retry = true;
                    model = self.ensure_model(&token, true).await?;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

impl Provider for ChatGptProvider {
    fn label(&self) -> ProviderLabel {
        let model = self
            .model
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .or_else(|| self.model_pref.clone());
        ProviderLabel {
            model,
            chatgpt_plan: self.auth.plan_enabled(),
        }
    }

    fn complete(&self, req: LlmRequest, cancel: CancelToken) -> LlmFuture<'_> {
        Box::pin(async move {
            tokio::select! {
                r = tokio::time::timeout(TOTAL_TIMEOUT, self.complete_inner(req)) => r.unwrap_or(Err(LlmError::Timeout)),
                _ = cancel.cancelled() => Err(LlmError::Cancelled),
            }
        })
    }

    fn prepare(&self) -> PrepareFuture<'_> {
        Box::pin(async move {
            if self.auth.tokens().is_none() {
                return Err(LlmError::AuthRequired);
            }
            if !self.auth.plan_enabled() {
                return Ok(self.label());
            }
            let token = self.token().await?;
            self.ensure_model(&token, false).await?;
            Ok(self.label())
        })
    }
}

#[derive(Default)]
struct Collector {
    text: String,
}

impl Collector {
    /// 완료 시 최종 텍스트 반환, 실패 이벤트는 오류.
    fn event(&mut self, ev: &SseEvent) -> Result<Option<String>, LlmError> {
        if ev.data.trim() == "[DONE]" {
            return Ok(None);
        }
        let Ok(v) = serde_json::from_str::<Value>(&ev.data) else {
            return Ok(None);
        };
        let kind = v
            .get("type")
            .and_then(Value::as_str)
            .or(ev.event.as_deref())
            .unwrap_or("");
        match kind {
            "response.output_text.delta" => {
                if let Some(d) = v.get("delta").and_then(Value::as_str) {
                    self.text.push_str(d);
                }
                Ok(None)
            }
            "response.completed" => {
                let full = v.get("response").map(output_text).filter(|t| !t.is_empty());
                Ok(Some(full.unwrap_or_else(|| std::mem::take(&mut self.text))))
            }
            "response.failed" => {
                let err = v.pointer("/response/error").cloned().unwrap_or(Value::Null);
                Err(map_error_object(&err, None))
            }
            "response.incomplete" => Err(LlmError::InvalidResponse {
                detail: "response incomplete".into(),
            }),
            "error" => Err(map_error_object(v.get("error").unwrap_or(&v), None)),
            _ => Ok(None),
        }
    }
}

fn output_text(response: &Value) -> String {
    let mut out = String::new();
    for item in response
        .get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        for part in item
            .get("content")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if part.get("type").and_then(Value::as_str) == Some("output_text")
                && let Some(t) = part.get("text").and_then(Value::as_str)
            {
                out.push_str(t);
            }
        }
    }
    out
}

fn network(e: reqwest::Error) -> LlmError {
    if e.is_timeout() {
        LlmError::Timeout
    } else {
        LlmError::Network {
            detail: e.without_url().to_string(),
        }
    }
}

pub fn map_auth_error(e: AuthError) -> LlmError {
    match e {
        AuthError::NotLoggedIn | AuthError::Rejected(_) => LlmError::AuthRequired,
        AuthError::InvalidClient => LlmError::PermissionConfig {
            code: "invalid_client".into(),
        },
        AuthError::Network(detail) => LlmError::Network { detail },
        AuthError::Transient(status) => LlmError::Transient {
            status: Some(status),
        },
        AuthError::Storage(detail) => LlmError::AuthFailed { detail },
    }
}

fn map_error_object(err: &Value, status: Option<u16>) -> LlmError {
    let code = err.get("code").and_then(Value::as_str).unwrap_or("");
    let param = err.get("param").and_then(Value::as_str).map(str::to_string);
    let message = err
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    match code {
        "subscription_sharing_user_not_eligible" => LlmError::NotEligible,
        "subscription_sharing_usage_limit_exceeded" => LlmError::UsageLimit,
        "subscription_sharing_usage_unavailable" | "subscription_sharing_user_unavailable" => {
            LlmError::Transient { status: Some(503) }
        }
        "subscription_sharing_unsupported_capability" => LlmError::Unsupported { param },
        "subscription_sharing_route_not_supported"
        | "chatpass_v2_scope_not_authorized"
        | "chatpass_v2_invalid_authorization_context" => LlmError::PermissionConfig {
            code: code.to_string(),
        },
        "subscription_sharing_invalid_user" | "invalid_token" | "invalid_api_key" => {
            LlmError::AuthFailed {
                detail: code.to_string(),
            }
        }
        "subscription_sharing_v2_client_not_enabled" => LlmError::ClientNotEnabled,
        "model_not_found" => LlmError::ModelNotFound,
        _ => match status {
            Some(401) => LlmError::AuthFailed { detail: message },
            Some(403) => LlmError::RegionOrPolicy {
                detail: if message.is_empty() {
                    code.to_string()
                } else {
                    message
                },
            },
            Some(404) => LlmError::ModelNotFound,
            Some(400) => LlmError::Unsupported { param },
            Some(s) if s == 429 || s >= 500 => LlmError::Transient { status: Some(s) },
            _ => LlmError::InvalidResponse {
                detail: if message.is_empty() {
                    code.to_string()
                } else {
                    message
                },
            },
        },
    }
}

/// HTTP 오류 응답 매핑 (research/chatgpt-oauth.md 8.6절).
pub fn map_http_error(status: u16, body: &str) -> LlmError {
    let v: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    if let Some(err) = v.get("error").filter(|e| e.is_object()) {
        return map_error_object(err, Some(status));
    }
    // 스트림 시작 전 진입 오류: {"detail": "..."}
    let detail = v
        .get("detail")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    match status {
        401 => LlmError::AuthFailed { detail },
        403 => LlmError::RegionOrPolicy {
            detail: if detail.is_empty() {
                "forbidden".into()
            } else {
                detail
            },
        },
        404 => LlmError::ModelNotFound,
        429 => LlmError::Transient { status: Some(429) },
        s if s >= 500 => LlmError::Transient { status: Some(s) },
        _ => LlmError::InvalidResponse {
            detail: format!("http {status}"),
        },
    }
}

#[cfg(test)]
mod tests;
