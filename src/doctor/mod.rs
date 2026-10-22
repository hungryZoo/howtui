//! howtui doctor (SRS 15절). 토큰, 환경변수 값, 홈 경로를 출력하지 않는다 (SRS-DOC-003).

use std::io::IsTerminal;
use std::process::ExitCode;

use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::auth::{AuthManager, mask_email};
use crate::config::Loaded;
use crate::i18n::{Msg, tr, trf};
use crate::llm::chatgpt::ChatGptProvider;
use crate::platform::paths::{process_env, tilde};
use crate::shell::{self, ShellSource};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
    Info,
    Skip,
}

#[derive(Clone, Debug, Serialize)]
pub struct Check {
    pub id: &'static str,
    pub status: CheckStatus,
    pub summary: String,
    pub details: Map<String, Value>,
}

fn check(
    id: &'static str,
    status: CheckStatus,
    summary: impl Into<String>,
    details: Value,
) -> Check {
    Check {
        id,
        status,
        summary: summary.into(),
        details: details.as_object().cloned().unwrap_or_default(),
    }
}

/// 알려진 터미널 프로그램 이름만 공개 (임의 값 비공개).
const KNOWN_TERM_PROGRAMS: &[&str] = &[
    "Apple_Terminal",
    "iTerm.app",
    "vscode",
    "WezTerm",
    "ghostty",
    "Hyper",
    "tmux",
    "WarpTerminal",
    "kitty",
    "alacritty",
    "Tabby",
    "rio",
];

