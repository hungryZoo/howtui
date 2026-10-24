//! LLM Provider 계층 (SRS 5절).

pub mod api;
pub mod chatgpt;
pub mod http;
pub mod keys;
#[cfg(feature = "mock-provider")]
pub mod mock;
pub mod parse;
pub mod prompt;
pub mod sse;
pub mod switch;
pub mod types;

use types::{CancelToken, LlmError, LlmRequest, LlmResponse, Provider};

/// 응답 형식 오류 시 1회 재시도 (PRD-LLM-010, SRS-LLM-003).
pub async fn complete_with_retry(
    provider: &dyn Provider,
    req: LlmRequest,
    cancel: CancelToken,
) -> Result<LlmResponse, LlmError> {
    match provider.complete(req.clone(), cancel.clone()).await {
        Err(LlmError::InvalidResponse { .. }) if !cancel.is_cancelled() => {
            provider.complete(req, cancel).await
        }
        other => other,
    }
}
