//! CLI 정의와 진입점 (SRS 3.4절). 도움말 문자열은 선택 언어로 주입 (SRS-I18N-004).

mod no_tui;
mod provider;
mod widget;

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
    /// 새 버전 즉시 확인 (CR-009)
    #[arg(long, conflicts_with_all = ["no_tui", "widget", "update"])]
    pub check_update: bool,
    /// 새 버전으로 즉시 업데이트 (CR-009)
    #[arg(long, conflicts_with_all = ["no_tui", "widget"])]
    pub update: bool,
    #[arg(long, value_enum)]
    pub shell: Option<ShellKind>,
    #[arg(long, value_parser = clap::value_parser!(u16).range(config::MIN_HEIGHT as i64..=config::MAX_HEIGHT as i64))]
    pub height: Option<u16>,
    #[arg(long, value_enum)]
    pub mode: Option<ModeArg>,
    /// 쉘 통합 스크립트 전용 (SRS 9.2절)
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
    Init {
        #[arg(value_enum)]
        shell: ShellKind,
        #[arg(long, value_name = "KEY")]
        key: Option<String>,
        #[arg(long)]
        no_bind: bool,
        #[arg(long)]
        no_wrap: bool,
        #[arg(long, conflicts_with = "uninstall")]
        install: bool,
        #[arg(long)]
        uninstall: bool,
        /// 설치 스크립트 전용: 표식 구간에 PATH 추가 포함
        #[arg(long, hide = true, value_name = "DIR", requires = "install")]
        add_path: Option<std::path::PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    Status,
}

fn help_arg() -> clap::Arg {
    clap::Arg::new("help")
        .short('h')
        .long("help")
        .action(clap::ArgAction::Help)
        .help(tr(Msg::HelpHelp))
}

fn localized_command() -> clap::Command {
    Cli::command()
        .about(tr(Msg::HelpAbout))
        .after_help(tr(Msg::HelpAfter))
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(help_arg())
        .arg(
            clap::Arg::new("version")
                .short('V')
                .long("version")
                .action(clap::ArgAction::Version)
                .help(tr(Msg::HelpVersion)),
        )
        .mut_arg("question", |a| a.help(tr(Msg::HelpQuestion)))
        .mut_arg("no_tui", |a| a.help(tr(Msg::HelpNoTui)))
        .mut_arg("check_update", |a| a.help(tr(Msg::HelpCheckUpdate)))
        .mut_arg("update", |a| a.help(tr(Msg::HelpUpdate)))
        .mut_arg("shell", |a| a.help(tr(Msg::HelpShell)))
        .mut_arg("height", |a| a.help(tr(Msg::HelpHeight)))
        .mut_arg("mode", |a| a.help(tr(Msg::HelpMode)))
        .mut_subcommand("login", |s| {
            s.about(tr(Msg::HelpLogin))
                .disable_help_flag(true)
                .arg(help_arg())
                .mut_arg("no_browser", |a| a.help(tr(Msg::HelpNoBrowser)))
                .mut_arg("port", |a| a.help(tr(Msg::HelpPort)))
        })
        .mut_subcommand("logout", |s| {
            s.about(tr(Msg::HelpLogout))
                .disable_help_flag(true)
                .arg(help_arg())
                .mut_arg("forget", |a| a.help(tr(Msg::HelpForget)))
        })
        .mut_subcommand("auth", |s| {
            s.about(tr(Msg::HelpAuth))
                .disable_help_flag(true)
                .arg(help_arg())
                .mut_subcommand("status", |s| {
                    s.about(tr(Msg::HelpAuthStatus))
                        .disable_help_flag(true)
                        .arg(help_arg())
                })
        })
        .mut_subcommand("doctor", |s| {
            s.about(tr(Msg::HelpDoctor))
                .disable_help_flag(true)
                .arg(help_arg())
                .mut_arg("json", |a| a.help(tr(Msg::HelpDoctorJson)))
        })
        .mut_subcommand("init", |s| {
            s.about(tr(Msg::HelpInit))
                .disable_help_flag(true)
                .arg(help_arg())
                .mut_arg("shell", |a| a.help(tr(Msg::HelpInitShell)))
                .mut_arg("key", |a| a.help(tr(Msg::HelpInitKey)))
                .mut_arg("no_bind", |a| a.help(tr(Msg::HelpNoBind)))
                .mut_arg("no_wrap", |a| a.help(tr(Msg::HelpNoWrap)))
                .mut_arg("install", |a| a.help(tr(Msg::HelpInstall)))
                .mut_arg("uninstall", |a| a.help(tr(Msg::HelpUninstall)))
        })
}

