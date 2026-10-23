//! 애플리케이션 상태, Action, Effect (SRS 4절, ADR-0004).

use super::editor::{EditOp, TextBuffer};
use crate::i18n::{Lang, Msg};
use crate::llm::types::{
    EnvContext, LlmError, LlmRequest, LlmResponse, ProviderLabel, ProviderSettings, RequestId, Turn,
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

/// 마우스 클릭 대상. 렌더러가 화면 영역과 함께 기록하고 런타임이 클릭 좌표로 찾는다 (CR-005).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    OpenSettings,
    ToggleFolder,
    Question,
    Command,
    SettingsItem(usize),
    CloseSettings,
}

/// 설정 화면 항목: 모델 목록 다음에 로그아웃.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsEntry {
    Model(usize),
    Folder,
    SignOut,
}

/// 설정 화면 (에이전트와 모델, CR-005). 열려 있는 동안 메인 화면 대신 표시.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SettingsView {
    pub data: Option<ProviderSettings>,
    pub cursor: usize,
    /// 모델 변경 또는 로그아웃 진행 중
    pub applying: bool,
    pub error: Option<Status>,
}

impl SettingsView {
    pub fn entries(&self) -> Vec<SettingsEntry> {
        let Some(d) = &self.data else {
            return Vec::new();
        };
        let mut v: Vec<SettingsEntry> = (0..d.models.len()).map(SettingsEntry::Model).collect();
        v.push(SettingsEntry::Folder);
        if d.can_sign_out {
            v.push(SettingsEntry::SignOut);
        }
        v
    }
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
    /// 설정 화면 열기와 닫기 (Ctrl+O)
    Settings,
    /// 현재 폴더 목록 전송 켜기와 끄기 (Ctrl+F, CR-007)
    ToggleFolder,
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
    Click(Hit),
    /// 휠 (음수 위, 양수 아래)
    Scroll(i8),
    SettingsLoaded(Result<ProviderSettings, LlmError>),
    ModelSelected(Result<ProviderLabel, LlmError>),
    SignedOut,
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
    LoadSettings,
    SelectModel(String),
    /// 폴더 목록 전송 설정 저장
    SaveFolder(bool),
    SignOut,
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
    pub settings: Option<SettingsView>,
    /// 현재 폴더 목록 전송 (CR-007)
    pub send_folder: bool,
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
            settings: None,
            send_folder: false,
        }
    }

    pub fn has_command(&self) -> bool {
        !self.command.is_blank()
    }

    pub fn is_busy(&self) -> bool {
        matches!(self.phase, Phase::Requesting | Phase::LoggingIn)
            || self
                .settings
                .as_ref()
                .is_some_and(|s| (s.data.is_none() && s.error.is_none()) || s.applying)
    }
}