pub fn collect(
    loaded: &Loaded,
    rt: &tokio::runtime::Runtime,
    auth: Option<&std::sync::Arc<AuthManager>>,
) -> Vec<Check> {
    let env = process_env;
    let mut checks = Vec::new();

    checks.push(check(
        "version",
        CheckStatus::Info,
        format!("howtui {} ({} {})", env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH),
        json!({"version": env!("CARGO_PKG_VERSION"), "os": std::env::consts::OS, "arch": std::env::consts::ARCH}),
    ));

    let os_name = os_release().unwrap_or_else(|| std::env::consts::OS.to_string());
    checks.push(check(
        "os",
        CheckStatus::Info,
        format!("{os_name} {}", std::env::consts::ARCH),
        json!({"name": os_name}),
    ));

    let term = env("TERM").unwrap_or_else(|| "-".into());
    let program = env("TERM_PROGRAM").map(|p| {
        if KNOWN_TERM_PROGRAMS.contains(&p.as_str()) {
            p
        } else {
            "other".into()
        }
    });
    let size = crossterm::terminal::size().ok();
    checks.push(check(
        "terminal",
        CheckStatus::Info,
        format!("TERM={term}{}{}", program.as_deref().map(|p| format!(" TERM_PROGRAM={p}")).unwrap_or_default(), size.map(|(w, h)| format!(" {w}x{h}")).unwrap_or_default()),
        json!({"term": term, "term_program": program, "columns": size.map(|s| s.0), "rows": size.map(|s| s.1), "in_tmux": env("TMUX").is_some()}),
    ));

    let (kind, source) = shell::resolve(None, &env);
    let source_name = match source {
        ShellSource::Flag => "flag",
        ShellSource::IntegrationEnv => "integration",
        ShellSource::OsDefault => "os-default",
    };
    let login_shell = env("SHELL").map(|s| s.rsplit(['/', '\\']).next().unwrap_or("").to_string());
    checks.push(check(
        "shell",
        if source == ShellSource::OsDefault {
            CheckStatus::Info
        } else {
            CheckStatus::Pass
        },
        format!("{kind} ({source_name})"),
        json!({"shell": kind.name(), "source": source_name, "login_shell": login_shell}),
    ));

    let stdin = std::io::stdin().is_terminal();
    let stdout = std::io::stdout().is_terminal();
    let stderr = std::io::stderr().is_terminal();
    let control = crate::platform::tty::open_term_out().is_ok();
    checks.push(check(
        "tty",
        if control {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        format!(
            "control terminal {} (stdin {stdin}, stdout {stdout}, stderr {stderr})",
            if control { "available" } else { "unavailable" }
        ),
        json!({"control_terminal": control, "stdin": stdin, "stdout": stdout, "stderr": stderr}),
    ));

    checks.push(locale_check(&env));

    if control {
        match crate::ui::terminal::probe() {
            Ok((w, h)) => checks.push(check(
                "render",
                CheckStatus::Pass,
                format!("cursor position query answered ({w}x{h})"),
                json!({"cursor_query": true}),
            )),
            Err(e) => checks.push(check(
                "render",
                CheckStatus::Fail,
                format!("cursor position query failed: {e}"),
                json!({"cursor_query": false}),
            )),
        }
    } else {
        checks.push(check(
            "render",
            CheckStatus::Skip,
            "no control terminal",
            json!({}),
        ));
    }

    let config_path = loaded
        .path
        .as_deref()
        .map(|p| tilde(p, &env))
        .unwrap_or_else(|| "-".into());
    let (status, summary) = if !loaded.unknown_keys.is_empty() {
        (
            CheckStatus::Warn,
            format!(
                "{config_path}: unknown keys {}",
                loaded.unknown_keys.join(", ")
            ),
        )
    } else if loaded.exists {
        (CheckStatus::Pass, format!("{config_path}: valid"))
    } else {
        (
            CheckStatus::Info,
            format!("{config_path}: not found, defaults in use"),
        )
    };
    checks.push(check(
        "config",
        status,
        summary,
        json!({"path": config_path, "exists": loaded.exists, "unknown_keys": loaded.unknown_keys}),
    ));

    match auth {
        None => {
            checks.push(check("auth", CheckStatus::Skip, "mock provider", json!({})));
            checks.push(check(
                "provider",
                CheckStatus::Skip,
                "mock provider",
                json!({}),
            ));
        }
        Some(auth) => {
            let st = auth.status();
            let store = st.store.map(|s| s.name());
            if !st.signed_in {
                checks.push(check(
                    "auth",
                    CheckStatus::Fail,
                    tr(Msg::AuthStatusSignedOut),
                    json!({"signed_in": false, "store": store}),
                ));
                checks.push(check(
                    "provider",
                    CheckStatus::Skip,
                    "not signed in",
                    json!({}),
                ));
            } else {
                let email = st.email.as_deref().map(mask_email);
                let plan = st.plan_enabled;
                checks.push(check(
                    "auth",
                    if plan { CheckStatus::Pass } else { CheckStatus::Fail },
                    format!(
                        "{}; {}",
                        trf(Msg::AuthStatusSignedIn, &[("email", email.as_deref().unwrap_or("-"))]),
                        tr(if plan { Msg::AuthStatusPlan } else { Msg::AuthStatusPlanDisabled })
                    ),
                    json!({"signed_in": true, "email": email, "plan_enabled": plan, "expires_at": st.expires_at, "store": store}),
                ));
                if plan {
                    let provider = ChatGptProvider::new(
                        crate::llm::http::client(),
                        auth.clone(),
                        loaded.config.provider.chatgpt.model.clone(),
                    );
                    let result = rt.block_on(async {
                        let token = auth
                            .access_token()
                            .await
                            .map_err(crate::llm::chatgpt::map_auth_error)?;
                        provider.list_models(token.expose()).await
                    });
                    match result {
                        Ok(models) => {
                            let chosen = ChatGptProvider::choose_model(
                                &models,
                                loaded.config.provider.chatgpt.model.as_deref(),
                            );
                            checks.push(check(
                                "provider",
                                if chosen.is_some() { CheckStatus::Pass } else { CheckStatus::Fail },
                                format!("{} model(s), using {}", models.len(), chosen.as_deref().unwrap_or("-")),
                                json!({"models": models.iter().map(|m| m.slug.clone()).collect::<Vec<_>>(), "selected": chosen}),
                            ));
                        }
                        Err(e) => checks.push(check(
                            "provider",
                            CheckStatus::Fail,
                            e.to_string(),
                            json!({}),
                        )),
                    }
                } else {
                    checks.push(check(
                        "provider",
                        CheckStatus::Skip,
                        "plan usage not granted",
                        json!({}),
                    ));
                }
            }
        }
    }

    let integration = env("HOWTUI_SHELL_INTEGRATION");
    checks.push(check(
        "integration",
        CheckStatus::Info,
        integration
            .as_deref()
            .map(|s| format!("shell integration: {s}"))
            .unwrap_or_else(|| "shell integration not loaded".into()),
        json!({"loaded": integration.is_some(), "shell": integration}),
    ));
    checks
}

fn locale_check(env: &dyn Fn(&str) -> Option<String>) -> Check {
    #[cfg(windows)]
    {
        let _ = env;
        let cp = unsafe { windows_sys::Win32::System::Console::GetConsoleOutputCP() };
        check(
            "locale",
            CheckStatus::Info,
            format!("console output code page {cp}"),
            json!({"code_page": cp}),
        )
    }
    #[cfg(not(windows))]
    {
        let value = ["LC_ALL", "LC_CTYPE", "LANG"]
            .iter()
            .filter_map(|k| env(k))
            .find(|v| !v.is_empty());
        let utf8 = value.as_deref().is_some_and(|v| {
            let v = v.to_ascii_lowercase();
            v.contains("utf-8") || v.contains("utf8")
        });
        check(
            "locale",
            if utf8 {
                CheckStatus::Pass
            } else {
                CheckStatus::Warn
            },
            format!(
                "{} ({})",
                value.as_deref().unwrap_or("unset"),
                if utf8 { "UTF-8" } else { "not UTF-8" }
            ),
            json!({"utf8": utf8, "value": value}),
        )
    }
}

fn os_release() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let text = std::fs::read_to_string("/etc/os-release").ok()?;
        text.lines()
            .find_map(|l| l.strip_prefix("PRETTY_NAME="))
            .map(|v| v.trim_matches('"').to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

pub fn render_text(checks: &[Check]) -> String {
    let mut out = format!("{}\n", tr(Msg::DoctorTitle));
    for c in checks {
        let word = match c.status {
            CheckStatus::Pass => tr(Msg::DoctorPass),
            CheckStatus::Warn => tr(Msg::DoctorWarn),
            CheckStatus::Fail => tr(Msg::DoctorFail),
            CheckStatus::Info => tr(Msg::DoctorInfo),
            CheckStatus::Skip => tr(Msg::DoctorSkip),
        };
        out.push_str(&format!("  [{word}] {:<12} {}\n", c.id, c.summary));
    }
    let failed = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Fail)
        .count();
    if failed == 0 {
        out.push_str(tr(Msg::DoctorSummaryOk));
    } else {
        out.push_str(&trf(
            Msg::DoctorSummaryFail,
            &[("count", &failed.to_string())],
        ));
    }
    out.push('\n');
    out
}

pub fn render_json(checks: &[Check]) -> String {
    serde_json::to_string_pretty(&json!({"schema_version": SCHEMA_VERSION, "howtui_version": env!("CARGO_PKG_VERSION"), "checks": checks}))
        .unwrap_or_default()
}

pub fn run(loaded: &Loaded, rt: &tokio::runtime::Runtime, json: bool) -> ExitCode {
    let auth = match crate::cli::provider_auth(loaded) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{}", trf(Msg::ErrConfig, &[("detail", &e)]));
            return ExitCode::from(8);
        }
    };
    let checks = collect(loaded, rt, auth.as_ref());
    let text = if json {
        render_json(&checks)
    } else {
        render_text(&checks)
    };
    print!("{}", mask_text(&text, &process_env));
    if checks.iter().any(|c| c.status == CheckStatus::Fail) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// 최종 출력 마스킹: 홈 경로 `~` 치환과 토큰 유사 문자열(JWT) 제거.
