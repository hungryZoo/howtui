//! API Key Provider 4종: OpenAI, Anthropic, Gemini, OpenAI 호환 (P5, SRS 5.5절)와
//! 로컬 서버 Ollama, llama-server (P6, SRS 5.6절).
//!
//! 요청 본문, 응답 추출, 오류 매핑은 순수 함수로 두고 단위 시험한다. 키 값은 헤더에만 넣는다.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

use super::parse::parse_response;
use super::prompt;
use super::types::*;
use crate::auth::AuthManager;
use crate::auth::secret::Secret;
use crate::auth::state::ModelInfo;
use crate::config::{ApiProviderConfig, ProviderKind};

const TOTAL_TIMEOUT: Duration = Duration::from_secs(120);
/// 로컬 서버 전체 제한 시간 (CPU 실행 고려, SRS 5.6절)
const LOCAL_TIMEOUT: Duration = Duration::from_secs(300);
const ANTHROPIC_VERSION: &str = "2026-10-21";
const ANTHROPIC_FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
/// 응답 상한 (적응형 사고 토큰 포함)
const ANTHROPIC_MAX_TOKENS: u32 = 16000;
/// 로컬, OpenAI 호환 서버 응답 상한 (반복 생성으로 제한 시간까지 멈추지 않는 경우 방지, 평가 TC-LLM-021 발견)
const SERVER_MAX_TOKENS: u32 = 1024;
/// 목록 페이지 조회 상한
const MAX_PAGES: usize = 5;

/// Provider별 기본 주소
pub fn default_base(kind: ProviderKind) -> Option<&'static str> {
    match kind {
        ProviderKind::Openai => Some("https://api.openai.com/v1"),
        ProviderKind::Anthropic => Some("https://api.anthropic.com/v1"),
        ProviderKind::Gemini => Some("https://generativelanguage.googleapis.com/v1beta"),
        ProviderKind::Ollama => Some("http://127.0.0.1:11434/v1"),
        ProviderKind::LlamaServer => Some("http://127.0.0.1:9931/v1"),
        ProviderKind::Chatgpt | ProviderKind::OpenaiCompatible => None,
    }
}

/// 주소 후보: 설정 `base_url` 또는 기본 주소. llama-server는 b11799 이전 기본 포트 8080 추가 (SRS 5.6절)
pub fn base_candidates(kind: ProviderKind, cfg: &ApiProviderConfig) -> Vec<String> {
    let clean = |s: &str| s.trim_end_matches('/').to_string();
    if let Some(url) = &cfg.base_url {
        return vec![clean(url)];
    }
    match kind {
        ProviderKind::LlamaServer => vec![
            "http://127.0.0.1:9931/v1".into(),
            "http://127.0.0.1:8080/v1".into(),
        ],
        _ => default_base(kind).map(clean).into_iter().collect(),
    }
}

/// 목록에 없어도 쓰는 기본 모델 (SRS 5.5절 표)
pub fn default_model(kind: ProviderKind) -> Option<&'static str> {
    match kind {
        ProviderKind::Openai => Some("gpt-6-astra"),
        ProviderKind::Anthropic => Some("claude-opus-5-5"),
        ProviderKind::Gemini => Some("gemini-3.8-flash"),
        _ => None,
    }
}

/// 거절 시 서버 측 대체 모델(`fallbacks: "default"`)을 받는 Anthropic 모델
const ANTHROPIC_FALLBACK_MODELS: &[&str] = &[
    "claude-fable-5-1",
    "claude-opus-5-5",
    "claude-opus-5",
    "claude-sonnet-5-5",
];
/// `output_config.effort`를 받는 Anthropic 모델
const ANTHROPIC_EFFORT_MODELS: &[&str] = &[
    "claude-fable-5-1",
    "claude-fable-5",
    "claude-opus-5-5",
    "claude-opus-5",
    "claude-opus-4-8",
    "claude-opus-4-7",
    "claude-opus-4-6",
    "claude-sonnet-5-5",
    "claude-sonnet-5",
    "claude-sonnet-4-6",
    "claude-haiku-5-5",
];

