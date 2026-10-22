//! TUI 이벤트 루프와 Effect 실행 (SRS 10절, ADR-0005).
//!
//! UI 스레드: 터미널 입력 읽기, 상태 갱신, 렌더링, 커서 질의.
//! tokio 작업 스레드: LLM 요청, 로그인, 시그널 수신. 결과는 채널로 UI 스레드에 전달.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::mpsc::{Sender, channel};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event};

use crate::app::{
    self, Action, Effect, ExitOutcome, LoginFailure, LoginInfo, LoginView, State, Status,
};
use crate::i18n::Lang;
use crate::llm::complete_with_retry;
use crate::llm::types::{CancelToken, EnvContext, LlmError, Provider, RequestId};
use crate::ui::terminal::{SessionError, TerminalSession};
use crate::ui::{inline, keymap};

pub const TICK: Duration = Duration::from_millis(120);
const IDLE_POLL: Duration = Duration::from_millis(500);

/// 로그인 진행 알림.
pub enum LoginProgress {
    Started(LoginView),
    InvalidPaste,
}

pub type LoginFuture<'a> =
    Pin<Box<dyn Future<Output = Result<LoginInfo, LoginFailure>> + Send + 'a>>;

/// TUI 안 로그인 (PRD-AUTH-013). 인증 구현과 런타임을 분리하는 경계.
pub trait LoginService: Send + Sync {
    fn login(
        &self,
        progress: Box<dyn Fn(LoginProgress) + Send + Sync>,
        paste: tokio::sync::mpsc::UnboundedReceiver<String>,
        cancel: CancelToken,
    ) -> LoginFuture<'_>;
}

pub struct TuiConfig {
    pub height: u16,
    pub lang: Lang,
    pub env: EnvContext,
    pub logged_in: bool,
    pub initial_question: Option<String>,
    /// 쉘 입력란 내용 (명령어 초기값)과 커서 앞 문자열
    pub initial_command: Option<(String, Option<String>)>,
    pub startup_notice: Option<Status>,
    pub keep_paste: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TuiOutcome {
    Accepted(String),
    Cancelled,
    Signal(i32),
}

enum Msg {
    Action(Action),
    Signal(i32),
}

struct LoginTask {
    cancel: CancelToken,
    paste: tokio::sync::mpsc::UnboundedSender<String>,
}

pub fn run_tui(
    cfg: TuiConfig,
    provider: Arc<dyn Provider>,
    login: Option<Arc<dyn LoginService>>,
    rt: &tokio::runtime::Handle,
) -> Result<TuiOutcome, SessionError> {
    let mut session = TerminalSession::start(cfg.height, cfg.keep_paste)?;
    let (tx, rx) = channel::<Msg>();
    spawn_signal_listener(rt, tx.clone());

    let mut state = State::new(cfg.lang, cfg.env.clone(), cfg.logged_in);
    state.provider = provider.label();
    state.status = cfg.startup_notice.clone();
    if let Some((buffer, lbuffer)) = &cfg.initial_command {
        state.command.set_text(buffer);
        if let Some(prefix) = lbuffer {
            state.command.set_cursor_after_prefix(prefix);
        }
        state.findings = crate::safety::analyze(buffer, state.env.shell);
    }
    let (w, h) = crossterm::terminal::size().unwrap_or((80, 24));
    let mut effects = app::update(
        &mut state,
        Action::Resize {
            width: w,
            height: h,
        },
    );
    if cfg.logged_in {
        spawn_prepare(rt, provider.clone(), tx.clone());
    }
    if let Some(q) = cfg
        .initial_question
        .as_deref()
        .filter(|q| !q.trim().is_empty())
    {
        state.question.set_text(q);
        if cfg.logged_in {
            effects.extend(app::update(&mut state, Action::Key(app::KeyAction::Submit)));
        } else {
            state.auto_submit = true;
        }
    }

    let mut requests: HashMap<RequestId, CancelToken> = HashMap::new();
    let mut login_task: Option<LoginTask> = None;
    let mut last_tick = Instant::now();

    let outcome = 'main: loop {
        for effect in effects.drain(..) {
            match effect {
                Effect::Exit(ExitOutcome::Accepted(cmd)) => break 'main TuiOutcome::Accepted(cmd),
                Effect::Exit(ExitOutcome::Cancelled) => break 'main TuiOutcome::Cancelled,
                Effect::StartRequest(req) => {
                    let cancel = CancelToken::new();
                    requests.insert(req.id, cancel.clone());
                    let provider = provider.clone();
                    let tx = tx.clone();
                    rt.spawn(async move {
                        let id = req.id;
                        let result = tokio::select! {
                            r = complete_with_retry(&*provider, req, cancel.clone()) => r,
                            _ = cancel.cancelled() => Err(LlmError::Cancelled),
                        };
                        let _ = tx.send(Msg::Action(Action::LlmDone { id, result }));
                    });
                }
                Effect::CancelRequest(id) => {
                    if let Some(token) = requests.remove(&id) {
                        token.cancel();
                    }
                }
                Effect::StartLogin => match &login {
                    Some(service) => {
                        login_task = Some(start_login(
                            rt,
                            service.clone(),
                            provider.clone(),
                            tx.clone(),
                        ))
                    }
                    None => {
                        let _ = tx.send(Msg::Action(Action::LoginDone(Err(LoginFailure::Other(
                            "login unavailable".into(),
                        )))));
                    }
                },
                Effect::CancelLogin => {
                    if let Some(task) = login_task.take() {
                        task.cancel.cancel();
                    }
                }
                Effect::SubmitRedirect(url) => {
                    if let Some(task) = &login_task {
                        let _ = task.paste.send(url);
                    }
                }
            }
        }

        if let Err(e) = session.draw(|f| inline::render(f, &state)) {
            return Err(SessionError::Io(e));
        }

        let mut actions = Vec::new();
        let timeout = if state.is_busy() { TICK } else { IDLE_POLL };
        match event::poll(timeout) {
            Ok(true) => match event::read() {
                Ok(Event::Key(key)) => actions.extend(keymap::map_key(key).map(Action::Key)),
                Ok(Event::Paste(text)) => actions.push(Action::Paste(text)),
                Ok(Event::Resize(width, height)) => actions.push(Action::Resize { width, height }),
                Ok(_) => {}
                Err(e) => return Err(SessionError::Io(e)),
            },
            Ok(false) => {}
            Err(e) => return Err(SessionError::Io(e)),
        }
        while let Ok(msg) = rx.try_recv() {
            match msg {
                Msg::Action(a) => actions.push(a),
                Msg::Signal(sig) => break 'main TuiOutcome::Signal(sig),
            }
        }
        if state.is_busy() && last_tick.elapsed() >= TICK {
            last_tick = Instant::now();
            actions.push(Action::Tick);
        }
        for action in actions {
            if let Action::LlmDone { id, .. } = &action {
                requests.remove(id);
            }
            let login_finished = matches!(&action, Action::LoginDone(r) if !matches!(r, Err(LoginFailure::InvalidPaste)));
            effects.extend(app::update(&mut state, action));
            if login_finished {
                login_task = None;
            }
        }
    };

