//! 테스트 전용 Mock Provider (SRS-LLM-009). cargo 기능 `mock-provider`에서만 컴파일.
//!
//! `HOWTUI_MOCK_SCENARIO` JSON 파일의 단계를 순서대로 사용하고, 마지막 단계는 반복한다.

use std::sync::Mutex;
use std::time::Duration;

use serde::Deserialize;

use super::parse::parse_response;
use super::types::*;

#[derive(Clone, Debug, Deserialize)]
pub struct Step {
    #[serde(default)]
    pub delay_ms: u64,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub explanation: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
    /// 모델 원문 응답 (파싱 경로 검증용)
    #[serde(default)]
    pub raw: Option<String>,
    /// 오류 이름 (`usage_limit`, `auth_required`, `network` 등)
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Scenario {
    pub steps: Vec<Step>,
    #[serde(default)]
    pub model: Option<String>,
}

pub struct MockProvider {
    scenario: Scenario,
    next: Mutex<usize>,
}

impl MockProvider {
    pub fn from_file(path: &std::path::Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let scenario: Scenario =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if scenario.steps.is_empty() {
            return Err("mock scenario has no steps".into());
        }
        Ok(Self {
            scenario,
            next: Mutex::new(0),
        })
    }

    pub fn echo() -> Self {
        Self {
            scenario: Scenario {
                steps: vec![Step {
                    delay_ms: 200,
                    command: None,
                    explanation: None,
                    notes: vec![],
                    raw: None,
                    error: None,
                }],
                model: None,
            },
            next: Mutex::new(0),
        }
    }

    fn take_step(&self) -> Step {
        let mut next = self.next.lock().unwrap_or_else(|e| e.into_inner());
        let idx = (*next).min(self.scenario.steps.len() - 1);
        *next += 1;
        self.scenario.steps[idx].clone()
    }

    fn model(&self) -> String {
        self.scenario.model.clone().unwrap_or_else(|| "mock".into())
    }
}

pub fn named_error(name: &str) -> LlmError {
    match name {
        "auth_required" => LlmError::AuthRequired,
        "auth_failed" => LlmError::AuthFailed {
            detail: "mock".into(),
        },
        "not_eligible" => LlmError::NotEligible,
        "client_not_enabled" => LlmError::ClientNotEnabled,
        "usage_limit" => LlmError::UsageLimit,
        "region" => LlmError::RegionOrPolicy {
            detail: "mock".into(),
        },
        "unsupported" => LlmError::Unsupported {
            param: Some("mock".into()),
        },
        "model_not_found" => LlmError::ModelNotFound,
        "transient" => LlmError::Transient { status: Some(503) },
        "timeout" => LlmError::Timeout,
        "invalid" => LlmError::InvalidResponse {
            detail: "mock".into(),
        },
        _ => LlmError::Network {
            detail: "mock".into(),
        },
    }
}

impl Provider for MockProvider {
    fn label(&self) -> ProviderLabel {
        ProviderLabel {
            model: Some(self.model()),
            chatgpt_plan: false,
        }
    }

    fn complete(&self, req: LlmRequest, cancel: CancelToken) -> LlmFuture<'_> {
        let step = self.take_step();
        let model = self.model();
        Box::pin(async move {
            tokio::select! {
                _ = cancel.cancelled() => return Err(LlmError::Cancelled),
                _ = tokio::time::sleep(Duration::from_millis(step.delay_ms)) => {}
            }
            if let Some(name) = &step.error {
                return Err(named_error(name));
            }
            if let Some(raw) = &step.raw {
                return parse_response(raw, req.op, &model);
            }
            let command = match req.op {
                Operation::Explain => None,
                _ => step.command.clone().or_else(|| {
                    Some(format!(
                        "echo {}",
                        shell_quote(req.question.as_deref().unwrap_or("hello"))
                    ))
                }),
            };
            Ok(LlmResponse {
                command,
                explanation: step
                    .explanation
                    .clone()
                    .unwrap_or_else(|| "mock explanation".into()),
                notes: step.notes.clone(),
                model,
            })
        })
    }
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