/// OpenAI 모델 목록에서 제외하는 텍스트 생성 외 모델
const OPENAI_NON_TEXT: &[&str] = &[
    "embedding",
    "tts",
    "whisper",
    "dall-e",
    "image",
    "audio",
    "realtime",
    "transcribe",
    "moderation",
    "search",
    "sora",
];

pub struct ApiProvider {
    kind: ProviderKind,
    http: reqwest::Client,
    /// 주소 후보와 현재 사용 위치 (연결 실패 시 다음 후보)
    bases: Vec<String>,
    active: AtomicUsize,
    key: Option<Secret>,
    key_env: Option<String>,
    model_pref: Option<String>,
    /// 설정 화면 선택 모델 저장 (상태 파일 `models_by_provider`)
    state: Option<Arc<AuthManager>>,
    model: Mutex<Option<String>>,
    models: Mutex<Option<Vec<ModelInfo>>>,
    structured: AtomicBool,
    timeout: Duration,
}

impl ApiProvider {
    pub fn new(
        kind: ProviderKind,
        http: reqwest::Client,
        cfg: &ApiProviderConfig,
        key: Option<Secret>,
        state: Option<Arc<AuthManager>>,
    ) -> Self {
        Self {
            kind,
            http,
            bases: base_candidates(kind, cfg),
            active: AtomicUsize::new(0),
            key,
            key_env: super::keys::env_name(kind, cfg),
            model_pref: cfg.model.clone(),
            state,
            model: Mutex::new(None),
            models: Mutex::new(None),
            structured: AtomicBool::new(kind != ProviderKind::OpenaiCompatible),
            timeout: if kind.is_local() {
                LOCAL_TIMEOUT
            } else {
                TOTAL_TIMEOUT
            },
        }
    }

    /// 시험용 주소 재정의
    pub fn with_base(self, base: impl Into<String>) -> Self {
        self.with_bases(vec![base.into()])
    }

    pub fn with_bases(mut self, bases: Vec<String>) -> Self {
        self.bases = bases
            .into_iter()
            .map(|b| b.trim_end_matches('/').to_string())
            .collect();
        self.active = AtomicUsize::new(0);
        self
    }

