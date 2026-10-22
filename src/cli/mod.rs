//! CLI 정의와 진입점 (SRS 3.4절). 도움말 문자열은 선택 언어로 주입 (SRS-I18N-004).

mod no_tui;
mod provider;

use std::io::Write;
use std::process::ExitCode;
use std::sync::Arc;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};

use crate::app::Status;
use crate::config::{self, UiMode};
use crate::i18n::{self, Msg, tr, trf};
use crate::llm::types::EnvContext;
use crate::platform::paths::{self, process_env};
use crate::runtime::{self, TuiConfig, TuiOutcome};
use crate::shell::{self, ShellKind};
use crate::ui::terminal::{SessionError, install_panic_hook};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ModeArg {
    Inline,
    Fullscreen,
}

#[derive(Debug, Parser)]
#[command(
    name = "howtui",
    version,
    disable_help_subcommand = true,
    args_conflicts_with_subcommands = true
)]
pub struct Cli {
    #[arg(value_name = "QUESTION", trailing_var_arg = true)]
    pub question: Vec<String>,
    #[arg(long, conflicts_with = "widget")]
    pub no_tui: bool,
    #[arg(long, value_enum)]
    pub shell: Option<ShellKind>,
    #[arg(long, value_parser = clap::value_parser!(u16).range(config::MIN_HEIGHT as i64..=config::MAX_HEIGHT as i64))]
    pub height: Option<u16>,
    #[arg(long, value_enum)]
    pub mode: Option<ModeArg>,
    /// 쉘 어댑터 전용 (P2)
    #[arg(long, hide = true)]
    pub widget: bool,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Login {
        #[arg(long)]
        no_browser: bool,
        #[arg(long)]
        port: Option<u16>,
    },
    Logout {
        #[arg(long)]
        forget: bool,
    },
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    Doctor {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    Status,
}

fn localized_command() -> clap::Command {
    Cli::command()
        .about(tr(Msg::HelpAbout))
        .after_help(tr(Msg::HelpAfter))
        .mut_arg("question", |a| a.help(tr(Msg::HelpQuestion)))
        .mut_arg("no_tui", |a| a.help(tr(Msg::HelpNoTui)))
        .mut_arg("shell", |a| a.help(tr(Msg::HelpShell)))
        .mut_arg("height", |a| a.help(tr(Msg::HelpHeight)))
        .mut_arg("mode", |a| a.help(tr(Msg::HelpMode)))
        .mut_subcommand("login", |s| {
            s.about(tr(Msg::HelpLogin))
                .mut_arg("no_browser", |a| a.help(tr(Msg::HelpNoBrowser)))
                .mut_arg("port", |a| a.help(tr(Msg::HelpPort)))
        })
        .mut_subcommand("logout", |s| {
            s.about(tr(Msg::HelpLogout))
                .mut_arg("forget", |a| a.help(tr(Msg::HelpForget)))
        })
        .mut_subcommand("auth", |s| {
            s.about(tr(Msg::HelpAuth))
                .mut_subcommand("status", |s| s.about(tr(Msg::HelpAuthStatus)))
        })
        .mut_subcommand("doctor", |s| {
            s.about(tr(Msg::HelpDoctor))
                .mut_arg("json", |a| a.help(tr(Msg::HelpDoctorJson)))
        })
}

pub fn run() -> ExitCode {
    install_panic_hook();
    let env = process_env;
    let config_path = paths::config_file(&env);
    let loaded = config::load(config_path.as_deref());
    let lang_setting = loaded
        .as_ref()
        .map(|l| l.config.ui.language)
        .unwrap_or_default();
    i18n::set_lang(i18n::resolve_lang(lang_setting, &env));

    let matches = localized_command().get_matches();
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(c) => c,
        Err(e) => e.exit(),
    };
    let loaded = match loaded {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{}", trf(Msg::ErrConfig, &[("detail", &e.message)]));
            return ExitCode::from(8);
        }
    };
    init_logging(&env);
    let _ = rustls::crypto::ring::default_provider().install_default();

    let rt = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("{}", trf(Msg::ErrInternal, &[("detail", &e.to_string())]));
            return ExitCode::from(1);
        }
    };

    match cli.command {
        None => run_main(cli, loaded, &rt),
        Some(Command::Login { no_browser, port }) => {
            provider::login_command(&loaded, &rt, no_browser, port)
        }
        Some(Command::Logout { forget }) => provider::logout_command(&loaded, &rt, forget),
        Some(Command::Auth {
            command: AuthCommand::Status,
        }) => provider::status_command(&loaded, &rt),
        Some(Command::Doctor { json }) => crate::doctor::run(&loaded, &rt, json),
    }
}