pub fn mask_text(text: &str, env: &dyn Fn(&str) -> Option<String>) -> String {
    let mut out = text.to_string();
    let _ = env;
    if let Some(home) = std::env::home_dir().map(|h| h.display().to_string())
        && home.len() > 1
    {
        out = out.replace(&home, "~");
    }
    out.split(' ')
        .map(|w| {
            if looks_like_jwt(w) {
                "[REDACTED]".to_string()
            } else {
                w.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn looks_like_jwt(word: &str) -> bool {
    let w =
        word.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '-' && c != '_');
    let parts: Vec<&str> = w.split('.').collect();
    parts.len() == 3 && w.starts_with("eyJ") && parts.iter().all(|p| p.len() >= 8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_tokens_and_home() {
        let env = |k: &str| (k == "HOWTUI_DATA_DIR").then(|| "/x".to_string());
        let home = std::env::home_dir().unwrap().display().to_string();
        let text = format!(
            "path {home}/.config/howtui token eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiJ1c2VyIn0.c2lnbmF0dXJlLXZhbHVl end"
        );
        let masked = mask_text(&text, &env);
        assert!(masked.contains("~/.config/howtui"), "{masked}");
        assert!(
            masked.contains("[REDACTED]") && !masked.contains("eyJzdWIi"),
            "{masked}"
        );
    }

    #[test]
    fn json_schema_is_stable() {
        let checks = vec![check("tty", CheckStatus::Pass, "ok", json!({"x": 1}))];
        let v: Value = serde_json::from_str(&render_json(&checks)).unwrap();
        assert_eq!(v["schema_version"], 1);
        assert_eq!(v["checks"][0]["id"], "tty");
        assert_eq!(v["checks"][0]["status"], "pass");
        assert_eq!(v["checks"][0]["details"]["x"], 1);
    }

    #[test]
    fn text_summary_counts_failures() {
        let checks = vec![
            check("a", CheckStatus::Fail, "x", json!({})),
            check("b", CheckStatus::Warn, "y", json!({})),
        ];
        let t = render_text(&checks);
        assert!(t.contains("1"), "{t}");
    }
}