    for token in requests.values() {
        token.cancel();
    }
    if let Some(task) = login_task {
        task.cancel.cancel();
    }
    session.restore()?;
    Ok(outcome)
}

fn start_login(
    rt: &tokio::runtime::Handle,
    service: Arc<dyn LoginService>,
    provider: Arc<dyn Provider>,
    tx: Sender<Msg>,
) -> LoginTask {
    let cancel = CancelToken::new();
    let (paste_tx, paste_rx) = tokio::sync::mpsc::unbounded_channel();
    let token = cancel.clone();
    rt.spawn(async move {
        let progress_tx = tx.clone();
        let progress = Box::new(move |p: LoginProgress| {
            let action = match p {
                LoginProgress::Started(view) => Action::LoginStarted(view),
                LoginProgress::InvalidPaste => Action::LoginDone(Err(LoginFailure::InvalidPaste)),
            };
            let _ = progress_tx.send(Msg::Action(action));
        });
        let result = service.login(progress, paste_rx, token).await;
        let ok = result.as_ref().is_ok_and(|info| info.plan_enabled);
        let _ = tx.send(Msg::Action(Action::LoginDone(result)));
        if ok && let Ok(label) = provider.prepare().await {
            let _ = tx.send(Msg::Action(Action::ProviderReady(label)));
        }
    });
    LoginTask {
        cancel,
        paste: paste_tx,
    }
}

fn spawn_prepare(rt: &tokio::runtime::Handle, provider: Arc<dyn Provider>, tx: Sender<Msg>) {
    rt.spawn(async move {
        let action = match provider.prepare().await {
            Ok(label) => Action::ProviderReady(label),
            Err(LlmError::AuthRequired) | Err(LlmError::AuthFailed { .. }) => Action::AuthLost,
            Err(e) => Action::Notice(app::update::error_status(&e)),
        };
        let _ = tx.send(Msg::Action(action));
    });
}

#[cfg(unix)]
fn spawn_signal_listener(rt: &tokio::runtime::Handle, tx: Sender<Msg>) {
    rt.spawn(async move {
        use tokio::signal::unix::{SignalKind, signal};
        let (Ok(mut term), Ok(mut hup), Ok(mut int)) = (
            signal(SignalKind::terminate()),
            signal(SignalKind::hangup()),
            signal(SignalKind::interrupt()),
        ) else {
            return;
        };
        let sig = tokio::select! {
            _ = term.recv() => 15,
            _ = hup.recv() => 1,
            _ = int.recv() => 2,
        };
        let _ = tx.send(Msg::Signal(sig));
    });
}

#[cfg(not(unix))]
fn spawn_signal_listener(_rt: &tokio::runtime::Handle, _tx: Sender<Msg>) {}
