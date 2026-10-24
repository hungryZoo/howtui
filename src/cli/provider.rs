//! Provider와 인증 구성, 인증 하위 명령 (PRD-CLI-003, PRD-AUTH 계열).

use std::io::BufRead;
use std::process::ExitCode;
use std::sync::Arc;

use crate::auth::{AuthManager, LoginOptions, LogoutOutcome, TuiLogin, mask_email};
use crate::config::{Loaded, ProviderKind};
use crate::i18n::{Msg, tr, trf};
use crate::llm::api::ApiProvider;
use crate::llm::chatgpt::ChatGptProvider;
use crate::llm::keys::{self, KeySource};
use crate::llm::switch::{Agent, AgentState, SwitchProvider};
use crate::llm::types::{CancelToken, LlmError, Provider};
use crate::platform::paths::{self, process_env};
use crate::runtime::{LoginProgress, LoginService};

pub struct Built {
    pub provider: Arc<dyn Provider>,
    pub login: Option<Arc<dyn LoginService>>,
    pub logged_in: bool,
    pub auth: Option<Arc<AuthManager>>,
}

pub fn auth_manager() -> Result<Arc<AuthManager>, String> {
    let data_dir = paths::data_dir(&process_env)
        .ok_or_else(|| "cannot determine the data directory (set HOWTUI_DATA_DIR)".to_string())?;
    Ok(Arc::new(AuthManager::new(
        data_dir,
        crate::llm::http::client(),
    )))
}

/// 에이전트 선택 순서: CLI `--provider` > 설정 화면 선택(상태 파일) > 설정 파일 > chatgpt (SRS-LLM-017).
pub fn choose_agent(
    cli: Option<ProviderKind>,
    saved: Option<&str>,
    config: Option<ProviderKind>,
) -> ProviderKind {
    cli.or_else(|| saved.and_then(ProviderKind::parse))
        .or(config)
        .unwrap_or(ProviderKind::Chatgpt)
}

/// 키 출처 표시 (`auth status`, 설정 화면)
pub fn key_source_text(source: &KeySource) -> String {
    match source {
        KeySource::Env(env) => trf(Msg::KeySourceEnv, &[("env", env)]),
        KeySource::Store => tr(Msg::KeySourceStore).to_string(),
        KeySource::Missing => tr(Msg::KeySourceNone).to_string(),
    }
}

/// 시험 빌드 전용 주소 재정의 (`HOWTUI_TEST_OPENAI_BASE` 등, SRS-LLM-009)
fn test_base(kind: ProviderKind) -> Option<String> {
    #[cfg(feature = "mock-provider")]
    {
        process_env(&format!(
            "HOWTUI_TEST_{}_BASE",
            kind.id().to_ascii_uppercase()
        ))
    }
    #[cfg(not(feature = "mock-provider"))]
    {
        let _ = kind;
        None
    }
}

fn api_agent(kind: ProviderKind, loaded: &Loaded, auth: &Arc<AuthManager>) -> Agent {
    let cfg = loaded
        .config
        .provider
        .api(kind)
        .cloned()
        .unwrap_or_default();
    let resolved = keys::resolve(kind, &cfg, &process_env, Some(auth));
    let mut provider = ApiProvider::new(
        kind,
        crate::llm::http::client(),
        &cfg,
        resolved.key.clone(),
        Some(auth.clone()),
    );
    if let Some(base) = test_base(kind) {
        provider = provider.with_base(base);
    }
    let source = resolved.source.clone();
    let base_url = cfg.base_url.clone();
    let env = keys::env_name(kind, &cfg);
    Agent {
        kind,
        provider: Arc::new(provider),
        describe: Arc::new(move || {
            if kind == ProviderKind::OpenaiCompatible {
                return match &base_url {
                    Some(url) => AgentState {
                        ready: true,
                        detail: url.clone(),
                        blocker: None,
                    },
                    None => AgentState {
                        ready: false,
                        detail: tr(Msg::AgentNoUrl).to_string(),
                        blocker: Some(LlmError::Unsupported {
                            param: Some("provider.openai_compatible.base_url".into()),
                        }),
                    },
                };
            }
            match &source {
                KeySource::Env(env) => AgentState {
                    ready: true,
                    detail: trf(Msg::AgentKeyEnv, &[("env", env)]),
                    blocker: None,
                },
                KeySource::Store => AgentState {
                    ready: true,
                    detail: tr(Msg::AgentKeyStore).to_string(),
                    blocker: None,
                },
                KeySource::Missing => AgentState {
                    ready: false,
                    detail: trf(Msg::AgentNoKey, &[("id", kind.id())]),
                    blocker: Some(LlmError::NoApiKey {
                        provider: kind.id().into(),
                        env: env.clone(),
                    }),
                },
            }
        }),
    }
}