/// 프로세스 시작 시각 (첫 화면 표시 시간 측정용).
pub static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

pub fn run() -> ExitCode {
    let _ = START.set(std::time::Instant::now());
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
    // rc 파일에서 매번 실행되므로 설정 파일 오류와 무관하게 동작
    if let Some(Command::Init {
        shell,
        key,
        no_bind,
        no_wrap,
        install,
        uninstall,
        add_path,
    }) = &cli.command
    {
        let opts = InitArgs {
            shell: *shell,
            key: key.as_deref(),
            no_bind: *no_bind,
            no_wrap: *no_wrap,
        };
        return match (*install, *uninstall) {
            (true, _) => rc_command(opts, true, add_path.as_deref()),
            (_, true) => rc_command(opts, false, None),
            _ => init_command(opts),
        };
    }
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

    // Windows: 지난 업데이트가 남긴 실행 파일 정리
    crate::update::cleanup_previous();
    if cli.check_update {
        return check_update_command(&rt);
    }
    if cli.update {
        return update_command(&rt);
    }

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
        Some(Command::Init { .. }) => unreachable!("handled before configuration loading"),
    }
}

#[derive(Clone, Copy)]
struct InitArgs<'a> {
    shell: ShellKind,
    key: Option<&'a str>,
    no_bind: bool,
    no_wrap: bool,
}

/// 키 검증 실패 시 메시지 출력 후 None (종료 코드 2).
fn parse_key(key: Option<&str>) -> Option<crate::shell::init::Key> {
    use crate::shell::init::{Key, KeyError};
    match key.map(Key::parse).unwrap_or(Ok(Key::default())) {
        Ok(k) => Some(k),
        Err(e) => {
            let msg = match e {
                KeyError::Format => Msg::ErrKeyFormat,
                KeyError::Reserved => Msg::ErrKeyReserved,
            };
            eprintln!("{}", trf(msg, &[("key", key.unwrap_or_default())]));
            None
        }
    }
}

/// `init --install`, `--uninstall`: rc 파일 표식 구간 추가와 제거 (CR-004).
fn rc_command(args: InitArgs, install: bool, add_path: Option<&std::path::Path>) -> ExitCode {
    use crate::shell::rc;
    let Some(key) = parse_key(args.key) else {
        return ExitCode::from(2);
    };
    let env = process_env;
    let targets = rc::rc_paths(args.shell, &env);
    if targets.is_empty() {
        eprintln!("{}", tr(Msg::ErrInstallUnsupported));
        return ExitCode::from(2);
    }
    let mut init_args = String::new();
    if key != crate::shell::init::Key::default() {
        init_args.push_str(&format!(" --key {}", key.arg()));
    }
    if args.no_bind {
        init_args.push_str(" --no-bind");
    }
    if args.no_wrap {
        init_args.push_str(" --no-wrap");
    }
    let block = rc::block(args.shell, &init_args, add_path);
    // PowerShell은 Profile 2개 (7, 5.1): 하나라도 실패하면 종료 코드 1
    let mut code = ExitCode::SUCCESS;
    for path in &targets {
        let result = if install {
            rc::install(path, &block)
        } else {
            rc::uninstall(path)
        };
        if report_rc(path, result) != ExitCode::SUCCESS {
            code = ExitCode::from(1);
        }
    }
    code
}

fn report_rc(
    path: &std::path::Path,
    result: std::io::Result<crate::shell::rc::Change>,
) -> ExitCode {
    use crate::shell::rc::Change;
    let shown = paths::tilde(path, &process_env);
    match result {
        Ok(change) => {
            let msg = match change {
                Change::Added => Msg::InitAdded,
                Change::Updated => Msg::InitUpdated,
                Change::Unchanged => Msg::InitUnchanged,
                Change::Removed => Msg::InitRemoved,
                Change::NotPresent => Msg::InitNotPresent,
            };
            println!("{}", trf(msg, &[("path", &shown)]));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!(
                "{}",
                trf(
                    Msg::ErrRcWrite,
                    &[("path", &shown), ("detail", &e.to_string())]
                )
            );
            ExitCode::from(1)
        }
    }
}

