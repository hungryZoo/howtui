//! 에이전트 전환 (P5, CR-011, SRS-LLM-018). Provider 목록 중 현재 항목으로 trait 호출을 위임한다.

use std::sync::{Arc, Mutex};

use super::types::*;
use crate::config::ProviderKind;

/// 설정 화면 표시용 상태 (호출 시점 기준: 로그인과 키 저장은 실행 중 바뀔 수 있음)
pub type Describe = Arc<dyn Fn() -> AgentState + Send + Sync>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentState {
    pub ready: bool,
    pub detail: String,
    /// 사용 불가 항목 선택 시 반환할 오류
    pub blocker: Option<LlmError>,
}

pub struct Agent {
    pub kind: ProviderKind,
    pub provider: Arc<dyn Provider>,
    pub describe: Describe,
}

pub struct SwitchProvider {
    agents: Vec<Agent>,
    current: Mutex<usize>,
    save: Option<Arc<dyn Fn(ProviderKind) + Send + Sync>>,
}

impl SwitchProvider {
    pub fn new(
        agents: Vec<Agent>,
        current: ProviderKind,
        save: Option<Arc<dyn Fn(ProviderKind) + Send + Sync>>,
    ) -> Self {
        let index = agents.iter().position(|a| a.kind == current).unwrap_or(0);
        Self {
            agents,
            current: Mutex::new(index),
            save,
        }
    }

    fn index(&self) -> usize {
        *self.current.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn active(&self) -> &Agent {
        &self.agents[self.index()]
    }

    pub fn current_kind(&self) -> ProviderKind {
        self.active().kind
    }

    fn choices(&self) -> Vec<AgentChoice> {
        self.agents
            .iter()
            .map(|a| {
                let st = (a.describe)();
                AgentChoice {
                    id: a.kind.id().into(),
                    name: a.kind.display().into(),
                    ready: st.ready,
                    detail: st.detail,
                }
            })
            .collect()
    }
}

impl Provider for SwitchProvider {
    fn label(&self) -> ProviderLabel {
        self.active().provider.label()
    }

    fn complete(&self, req: LlmRequest, cancel: CancelToken) -> LlmFuture<'_> {
        let provider = self.active().provider.clone();
        Box::pin(async move { provider.complete(req, cancel).await })
    }

    fn prepare(&self) -> PrepareFuture<'_> {
        let provider = self.active().provider.clone();
        Box::pin(async move { provider.prepare().await })
    }

    fn settings(&self) -> SettingsFuture<'_> {
        let agent = self.active();
        let provider = agent.provider.clone();
        let name = agent.kind.display().to_string();
        let id = agent.kind.id().to_string();
        Box::pin(async move {
            let mut data = match provider.settings().await {
                Ok(d) => d,
                // 현재 에이전트 조회 실패여도 다른 에이전트로 바꿀 수 있도록 목록은 표시
                Err(e) => ProviderSettings {
                    agent: name,
                    problem: Some(e),
                    ..Default::default()
                },
            };
            data.agents = self.choices();
            data.current_agent = Some(id);
            Ok(data)
        })
    }

    fn select_model(&self, id: String) -> PrepareFuture<'_> {
        let provider = self.active().provider.clone();
        Box::pin(async move { provider.select_model(id).await })
    }

    fn select_agent(&self, id: String) -> PrepareFuture<'_> {
        Box::pin(async move {
            let Some(index) = self.agents.iter().position(|a| a.kind.id() == id) else {
                return Err(LlmError::Unsupported { param: Some(id) });
            };
            let agent = &self.agents[index];
            let st = (agent.describe)();
            if !st.ready
                && let Some(e) = st.blocker
            {
                return Err(e);
            }
            *self.current.lock().unwrap_or_else(|e| e.into_inner()) = index;
            if let Some(save) = &self.save {
                save(agent.kind);
            }
            Ok(agent.provider.label())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed(&'static str);

    impl Provider for Fixed {
        fn label(&self) -> ProviderLabel {
            ProviderLabel {
                model: Some(self.0.into()),
                chatgpt_plan: false,
            }
        }
        fn complete(&self, _req: LlmRequest, _c: CancelToken) -> LlmFuture<'_> {
            let model = self.0.to_string();
            Box::pin(async move {
                Ok(LlmResponse {
                    command: Some("true".into()),
                    explanation: "x".into(),
                    notes: vec![],
                    model,
                })
            })
        }
        fn settings(&self) -> SettingsFuture<'_> {
            let name = self.0;
            Box::pin(async move {
                if name == "broken" {
                    return Err(LlmError::AuthRequired);
                }
                Ok(ProviderSettings {
                    agent: name.into(),
                    ..Default::default()
                })
            })
        }
        fn select_model(&self, _id: String) -> PrepareFuture<'_> {
            Box::pin(async move { Ok(self.label()) })
        }
    }

    fn agent(kind: ProviderKind, model: &'static str, ready: bool) -> Agent {
        Agent {
            kind,
            provider: Arc::new(Fixed(model)),
            describe: Arc::new(move || AgentState {
                ready,
                detail: if ready { "ok".into() } else { "no key".into() },
                blocker: (!ready).then(|| LlmError::NoApiKey {
                    provider: "gemini".into(),
                    env: None,
                }),
            }),
        }
    }

    fn request() -> LlmRequest {
        LlmRequest {
            id: 1,
            op: Operation::Generate,
            env: EnvContext::current(crate::shell::ShellKind::Bash, None),
            question: Some("q".into()),
            command: None,
            history: vec![],
            include_folder: false,
            folder: None,
        }
    }

    #[tokio::test]
    async fn switch_delegates_saves_and_blocks_unready() {
        let saved = Arc::new(Mutex::new(Vec::new()));
        let s2 = saved.clone();
        let sw = SwitchProvider::new(
            vec![
                agent(ProviderKind::Chatgpt, "broken", true),
                agent(ProviderKind::Openai, "gpt", true),
                agent(ProviderKind::Gemini, "gem", false),
            ],
            ProviderKind::Chatgpt,
            Some(Arc::new(move |k| s2.lock().unwrap().push(k))),
        );
        // 현재 에이전트 조회 실패: 목록과 문제 함께 반환
        let data = sw.settings().await.unwrap();
        assert_eq!(data.problem, Some(LlmError::AuthRequired));
        assert_eq!(data.agents.len(), 3);
        assert_eq!(data.current_agent.as_deref(), Some("chatgpt"));
        assert!(!data.agents[2].ready);

        assert!(matches!(
            sw.select_agent("gemini".into()).await,
            Err(LlmError::NoApiKey { .. })
        ));
        assert_eq!(
            sw.current_kind(),
            ProviderKind::Chatgpt,
            "사용 불가 항목은 전환 없음"
        );

        let label = sw.select_agent("openai".into()).await.unwrap();
        assert_eq!(label.model.as_deref(), Some("gpt"));
        assert_eq!(sw.current_kind(), ProviderKind::Openai);
        assert_eq!(*saved.lock().unwrap(), vec![ProviderKind::Openai]);
        let r = sw.complete(request(), CancelToken::new()).await.unwrap();
        assert_eq!(r.model, "gpt", "전환 후 요청은 새 Provider");
        assert!(sw.select_agent("nope".into()).await.is_err());
    }
}