pub fn build(loaded: &Loaded) -> Result<Built, String> {
    build_with(loaded, None)
}

pub fn build_with(loaded: &Loaded, cli_agent: Option<ProviderKind>) -> Result<Built, String> {
    #[cfg(feature = "mock-provider")]
    {
        if let Some(path) = process_env("HOWTUI_MOCK_SCENARIO") {
            let mock = if path == "echo" {
                crate::llm::mock::MockProvider::echo()
            } else {
                crate::llm::mock::MockProvider::from_file(std::path::Path::new(&path))?
            };
            return Ok(Built {
                provider: Arc::new(mock),
                login: None,
                logged_in: true,
                auth: None,
            });
        }
    }
    let auth = auth_manager()?;
    let chatgpt: Arc<dyn Provider> = Arc::new(ChatGptProvider::new(
        crate::llm::http::client(),
        auth.clone(),
        loaded.config.provider.chatgpt.model.clone(),
    ));
    let a = auth.clone();
    let mut agents = vec![Agent {
        kind: ProviderKind::Chatgpt,
        provider: chatgpt,
        describe: Arc::new(move || AgentState {
            ready: true,
            detail: tr(if a.has_credentials() {
                Msg::AgentSignedIn
            } else {
                Msg::AgentNeedsLogin
            })
            .to_string(),
            blocker: None,
        }),
    }];
    for kind in [
        ProviderKind::Openai,
        ProviderKind::Anthropic,
        ProviderKind::Gemini,
        ProviderKind::OpenaiCompatible,
    ] {
        agents.push(api_agent(kind, loaded, &auth));
    }
    let current = choose_agent(
        cli_agent,
        auth.load_state().selected_provider.as_deref(),
        loaded.config.provider.default,
    );
    let a = auth.clone();
    let save: Arc<dyn Fn(ProviderKind) + Send + Sync> = Arc::new(move |kind| {
        let mut s = a.load_state();
        s.selected_provider = Some(kind.id().into());
        a.save_state(&s);
    });
    let provider = Arc::new(SwitchProvider::new(agents, current, Some(save)));
    // ChatGPT 외 에이전트는 로그인 화면 없이 시작 (키 없음은 첫 화면 상태줄로 안내)
    let logged_in = current != ProviderKind::Chatgpt || auth.has_credentials();
    Ok(Built {
        provider,
        login: Some(Arc::new(TuiLogin { auth: auth.clone() })),
        logged_in,
        auth: Some(auth),
    })
}

pub fn login_command(
    _loaded: &Loaded,
    rt: &tokio::runtime::Runtime,
    no_browser: bool,
    port: Option<u16>,
) -> ExitCode {
    let auth = match auth_manager() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{}", trf(Msg::ErrConfig, &[("detail", &e)]));
            return ExitCode::from(8);
        }
    };
    println!("{}\n", tr(Msg::PrivacyNotice));
    let (paste_tx, paste_rx) = tokio::sync::mpsc::unbounded_channel();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            if !line.trim().is_empty() && paste_tx.send(line).is_err() {
                break;
            }
        }
    });
    let progress = move |p: LoginProgress| match p {
        LoginProgress::Started(view) => {
            if view.browser_failed && !no_browser {
                println!("{}", tr(Msg::LoginBrowserFailed));
            }
            println!(
                "{}\n\n  {}\n",
                tr(Msg::LoginOpenUrl),
                view.url.unwrap_or_default()
            );
            println!("{} {}", tr(Msg::LoginPasteHint), tr(Msg::LoginPressEnter));
            if let Some(port) = view.port {
                println!("{}", trf(Msg::LoginSshHint, &[("port", &port.to_string())]));
            }
            println!("\n{}", tr(Msg::LoginWaiting));
        }
        LoginProgress::InvalidPaste => println!("{}", tr(Msg::LoginInvalidPaste)),
    };
    let cancel = CancelToken::new();
    let c = cancel.clone();
    rt.spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            c.cancel();
        }
    });
    let result = rt.block_on(auth.login(
        LoginOptions { no_browser, port },
        &progress,
        paste_rx,
        cancel,
    ));
    match result {
        Ok(info) => {
            let email = info
                .email
                .as_deref()
                .map(mask_email)
                .unwrap_or_else(|| "-".into());
            println!("{}", trf(Msg::LoginSuccess, &[("email", &email)]));
            if let Some(path) = auth
                .file_store_notice
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_ref()
            {
                println!(
                    "{}",
                    trf(
                        Msg::StoreFileFallback,
                        &[("path", &paths::tilde(path, &process_env))]
                    )
                );
            }
            if !info.plan_enabled {
                println!("{}", tr(Msg::LoginPlanDisabled));
                return ExitCode::from(5);
            }
            if info.show_plan_notice {
                println!("{}", tr(Msg::PlanNotice));
            }
            ExitCode::SUCCESS
        }
        Err(failure) => {
            use crate::app::LoginFailure as F;
            let (msg, code) = match &failure {
                F::Cancelled => (tr(Msg::LoginCancelled).to_string(), 130),
                F::Denied => (tr(Msg::LoginDenied).to_string(), 4),
                F::StateMismatch => (tr(Msg::LoginStateMismatch).to_string(), 4),
                F::InvalidPaste => (tr(Msg::LoginInvalidPaste).to_string(), 4),
                F::Timeout => (tr(Msg::LoginTimeout).to_string(), 4),
                F::PlanDisabled => (tr(Msg::LoginPlanDisabled).to_string(), 5),
                F::Other(d) => (trf(Msg::ErrInternal, &[("detail", d)]), 4),
            };
            eprintln!("{msg}");
            ExitCode::from(code)
        }
    }
}