fn run_main(cli: Cli, loaded: config::Loaded, rt: &tokio::runtime::Runtime) -> ExitCode {
    let env = process_env;
    let (shell, _) = shell::resolve(cli.shell, &env);
    let shell_version = env("HOWTUI_SHELL_VERSION");
    let ctx = EnvContext::current(shell, shell_version);
    let question = cli.question.join(" ");

    let built = match provider::build(&loaded) {
        Ok(b) => b,
        Err(message) => {
            eprintln!("{}", trf(Msg::ErrConfig, &[("detail", &message)]));
            return ExitCode::from(8);
        }
    };

    if cli.no_tui {
        return no_tui::run(&question, built, ctx, rt);
    }

    let wants_fullscreen = cli
        .mode
        .map(|m| m == ModeArg::Fullscreen)
        .unwrap_or(loaded.config.ui.mode == UiMode::Fullscreen);
    let startup_notice = wants_fullscreen.then(|| Status::warn(Msg::StatusFullscreenUnavailable));
    let height = cli.height.unwrap_or(loaded.config.ui.inline_height);
    let cfg = TuiConfig {
        height,
        lang: i18n::lang(),
        env: ctx,
        logged_in: built.logged_in,
        initial_question: (!question.trim().is_empty()).then_some(question),
        initial_command: None,
        startup_notice,
        keep_paste: cli.widget,
    };
    let login = built.login.clone();
    match runtime::run_tui(cfg, Arc::clone(&built.provider), login, rt.handle()) {
        Ok(TuiOutcome::Accepted(cmd)) => match crate::output::emit_stdout(&cmd) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{}", trf(Msg::ErrInternal, &[("detail", &e.to_string())]));
                ExitCode::from(1)
            }
        },
        Ok(TuiOutcome::Cancelled) => ExitCode::from(130),
        Ok(TuiOutcome::Signal(sig)) => ExitCode::from(128u8.saturating_add(sig as u8)),
        Err(SessionError::NoTerminal(_)) => {
            eprintln!("{}", tr(Msg::ErrNoTerminal));
            ExitCode::from(3)
        }
        Err(SessionError::Io(e)) => {
            eprintln!("{}", trf(Msg::ErrInternal, &[("detail", &e.to_string())]));
            ExitCode::from(1)
        }
    }
}

/// `HOWTUI_LOG`(수준) 설정 시에만 파일 로그 (SRS-OUT-004). stdout과 stderr 미사용.
fn init_logging(env: &dyn Fn(&str) -> Option<String>) {
    let Some(level) = env("HOWTUI_LOG") else {
        return;
    };
    let level = match level.to_ascii_lowercase().as_str() {
        "error" => tracing::Level::ERROR,
        "warn" => tracing::Level::WARN,
        "info" => tracing::Level::INFO,
        "trace" => tracing::Level::TRACE,
        _ => tracing::Level::DEBUG,
    };
    let path = env("HOWTUI_LOG_FILE")
        .map(std::path::PathBuf::from)
        .or_else(|| paths::data_dir(env).map(|d| d.join("logs").join("howtui.log")));
    let Some(path) = path else { return };
    if let Some(dir) = path.parent()
        && crate::platform::fs::ensure_private_dir(dir).is_err()
    {
        return;
    }
    let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    else {
        return;
    };
    let _ = tracing_subscriber::fmt()
        .with_max_level(level)
        .with_ansi(false)
        .with_writer(std::sync::Mutex::new(file))
        .try_init();
    let _ = std::io::stderr().flush();
}
