//! 애플리케이션 상태, Action, Effect (SRS 4절, ADR-0004).

use super::editor::{EditOp, TextBuffer};
use crate::i18n::{Lang, Msg};
use crate::llm::types::{
    EnvContext, LlmError, LlmRequest, LlmResponse, ProviderLabel, RequestId, Turn,
};
use crate::safety::Finding;

/// 요청 시 포함하는 최근 이력 수 (SRS-DATA-005).
pub const HISTORY_LIMIT: usize = 6;
pub const MIN_WIDTH: u16 = 30;
pub const MIN_HEIGHT: u16 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    LoginRequired,
    LoggingIn,
    /// 입력 대기 (명령어 없음) 또는 편집 중 (명령어 있음).
    Ready,
    Requesting,
    ConfirmDanger,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Question,
    Command,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub kind: StatusKind,
    pub msg: Msg,
    pub args: Vec<(&'static str, String)>,
}

impl Status {
    pub fn info(msg: Msg) -> Self {
        Self {
            kind: StatusKind::Info,
            msg,
            args: Vec::new(),
        }
    }
    pub fn warn(msg: Msg) -> Self {
        Self {
            kind: StatusKind::Warn,
            msg,
            args: Vec::new(),
        }
    }
    pub fn error(msg: Msg, args: Vec<(&'static str, String)>) -> Self {
        Self {
            kind: StatusKind::Error,
            msg,
            args,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Explanation {
    pub text: String,
    pub stale: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pending {
    pub id: RequestId,
    pub op: crate::llm::types::Operation,
    pub return_phase: Phase,
    pub question: Option<String>,
    pub command_sent: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoginView {
    pub url: Option<String>,
    pub port: Option<u16>,
    pub browser_failed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoginFailure {
    Cancelled,
    Denied,
    StateMismatch,
    InvalidPaste,
    Timeout,
    PlanDisabled,
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoginInfo {
    pub email: Option<String>,
    pub plan_enabled: bool,
    pub show_plan_notice: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyAction {
    Submit,
    InsertNewline,
    ToggleFocus,
    Cancel,
    Quit,
    Explain,
    Undo,
    Redo,
    Edit(EditOp),
    /// 그 외 키 (위험 확인 해제용)
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Key(KeyAction),
    Paste(String),
    Resize {
        width: u16,
        height: u16,
    },
    Tick,
    LlmDone {
        id: RequestId,
        result: Result<LlmResponse, LlmError>,
    },
    LoginStarted(LoginView),
    LoginDone(Result<LoginInfo, LoginFailure>),
    ProviderReady(ProviderLabel),
    /// 시작 준비 중 인증 정보 무효 확인
    AuthLost,
    Notice(Status),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExitOutcome {
    Accepted(String),
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    StartRequest(LlmRequest),
    CancelRequest(RequestId),
    StartLogin,
    CancelLogin,
    SubmitRedirect(String),
    Exit(ExitOutcome),
}

#[derive(Clone, Debug)]
pub struct State {
    pub lang: Lang,
    pub phase: Phase,
    pub focus: Focus,
    pub question: TextBuffer,
    pub command: TextBuffer,
    pub explanation: Explanation,
    pub notes: Vec<String>,
    pub findings: Vec<Finding>,
    pub conversation: Vec<Turn>,
    pub pending: Option<Pending>,
    pub status: Option<Status>,
    pub env: EnvContext,
    pub provider: ProviderLabel,
    pub too_small: bool,
    pub spinner: usize,
    pub last_question: Option<String>,
    pub login: LoginView,
    pub login_paste: TextBuffer,
    /// 로그인 완료 직후 질문 자동 제출 여부 (CLI 질문 인자).
    pub auto_submit: bool,
    pub next_request_id: RequestId,
}

impl State {
    pub fn new(lang: Lang, env: EnvContext, logged_in: bool) -> Self {
        Self {
            lang,
            phase: if logged_in {
                Phase::Ready
            } else {
                Phase::LoginRequired
            },
            focus: Focus::Question,
            question: TextBuffer::new(),
            command: TextBuffer::new(),
            explanation: Explanation::default(),
            notes: Vec::new(),
            findings: Vec::new(),
            conversation: Vec::new(),
            pending: None,
            status: None,
            env,
            provider: ProviderLabel::default(),
            too_small: false,
            spinner: 0,
            last_question: None,
            login: LoginView::default(),
            login_paste: TextBuffer::new(),
            auto_submit: false,
            next_request_id: 1,
        }
    }

    pub fn has_command(&self) -> bool {
        !self.command.is_blank()
    }

    pub fn is_busy(&self) -> bool {
        matches!(self.phase, Phase::Requesting | Phase::LoggingIn)
    }
}