fn init_command(args: InitArgs) -> ExitCode {
    use crate::shell::init::InitOptions;
    let Some(key) = parse_key(args.key) else {
        return ExitCode::from(2);
    };
    let script = crate::shell::init::script(InitOptions {
        shell: args.shell,
        key,
        bind: !args.no_bind,
        wrap: !args.no_wrap,
    });
    let mut out = std::io::stdout().lock();
    match out.write_all(script.as_bytes()).and_then(|_| out.flush()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::from(1),
    }
}

/// doctor용 인증 관리자 (Mock 시나리오 사용 시 None).
pub fn provider_auth(
    loaded: &config::Loaded,
) -> Result<Option<Arc<crate::auth::AuthManager>>, String> {
    provider::build(loaded).map(|b| b.auth)
}

fn run_main(cli: Cli, loaded: config::Loaded, rt: &tokio::runtime::Runtime) -> ExitCode {
    let env = process_env;
    let (shell, _) = shell::resolve(cli.shell, &env);
    let shell_version = env("HOWTUI_SHELL_VERSION");
    let ctx = EnvContext::current(shell, shell_version);
    let question = cli.question.join(" ");
    let widget_input = if cli.widget {
        match widget::WidgetInput::from_env(&env) {
            Some(w) => Some(w),
            None => return widget::missing_output(),
        }
    } else {
        None
    };

    let built = match provider::build(&loaded) {
        Ok(b) => b,
        Err(message) => {
            eprintln!("{}", trf(Msg::ErrConfig, &[("detail", &message)]));
            return ExitCode::from(8);
        }
    };

    // 폴더 목록 전송: 설정 화면에서 바꾼 값 > 설정 파일 context.folder (기본 끔, CR-007)
    let send_folder = built
        .auth
        .as_ref()
        .and_then(|a| a.load_state().send_folder)
        .unwrap_or(loaded.config.context.folder);
    let save_folder: Option<Arc<dyn Fn(bool) + Send + Sync>> = built.auth.clone().map(|auth| {
        Arc::new(move |on: bool| {
            let mut s = auth.load_state();
            s.send_folder = Some(on);
            auth.save_state(&s);
        }) as Arc<dyn Fn(bool) + Send + Sync>
    });

    if cli.no_tui {
        return no_tui::run(&question, built, ctx, send_folder, rt);
    }

    let wants_fullscreen = cli
        .mode
        .map(|m| m == ModeArg::Fullscreen)
        .unwrap_or(loaded.config.ui.mode == UiMode::Fullscreen);
    let startup_notice = wants_fullscreen.then(|| Status::warn(Msg::StatusFullscreenUnavailable));
    let height = cli.height.unwrap_or(loaded.config.ui.inline_height);
    // 새 버전: 지난 실행 캐시로 알림, 하루 지났으면 백그라운드 확인 (시작 대기 없음, CR-009)
    let update_enabled = loaded.config.update.check && !crate::update::disabled_by_env(&env);
    let update_notice = crate::update::plan(&env, update_enabled)
        .map(|p| crate::update::info_for(&p.latest, &p.method));
    if update_enabled {
        crate::update::maybe_spawn_check(rt.handle(), &env);
    }
    let cfg = TuiConfig {
        height,
        lang: i18n::lang(),
        env: ctx,
        logged_in: built.logged_in,
        initial_question: (widget_input.is_none() && !question.trim().is_empty())
            .then_some(question),
        initial_command: widget_input.as_ref().and_then(|w| w.initial_command()),
        startup_notice,
        keep_paste: widget_input.as_ref().is_some_and(|w| w.keep_paste),
        mouse: loaded.config.ui.mouse,
        update: update_notice,
        send_folder,
        save_folder,
    };
    // TUI 시작 전 판정 (TUI 중에는 fd 1이 터미널로 전환됨)
    let destination = widget::Destination::current(widget_input.as_ref(), &env);
    let login = built.login.clone();
    match runtime::run_tui(cfg, Arc::clone(&built.provider), login, rt.handle()) {
        Ok(TuiOutcome::Accepted(cmd)) => destination.deliver(&cmd),
        Ok(TuiOutcome::Cancelled) => ExitCode::from(130),
        Ok(TuiOutcome::Update(info)) => apply_update(rt, &info),
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

/// `--check-update`: 0 최신, 10 업데이트 있음, 11 접속 불가 (self-update-check 스킬).
fn check_update_command(rt: &tokio::runtime::Runtime) -> ExitCode {
    use crate::update::{self, CheckOutcome};
    let env = process_env;
    let current = update::CURRENT_VERSION;
    match rt.block_on(update::check_now(&env)) {
        CheckOutcome::UpdateAvailable { latest } => {
            let method = update::detect_install_method(&env).to_app();
            println!(
                "{}",
                trf(
                    Msg::CheckUpdateAvailable,
                    &[("current", current), ("latest", &latest)]
                )
            );
            println!(
                "{}",
                trf(
                    Msg::CheckUpdateHowTo,
                    &[("hint", &update::upgrade_hint(&method))]
                )
            );
            ExitCode::from(10)
        }
        CheckOutcome::UpToDate { latest } => {
            println!(
                "{}",
                trf(
                    Msg::UpdateUpToDate,
                    &[("current", current), ("latest", &latest)]
                )
            );
            ExitCode::SUCCESS
        }
        CheckOutcome::Unreachable => {
            eprintln!(
                "{}",
                trf(
                    Msg::CheckUpdateUnreachable,
                    &[("current", current), ("url", &update::latest_url(&env))]
                )
            );
            ExitCode::from(11)
        }
    }
}

/// `--update`: 0 완료 또는 최신, 1 실패, 11 접속 불가.
fn update_command(rt: &tokio::runtime::Runtime) -> ExitCode {
    use crate::update::{self, CheckOutcome};
    let env = process_env;
    match rt.block_on(update::check_now(&env)) {
        CheckOutcome::UpdateAvailable { latest } => {
            let method = update::detect_install_method(&env);
            apply_update(rt, &update::info_for(&latest, &method))
        }
        CheckOutcome::UpToDate { latest } => {
            println!(
                "{}",
                trf(
                    Msg::UpdateUpToDate,
                    &[("current", update::CURRENT_VERSION), ("latest", &latest)]
                )
            );
            ExitCode::SUCCESS
        }
        CheckOutcome::Unreachable => {
            eprintln!(
                "{}",
                trf(
                    Msg::CheckUpdateUnreachable,
                    &[
                        ("current", update::CURRENT_VERSION),
                        ("url", &update::latest_url(&env))
                    ]
                )
            );
            ExitCode::from(11)
        }
    }
}

/// 설치 스크립트 설치본은 내려받아 검증 후 교체, 그 외는 갱신 명령 안내 (외부 명령 실행 없음).
fn apply_update(rt: &tokio::runtime::Runtime, info: &crate::app::UpdateInfo) -> ExitCode {
    use crate::app::UpdateMethod;
    let env = process_env;
    let UpdateMethod::Script { dir } = &info.method else {
        eprintln!(
            "{}",
            trf(
                Msg::UpdateNotSelf,
                &[
                    ("method", &method_label(&info.method)),
                    ("command", &crate::update::upgrade_hint(&info.method)),
                ]
            )
        );
        return ExitCode::from(1);
    };
    let progress = |asset: &str| eprintln!("{}", trf(Msg::UpdateDownloading, &[("asset", asset)]));
    match rt.block_on(crate::update::apply(
        &env,
        &info.latest,
        std::path::Path::new(dir),
        &progress,
    )) {
        Ok(()) => {
            eprintln!(
                "{}",
                trf(
                    Msg::UpdateDone,
                    &[("current", &info.current), ("latest", &info.latest)]
                )
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}", trf(Msg::UpdateFailed, &[("detail", &e.to_string())]));
            ExitCode::from(1)
        }
    }
}

fn method_label(method: &crate::app::UpdateMethod) -> String {
    crate::ui::inline::method_label(i18n::lang(), method)
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