    #[cfg(test)]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn kind(&self) -> ProviderKind {
        self.kind
    }

    /// 현재 주소
    pub fn base(&self) -> String {
        let i = self.active.load(Ordering::Relaxed);
        self.bases.get(i).cloned().unwrap_or_default()
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base())
    }

    /// 다음 주소 후보로 이동 (없으면 false)
    fn advance(&self) -> bool {
        let i = self.active.load(Ordering::Relaxed);
        if i + 1 < self.bases.len() {
            self.active.store(i + 1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// 연결 실패 분류: 주소 지정 서버(로컬, OpenAI 호환)는 `LocalUnreachable` (SRS-LLM-021)
    fn net_err(&self, e: reqwest::Error) -> LlmError {
        let server = self.kind.is_local() || self.kind == ProviderKind::OpenaiCompatible;
        if server && e.is_connect() {
            return LlmError::LocalUnreachable {
                provider: self.kind.display().into(),
                url: self.base(),
            };
        }
        network(e)
    }

    /// 요청 전송. 연결 실패 시 남은 주소 후보로 재시도
    async fn send_with(
        &self,
        make: impl Fn(&Self) -> reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, LlmError> {
        loop {
            match make(self).send().await {
                Ok(r) => return Ok(r),
                Err(e) if e.is_connect() && self.advance() => {
                    tracing::debug!(provider = self.kind.id(), "trying next address");
                }
                Err(e) => return Err(self.net_err(e)),
            }
        }
    }

    fn missing(&self) -> Option<LlmError> {
        if self.kind == ProviderKind::OpenaiCompatible {
            return self.base().is_empty().then(|| LlmError::Unsupported {
                param: Some("provider.openai_compatible.base_url".into()),
            });
        }
        if self.kind.is_local() {
            return None;
        }
        self.key.is_none().then(|| LlmError::NoApiKey {
            provider: self.kind.id().into(),
            env: self.key_env.clone(),
        })
    }

    fn authed(&self, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let Some(key) = &self.key else {
            return rb;
        };
        // 민감 헤더 표시: HTTP 계층 로그와 Debug 출력에서 값 제외
        let value = |v: String| {
            let mut h = reqwest::header::HeaderValue::from_str(&v)
                .unwrap_or_else(|_| reqwest::header::HeaderValue::from_static(""));
            h.set_sensitive(true);
            h
        };
        match self.kind {
            ProviderKind::Anthropic => rb
                .header("x-api-key", value(key.expose().to_string()))
                .header("anthropic-version", ANTHROPIC_VERSION),
            ProviderKind::Gemini => rb.header("x-goog-api-key", value(key.expose().to_string())),
            _ => rb.header(
                reqwest::header::AUTHORIZATION,
                value(format!("Bearer {}", key.expose())),
            ),
        }
    }

    async fn get_json(&self, path: &str) -> Result<Value, LlmError> {
        let resp = self
            .send_with(|p| {
                let rb = p.authed(p.http.get(p.url(path)));
                if p.kind == ProviderKind::Anthropic && p.key.is_none() {
                    rb.header("anthropic-version", ANTHROPIC_VERSION)
                } else {
                    rb
                }
            })
            .await?;
        read_json(self.kind, resp).await
    }

    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
        let mut out = Vec::new();
        let mut page: Option<String> = None;
        for _ in 0..MAX_PAGES {
            let path = match (self.kind, &page) {
                (ProviderKind::Anthropic, None) => "/models?limit=100".to_string(),
                (ProviderKind::Anthropic, Some(after)) => {
                    format!("/models?limit=100&after_id={after}")
                }
                (ProviderKind::Gemini, None) => "/models?pageSize=1000".to_string(),
                (ProviderKind::Gemini, Some(token)) => {
                    format!("/models?pageSize=1000&pageToken={token}")
                }
                _ => "/models".to_string(),
            };
            let v = self.get_json(&path).await?;
            out.extend(models_from(self.kind, &v));
            page = next_page(self.kind, &v);
            if page.is_none() {
                break;
            }
        }
        Ok(out)
    }

    async fn cached_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
        if let Some(m) = self
            .models
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
        {
            return Ok(m);
        }
        let m = self.list_models().await?;
        *self.models.lock().unwrap_or_else(|e| e.into_inner()) = Some(m.clone());
        Ok(m)
    }

    fn selected(&self) -> Option<String> {
        self.state.as_ref().and_then(|a| {
            a.load_state()
                .models_by_provider
                .get(self.kind.id())
                .cloned()
        })
    }

    /// 선택 순서: 설정 화면 선택 > 설정 파일 > 기본 모델 > 목록 첫 항목 (SRS 5.5절).
    /// 목록 조회 실패 시 선택값, 설정값, 기본 모델 순서로 목록 없이 사용.
    async fn ensure_model(&self) -> Result<String, LlmError> {
        if let Some(m) = self.model.lock().unwrap_or_else(|e| e.into_inner()).clone() {
            return Ok(m);
        }
        let selected = self.selected();
        let chosen = match self.cached_models().await {
            Ok(models) => choose_model(
                &models,
                selected.as_deref(),
                self.model_pref.as_deref(),
                default_model(self.kind),
            ),
            Err(e) => {
                tracing::debug!(provider = self.kind.id(), error = %e, "model list failed");
                match selected
                    .or_else(|| self.model_pref.clone())
                    .or_else(|| default_model(self.kind).map(str::to_string))
                {
                    Some(m) => Some(m),
                    None => return Err(e),
                }
            }
        }
        .ok_or(LlmError::ModelNotFound)?;
        *self.model.lock().unwrap_or_else(|e| e.into_inner()) = Some(chosen.clone());
        Ok(chosen)
    }

    fn infer_path(&self, model: &str) -> String {
        match self.kind {
            ProviderKind::Openai => self.url("/responses"),
            ProviderKind::Anthropic => self.url("/messages"),
            ProviderKind::Gemini => self.url(&format!("/models/{model}:generateContent")),
            _ => self.url("/chat/completions"),
        }
    }

    async fn send(
        &self,
        req: &LlmRequest,
        model: &str,
        structured: bool,
    ) -> Result<String, LlmError> {
        let body = body(self.kind, req, model, structured);
        let resp = self
            .send_with(|p| {
                let rb = p.authed(p.http.post(p.infer_path(model))).json(&body);
                if p.kind == ProviderKind::Anthropic && body.get("fallbacks").is_some() {
                    rb.header("anthropic-beta", ANTHROPIC_FALLBACK_BETA)
                } else {
                    rb
                }
            })
            .await?;
        let status = resp.status().as_u16();
        tracing::debug!(
            provider = self.kind.id(),
            status,
            structured,
            model,
            "api status"
        );
        if !(200..300).contains(&status) {
            let text = resp.text().await.unwrap_or_default();
            if structured && structured_rejected(status, &text) {
                return Err(LlmError::Unsupported {
                    param: Some(STRUCTURED_PARAM.into()),
                });
            }
            return Err(map_error(self.kind, status, &text));
        }
        let v = read_json(self.kind, resp).await?;
        extract(self.kind, &v)
    }

    async fn complete_inner(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        if let Some(e) = self.missing() {
            return Err(e);
        }
        let model = self.ensure_model().await?;
        let mut structured = self.structured.load(Ordering::Relaxed);
        loop {
            match self.send(&req, &model, structured).await {
                Ok(text) => return parse_response(&text, req.op, &model),
                // 구조화 출력 거부: 지시문 방식으로 1회 재요청 후 실행 중 기억 (SRS-LLM-016)
                Err(LlmError::Unsupported { param: Some(p) })
                    if structured && p == STRUCTURED_PARAM =>
                {
                    structured = false;
                    self.structured.store(false, Ordering::Relaxed);
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn account(&self) -> Option<String> {
        (self.kind == ProviderKind::OpenaiCompatible || self.kind.is_local()).then(|| self.base())
    }
}

const STRUCTURED_PARAM: &str = "structured output";

impl Provider for ApiProvider {
    fn label(&self) -> ProviderLabel {
        let model = self
            .model
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .or_else(|| self.selected())
            .or_else(|| self.model_pref.clone())
            .or_else(|| default_model(self.kind).map(str::to_string));
        ProviderLabel {
            model,
            chatgpt_plan: false,
        }
    }

    fn complete(&self, req: LlmRequest, cancel: CancelToken) -> LlmFuture<'_> {
        Box::pin(async move {
            tokio::select! {
                r = tokio::time::timeout(self.timeout, self.complete_inner(req)) => r.unwrap_or(Err(LlmError::Timeout)),
                _ = cancel.cancelled() => Err(LlmError::Cancelled),
            }
        })
    }

    fn prepare(&self) -> PrepareFuture<'_> {
        Box::pin(async move {
            if let Some(e) = self.missing() {
                return Err(e);
            }
            self.ensure_model().await?;
            Ok(self.label())
        })
    }

    fn settings(&self) -> SettingsFuture<'_> {
        Box::pin(async move {
            let mut data = ProviderSettings {
                agent: self.kind.display().into(),
                account: self.account(),
                ..Default::default()
            };
            if let Some(e) = self.missing() {
                data.problem = Some(e);
                return Ok(data);
            }
            match self.cached_models().await {
                Ok(models) => {
                    data.models = models
                        .into_iter()
                        .map(|m| ModelChoice {
                            name: m.display_name.unwrap_or_else(|| m.slug.clone()),
                            id: m.slug,
                        })
                        .collect();
                }
                Err(e) => data.problem = Some(e),
            }
            data.current = self.ensure_model().await.ok();
            Ok(data)
        })
    }

    fn select_model(&self, id: String) -> PrepareFuture<'_> {
        Box::pin(async move {
            if let Ok(models) = self.cached_models().await
                && !models.iter().any(|m| m.slug == id)
            {
                return Err(LlmError::ModelNotFound);
            }
            if let Some(auth) = &self.state {
                let mut s = auth.load_state();
                s.models_by_provider
                    .insert(self.kind.id().to_string(), id.clone());
                auth.save_state(&s);
            }
            *self.model.lock().unwrap_or_else(|e| e.into_inner()) = Some(id);
            Ok(self.label())
        })
    }
}

/// 설정 화면 선택 > 설정 파일 > 기본 모델 > 목록 첫 항목. 목록에 없는 선택과 기본 모델은 건너뜀,
/// 설정 파일 값은 목록에 없어도 사용 (목록에 없는 배포 모델 대비).
pub fn choose_model(
    models: &[ModelInfo],
    selected: Option<&str>,
    preference: Option<&str>,
    default: Option<&str>,
) -> Option<String> {
    let listed = |id: &str| models.iter().any(|m| m.slug == id);
    if let Some(s) = selected
        && listed(s)
    {
        return Some(s.to_string());
    }
    if let Some(p) = preference {
        return Some(p.to_string());
    }
    if let Some(d) = default
        && (models.is_empty() || listed(d))
    {
        return Some(d.to_string());
    }
    models.first().map(|m| m.slug.clone())
}

fn models_from(kind: ProviderKind, v: &Value) -> Vec<ModelInfo> {
    let list = |key: &str| {
        v.get(key)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    match kind {
        ProviderKind::Gemini => list("models")
            .iter()
            .filter(|m| {
                m.get("supportedGenerationMethods")
                    .and_then(Value::as_array)
                    .is_some_and(|a| a.iter().any(|x| x == "generateContent"))
            })
            .filter_map(|m| {
                let name = m.get("name").and_then(Value::as_str)?;
                Some(ModelInfo {
                    slug: name.trim_start_matches("models/").to_string(),
                    display_name: m
                        .get("displayName")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                })
            })
            .collect(),
        _ => list("data")
            .iter()
            .filter_map(|m| {
                let id = m.get("id").and_then(Value::as_str)?;
                if kind == ProviderKind::Openai && OPENAI_NON_TEXT.iter().any(|x| id.contains(x)) {
                    return None;
                }
                Some(ModelInfo {
                    slug: id.to_string(),
                    display_name: m
                        .get("display_name")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                })
            })
            .collect(),
    }
}

fn next_page(kind: ProviderKind, v: &Value) -> Option<String> {
    match kind {
        ProviderKind::Anthropic => v
            .get("has_more")
            .and_then(Value::as_bool)
            .filter(|m| *m)
            .and_then(|_| v.get("last_id").and_then(Value::as_str))
            .map(str::to_string),
        ProviderKind::Gemini => v
            .get("nextPageToken")
            .and_then(Value::as_str)
            .filter(|t| !t.is_empty())
            .map(str::to_string),
        _ => None,
    }
}

/// 요청 본문 (SRS 5.5절 표). 전송 필드는 `prompt::build` 결과와 모델 이름만 사용.
pub fn body(kind: ProviderKind, req: &LlmRequest, model: &str, structured: bool) -> Value {
    let p = prompt::build(req);
    let turns = |assistant: &str| -> Vec<Value> {
        p.messages
            .iter()
            .map(|(role, text)| {
                let r = match role {
                    prompt::Role::User => "user",
                    prompt::Role::Assistant => assistant,
                };
                json!({ "role": r, "content": text })
            })
            .collect()
    };
    let schema = prompt::response_schema();
    match kind {
        ProviderKind::Openai => {
            let mut b = json!({
                "model": model,
                "instructions": p.instructions,
                "input": turns("assistant"),
                "store": false,
            });
            if structured {
                b["text"] = json!({ "format": { "type": "json_schema", "name": "howtui_response", "strict": true, "schema": schema } });
            }
            b
        }
        ProviderKind::Anthropic => {
            let mut b = json!({
                "model": model,
                "max_tokens": ANTHROPIC_MAX_TOKENS,
                "system": p.instructions,
                "messages": turns("assistant"),
            });
            let mut output = serde_json::Map::new();
            if ANTHROPIC_EFFORT_MODELS.contains(&model) {
                // 짧은 명령어 생성: 낮은 사고 수준으로 지연과 비용 축소
                output.insert("effort".into(), json!("low"));
            }
            if structured {
                output.insert(
                    "format".into(),
                    json!({ "type": "json_schema", "schema": schema }),
                );
            }
            if !output.is_empty() {
                b["output_config"] = Value::Object(output);
            }
            if ANTHROPIC_FALLBACK_MODELS.contains(&model) {
                b["fallbacks"] = json!("default");
            }
            b
        }
        ProviderKind::Gemini => {
            let contents: Vec<Value> = p
                .messages
                .iter()
                .map(|(role, text)| {
                    let r = match role {
                        prompt::Role::User => "user",
                        prompt::Role::Assistant => "model",
                    };
                    json!({ "role": r, "parts": [{ "text": text }] })
                })
                .collect();
            let mut b = json!({
                "systemInstruction": { "parts": [{ "text": p.instructions }] },
                "contents": contents,
            });
            if structured {
                b["generationConfig"] = json!({ "responseMimeType": "application/json" });
            }
            b
        }
        ProviderKind::OpenaiCompatible | ProviderKind::Chatgpt => {
            let mut messages = vec![json!({ "role": "system", "content": p.instructions })];
            messages.extend(turns("assistant"));
            json!({ "model": model, "messages": messages, "stream": false, "max_tokens": SERVER_MAX_TOKENS })
        }
        ProviderKind::Ollama | ProviderKind::LlamaServer => {
            let mut messages = vec![json!({ "role": "system", "content": p.instructions })];
            messages.extend(turns("assistant"));
            // 사고 모드 끔 (지연 축소), 구조화 출력은 OpenAI 중첩 형식만 (research 4.3절)
            let mut b = json!({
                "model": model,
                "messages": messages,
                "stream": false,
                "reasoning_effort": "none",
                "max_tokens": SERVER_MAX_TOKENS,
            });
            if structured {
                b["response_format"] = json!({ "type": "json_schema", "json_schema": { "name": "howtui_response", "schema": schema } });
            }
            b
        }
    }
}

/// 응답 JSON에서 모델 출력 텍스트 추출. 거절과 차단은 `Refused`.
pub fn extract(kind: ProviderKind, v: &Value) -> Result<String, LlmError> {
    let invalid = |detail: &str| LlmError::InvalidResponse {
        detail: detail.into(),
    };
    let mut out = String::new();
    match kind {
        ProviderKind::Openai => {
            if v.get("status").and_then(Value::as_str) == Some("incomplete") {
                return Err(invalid("response incomplete"));
            }
            for item in v
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
                    match part.get("type").and_then(Value::as_str) {
                        Some("output_text") => {
                            out.push_str(part.get("text").and_then(Value::as_str).unwrap_or(""))
                        }
                        Some("refusal") => return Err(LlmError::Refused),
                        _ => {}
                    }
                }
            }
        }
        ProviderKind::Anthropic => {
            match v.get("stop_reason").and_then(Value::as_str) {
                Some("refusal") => return Err(LlmError::Refused),
                Some("max_tokens") => return Err(invalid("max_tokens reached")),
                _ => {}
            }
            for block in v
                .get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if block.get("type").and_then(Value::as_str) == Some("text") {
                    out.push_str(block.get("text").and_then(Value::as_str).unwrap_or(""));
                }
            }
        }
        ProviderKind::Gemini => {
            if v.pointer("/promptFeedback/blockReason").is_some() {
                return Err(LlmError::Refused);
            }
            let Some(candidate) = v.pointer("/candidates/0") else {
                return Err(invalid("no candidates"));
            };
            match candidate.get("finishReason").and_then(Value::as_str) {
                Some("SAFETY" | "PROHIBITED_CONTENT" | "BLOCKLIST" | "SPII" | "RECITATION") => {
                    return Err(LlmError::Refused);
                }
                Some("MAX_TOKENS") => return Err(invalid("max tokens reached")),
                _ => {}
            }
            for part in candidate
                .pointer("/content/parts")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if part.get("thought").and_then(Value::as_bool) == Some(true) {
                    continue;
                }
                out.push_str(part.get("text").and_then(Value::as_str).unwrap_or(""));
            }
        }
        ProviderKind::OpenaiCompatible
        | ProviderKind::Chatgpt
        | ProviderKind::Ollama
        | ProviderKind::LlamaServer => {
            let choice = v
                .pointer("/choices/0")
                .ok_or_else(|| invalid("no choices"))?;
            match choice.get("finish_reason").and_then(Value::as_str) {
                Some("content_filter") => return Err(LlmError::Refused),
                // 응답 상한 도달: 형식 오류로 분류 (재시도 1회 대상)
                Some("length") => return Err(invalid("max tokens reached")),
                _ => {}
            }
            out.push_str(
                choice
                    .pointer("/message/content")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            );
        }
    }
    if out.trim().is_empty() {
        return Err(invalid("empty output"));
    }
    Ok(out)
}