pub fn logout_command(_loaded: &Loaded, rt: &tokio::runtime::Runtime, forget: bool) -> ExitCode {
    let auth = match auth_manager() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{}", trf(Msg::ErrConfig, &[("detail", &e)]));
            return ExitCode::from(8);
        }
    };
    match rt.block_on(auth.logout(forget)) {
        LogoutOutcome::Revoked => println!(
            "{}",
            tr(if forget {
                Msg::LogoutForgetDone
            } else {
                Msg::LogoutDone
            })
        ),
        LogoutOutcome::RevokeUnconfirmed => println!("{}", tr(Msg::LogoutRevokeUnconfirmed)),
        LogoutOutcome::NotSignedIn => println!("{}", tr(Msg::LogoutNotSignedIn)),
    }
    ExitCode::SUCCESS
}

pub fn status_command(_loaded: &Loaded, _rt: &tokio::runtime::Runtime) -> ExitCode {
    let auth = match auth_manager() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{}", trf(Msg::ErrConfig, &[("detail", &e)]));
            return ExitCode::from(8);
        }
    };
    let st = auth.status();
    let current = choose_agent(
        None,
        auth.load_state().selected_provider.as_deref(),
        _loaded.config.provider.default,
    );
    println!(
        "{}",
        trf(Msg::AuthStatusProvider, &[("agent", current.display())])
    );
    print_keys(_loaded, &auth);
    if !st.signed_in {
        println!("{}", tr(Msg::AuthStatusSignedOut));
        return ExitCode::from(4);
    }
    let email = st
        .email
        .as_deref()
        .map(mask_email)
        .unwrap_or_else(|| "-".into());
    println!("{}", trf(Msg::AuthStatusSignedIn, &[("email", &email)]));
    println!(
        "{}",
        tr(if st.plan_enabled {
            Msg::AuthStatusPlan
        } else {
            Msg::AuthStatusPlanDisabled
        })
    );
    if let Some(exp) = st.expires_at {
        println!(
            "{}",
            trf(
                Msg::AuthStatusExpires,
                &[("time", &crate::platform::time::format_utc(exp))]
            )
        );
    }
    if let Some(store) = st.store {
        println!("{}", trf(Msg::AuthStatusStore, &[("store", store.name())]));
    }
    ExitCode::SUCCESS
}

/// API 키 출처 목록 (키 값 미출력, PRD-LLM-022)
fn print_keys(loaded: &Loaded, auth: &AuthManager) {
    println!("{}", tr(Msg::AuthStatusKeys));
    for kind in [
        ProviderKind::Openai,
        ProviderKind::Anthropic,
        ProviderKind::Gemini,
    ] {
        let cfg = loaded
            .config
            .provider
            .api(kind)
            .cloned()
            .unwrap_or_default();
        let r = keys::resolve(kind, &cfg, &process_env, Some(auth));
        println!("  {}: {}", kind.id(), key_source_text(&r.source));
    }
    let compat = &loaded.config.provider.openai_compatible;
    if compat.base_url.is_some() {
        let r = keys::resolve(
            ProviderKind::OpenaiCompatible,
            compat,
            &process_env,
            Some(auth),
        );
        println!(
            "  {}: {}",
            ProviderKind::OpenaiCompatible.id(),
            key_source_text(&r.source)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_choice_order() {
        use ProviderKind as K;
        assert_eq!(
            choose_agent(Some(K::Gemini), Some("openai"), Some(K::Anthropic)),
            K::Gemini
        );
        assert_eq!(
            choose_agent(None, Some("openai"), Some(K::Anthropic)),
            K::Openai
        );
        assert_eq!(
            choose_agent(None, Some("bogus"), Some(K::Anthropic)),
            K::Anthropic
        );
        assert_eq!(choose_agent(None, None, None), K::Chatgpt);
    }
}
