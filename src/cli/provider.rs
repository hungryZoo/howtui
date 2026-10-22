//! Provider와 인증 구성, 인증 하위 명령 (PRD-CLI-003, PRD-AUTH 계열).

use std::io::BufRead;
use std::process::ExitCode;
use std::sync::Arc;

use crate::auth::{AuthManager, LoginOptions, LogoutOutcome, TuiLogin, mask_email};
use crate::config::Loaded;
use crate::i18n::{Msg, tr, trf};
use crate::llm::chatgpt::ChatGptProvider;
use crate::llm::types::{CancelToken, Provider};
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

pub fn build(loaded: &Loaded) -> Result<Built, String> {
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
    let provider = Arc::new(ChatGptProvider::new(
        crate::llm::http::client(),
        auth.clone(),
        loaded.config.provider.chatgpt.model.clone(),
    ));
    let logged_in = auth.has_credentials();
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
            println!("{}", tr(Msg::LoginPasteHint));
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
            trf(Msg::AuthStatusExpires, &[("time", &format_utc(exp))])
        );
    }
    if let Some(store) = st.store {
        println!("{}", trf(Msg::AuthStatusStore, &[("store", store.name())]));
    }
    ExitCode::SUCCESS
}

/// Unix 시각 → `YYYY-MM-DD HH:MM:SS UTC` (외부 crate 없이 그레고리력 변환).
pub fn format_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Howard Hinnant civil_from_days
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02}:{s:02} UTC")
}

#[cfg(test)]
mod tests {
    use super::format_utc;

    #[test]
    fn utc_formatting() {
        assert_eq!(format_utc(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(format_utc(1_792_253_674), "2026-10-17 16:14:34 UTC");
        assert_eq!(format_utc(951_782_400), "2000-02-29 00:00:00 UTC");
    }
}