/// 400 응답이 구조화 출력 관련 항목을 지목하는지 판정.
pub fn structured_rejected(status: u16, body: &str) -> bool {
    if status != 400 && status != 422 {
        return false;
    }
    let lower = body.to_ascii_lowercase();
    [
        "output_config",
        "text.format",
        "response_format",
        "responsemimetype",
        "json_schema",
        "schema",
    ]
    .iter()
    .any(|k| lower.contains(k))
}

fn short(text: &str) -> String {
    let t: String = text.chars().take(160).collect();
    if t.len() < text.len() {
        format!("{t}…")
    } else {
        t
    }
}

/// HTTP 오류 응답 매핑 (SRS 5.5절).
pub fn map_error(kind: ProviderKind, status: u16, body: &str) -> LlmError {
    let v: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let err = v.get("error").cloned().unwrap_or(Value::Null);
    let code = err
        .get("code")
        .and_then(Value::as_str)
        .or_else(|| err.get("type").and_then(Value::as_str))
        .or_else(|| err.get("status").and_then(Value::as_str))
        .unwrap_or("");
    let message = err
        .get("message")
        .and_then(Value::as_str)
        .or_else(|| v.get("message").and_then(Value::as_str))
        .unwrap_or("");
    let lower = message.to_ascii_lowercase();
    let rejected = || LlmError::KeyRejected {
        provider: kind.id().into(),
    };
    if matches!(
        code,
        "insufficient_quota" | "billing_error" | "billing_hard_limit_reached"
    ) || lower.contains("credit balance")
    {
        return LlmError::QuotaExceeded;
    }
    if matches!(code, "invalid_api_key" | "authentication_error")
        || body.contains("API_KEY_INVALID")
    {
        return rejected();
    }
    if code == "model_not_found" || code == "not_found_error" {
        return LlmError::ModelNotFound;
    }
    let detail = if message.is_empty() {
        format!("http {status}")
    } else {
        short(message)
    };
    match status {
        401 => rejected(),
        402 => LlmError::QuotaExceeded,
        403 => LlmError::RegionOrPolicy { detail },
        404 if kind == ProviderKind::OpenaiCompatible || kind.is_local() => LlmError::Unsupported {
            param: Some(detail),
        },
        404 => LlmError::ModelNotFound,
        400 | 422 => LlmError::Unsupported {
            param: Some(detail),
        },
        429 => LlmError::Transient { status: Some(429) },
        s if s >= 500 => LlmError::Transient { status: Some(s) },
        _ => LlmError::InvalidResponse { detail },
    }
}

async fn read_json(kind: ProviderKind, resp: reqwest::Response) -> Result<Value, LlmError> {
    let status = resp.status().as_u16();
    let text = resp.text().await.map_err(network)?;
    if !(200..300).contains(&status) {
        return Err(map_error(kind, status, &text));
    }
    serde_json::from_str(&text).map_err(|e| LlmError::InvalidResponse {
        detail: e.to_string(),
    })
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

#[cfg(test)]
mod tests;
