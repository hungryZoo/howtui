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

/// 설치 방식 (CR-009). 업데이트 적용 방법을 정한다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateMethod {
    /// 설치 스크립트 설치본: howtui가 직접 내려받아 검증 후 교체
    Script {
        dir: String,
    },
    Cargo,
    Brew,
    Unknown,
}

/// 새 버전 정보 (지난 실행이 남긴 캐시 또는 즉시 확인 결과).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub method: UpdateMethod,
}

/// 업데이트 화면 상태.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateView {
    Checking,
    Ready(UpdateInfo),
    UpToDate { current: String, latest: String },
    Unreachable,
}

/// 마우스 클릭 대상. 렌더러가 화면 영역과 함께 기록하고 런타임이 클릭 좌표로 찾는다 (CR-005).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    OpenSettings,
    ToggleFolder,
    OpenUpdate,
    ConfirmUpdate,
    CloseUpdate,
    Question,
    Command,
    SettingsItem(usize),
    CloseSettings,
}

/// 설정 화면 항목: 에이전트 목록, 모델 목록, 폴더, 업데이트, 로그아웃 순서.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsEntry {
    /// 에이전트 전환 (CR-011)
    Agent(usize),
    Model(usize),
    Folder,
    Update,
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
    /// 전환 요청 중인 에이전트 표시 이름 (CR-011)
    pub pending_agent: Option<String>,
}

impl SettingsView {
    pub fn entries(&self) -> Vec<SettingsEntry> {
        let Some(d) = &self.data else {
            return Vec::new();
        };
        let mut v: Vec<SettingsEntry> = (0..d.agents.len()).map(SettingsEntry::Agent).collect();
        v.extend((0..d.models.len()).map(SettingsEntry::Model));
        v.push(SettingsEntry::Folder);
        v.push(SettingsEntry::Update);
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
    /// 에이전트 전환 결과 (CR-011)
    AgentSelected(Result<ProviderLabel, LlmError>),
    SignedOut,
    UpdateChecked(UpdateView),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExitOutcome {
    Accepted(String),
    Cancelled,
    /// 업데이트 화면에서 Enter: TUI 종료 후 적용 또는 안내 (CR-009)
    Update(UpdateInfo),
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
    /// 에이전트 전환 (CR-011)
    SelectAgent(String),
    /// Provider 준비 작업 재실행 (전환 후 모델 확인, 로그인 상태 판정)
    Prepare,
    /// 폴더 목록 전송 설정 저장
    SaveFolder(bool),
    /// 즉시 업데이트 확인 (업데이트 화면)
    CheckUpdate,
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
    /// 새 버전 알림 (캐시 기준, CR-009)
    pub update: Option<UpdateInfo>,
    pub update_view: Option<UpdateView>,
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
            update: None,
            update_view: None,
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
            || self.update_view == Some(UpdateView::Checking)
    }
}
