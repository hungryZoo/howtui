//! Provider 무관 표준 타입 (SRS 5.1절, ADR-0006). 네트워크 crate에 의존하지 않는다.

use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};

use crate::shell::ShellKind;

pub type RequestId = u64;
pub use tokio_util::sync::CancellationToken as CancelToken;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Generate,
    Revise,
    Explain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OsKind {
    Linux,
    MacOs,
    Windows,
    Other,
}

impl OsKind {
    pub fn current() -> OsKind {
        match std::env::consts::OS {
            "linux" => OsKind::Linux,
            "macos" => OsKind::MacOs,
            "windows" => OsKind::Windows,
            _ => OsKind::Other,
        }
    }

    pub fn display(self) -> &'static str {
        match self {
            OsKind::Linux => "Linux",
            OsKind::MacOs => "macOS",
            OsKind::Windows => "Windows",
            OsKind::Other => std::env::consts::OS,
        }
    }
}

/// LLM에 전달하는 환경 정보. 허용 필드 외 추가 금지 (SRS-SEC-004).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvContext {
    pub os: OsKind,
    pub arch: &'static str,
    pub shell: ShellKind,
    pub shell_version: Option<String>,
}

impl EnvContext {
    pub fn current(shell: ShellKind, shell_version: Option<String>) -> EnvContext {
        EnvContext {
            os: OsKind::current(),
            arch: std::env::consts::ARCH,
            shell,
            shell_version,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmResponse {
    pub command: Option<String>,
    pub explanation: String,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(skip)]
    pub model: String,
}

/// 세션 내 대화 이력 1건 (메모리 전용).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Turn {
    pub op: Operation,
    pub question: Option<String>,
    pub command_sent: Option<String>,
    pub response: LlmResponse,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LlmRequest {
    pub id: RequestId,
    pub op: Operation,
    pub env: EnvContext,
    pub question: Option<String>,
    pub command: Option<String>,
    pub history: Vec<Turn>,
    /// 현재 폴더 목록 포함 여부 (사용자 설정, CR-007). 앱 코어가 결정
    pub include_folder: bool,
    /// 현재 폴더 목록 (요청 직전 런타임이 채움, 이름과 종류와 크기만)
    pub folder: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LlmError {
    #[error("authentication required")]
    AuthRequired,
    #[error("authentication failed: {detail}")]
    AuthFailed { detail: String },
    #[error("ChatGPT plan usage not eligible")]
    NotEligible,
    #[error("ChatGPT plan usage not enabled for this app")]
    ClientNotEnabled,
    #[error("ChatGPT plan usage not granted at sign-in")]
    PlanNotGranted,
    #[error("permission configuration error: {code}")]
    PermissionConfig { code: String },
    #[error("blocked by policy or region: {detail}")]
    RegionOrPolicy { detail: String },
    #[error("usage limit reached")]
    UsageLimit,
    #[error("unsupported capability: {param:?}")]
    Unsupported { param: Option<String> },
    #[error("model not found")]
    ModelNotFound,
    #[error("temporarily unavailable (status {status:?})")]
    Transient { status: Option<u16> },
    #[error("network error: {detail}")]
    Network { detail: String },
    #[error("invalid response: {detail}")]
    InvalidResponse { detail: String },
    #[error("timed out")]
    Timeout,
    #[error("cancelled")]
    Cancelled,
    /// API Key Provider 키 없음 (P5, SRS 5.5절)
    #[error("no API key for {provider}")]
    NoApiKey {
        provider: String,
        env: Option<String>,
    },
    /// API 키 거부 (401, 잘못된 키)
    #[error("API key rejected by {provider}")]
    KeyRejected { provider: String },
    /// 계정 사용량, 결제 한도 (API Key Provider)
    #[error("quota or billing limit reached")]
    QuotaExceeded,
    /// 모델 또는 안전 정책의 응답 거절
    #[error("request declined by the model")]
    Refused,
}

impl LlmError {
    /// 비대화형 모드 종료 코드 (SRS 8.2절).
    pub fn exit_code(&self) -> u8 {
        match self {
            LlmError::AuthRequired
            | LlmError::AuthFailed { .. }
            | LlmError::NoApiKey { .. }
            | LlmError::KeyRejected { .. } => 4,
            LlmError::NotEligible
            | LlmError::UsageLimit
            | LlmError::QuotaExceeded
            | LlmError::ClientNotEnabled
            | LlmError::PlanNotGranted => 5,
            LlmError::Transient { .. } | LlmError::Network { .. } | LlmError::Timeout => 6,
            LlmError::Cancelled => 130,
            _ => 7,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProviderLabel {
    pub model: Option<String>,
    /// "Using ChatGPT plan" 표시 여부.
    pub chatgpt_plan: bool,
}

/// 설정 화면의 모델 항목 (CR-005).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelChoice {
    pub id: String,
    pub name: String,
}

/// 설정 화면 에이전트 항목 (P5, CR-011).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentChoice {
    /// 설정 이름 (`chatgpt`, `openai`, ...)
    pub id: String,
    pub name: String,
    /// 사용 가능 여부 (로그인 또는 키, 주소 설정). ChatGPT는 미로그인도 선택 가능
    pub ready: bool,
    /// 상태 설명 (키 출처, 로그인 필요 등)
    pub detail: String,
}

/// 설정 화면 내용: 현재 에이전트(Provider)와 선택 가능한 모델.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProviderSettings {
    pub agent: String,
    /// 마스킹된 계정 표시
    pub account: Option<String>,
    pub models: Vec<ModelChoice>,
    pub current: Option<String>,
    pub can_sign_out: bool,
    /// 전환 가능한 에이전트 목록과 현재 에이전트 설정 이름 (CR-011)
    pub agents: Vec<AgentChoice>,
    pub current_agent: Option<String>,
    /// 현재 에이전트 조회 실패 (목록은 표시하고 오류 줄에 안내)
    pub problem: Option<LlmError>,
}

pub type LlmFuture<'a> = Pin<Box<dyn Future<Output = Result<LlmResponse, LlmError>> + Send + 'a>>;
pub type PrepareFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ProviderLabel, LlmError>> + Send + 'a>>;
pub type SettingsFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ProviderSettings, LlmError>> + Send + 'a>>;

/// dyn 호환을 위해 표준 라이브러리 Box<dyn Future> 반환.
pub trait Provider: Send + Sync {
    fn label(&self) -> ProviderLabel;
    fn complete(&self, req: LlmRequest, cancel: CancelToken) -> LlmFuture<'_>;
    /// 시작 시 준비 작업 (모델 목록 조회 등). 첫 화면 표시와 병행 실행.
    fn prepare(&self) -> PrepareFuture<'_> {
        let label = self.label();
        Box::pin(async move { Ok(label) })
    }
    /// 설정 화면 내용 조회.
    fn settings(&self) -> SettingsFuture<'_>;
    /// 모델 변경 후 저장 (다음 실행에도 유지).
    fn select_model(&self, id: String) -> PrepareFuture<'_>;
    /// 에이전트 전환 후 저장 (CR-011). 단일 Provider는 미지원
    fn select_agent(&self, id: String) -> PrepareFuture<'_> {
        Box::pin(async move { Err(LlmError::Unsupported { param: Some(id) }) })
    }
}
