//! 상태 갱신 (PRD 3절 상태 전이 표, SRS 7.1절). 순수 함수: 입출력, 시계, 네트워크 접근 없음.

use super::editor::EditOp;
use super::state::*;
use crate::i18n::Msg;
use crate::llm::types::{LlmError, LlmRequest, Operation, Turn};
use crate::safety::{self, Level};

pub fn update(state: &mut State, action: Action) -> Vec<Effect> {
    match action {
        Action::Resize { width, height } => {
            state.too_small = width < MIN_WIDTH || height < MIN_HEIGHT;
            Vec::new()
        }
        Action::Tick => {
            state.spinner = state.spinner.wrapping_add(1);
            Vec::new()
        }
        Action::ProviderReady(label) => {
            state.provider = label;
            Vec::new()
        }
        Action::Notice(status) => {
            state.status = Some(status);
            Vec::new()
        }
        Action::AuthLost => {
            if matches!(state.phase, Phase::Ready) {
                state.phase = Phase::LoginRequired;
                state.status = Some(error_status(&LlmError::AuthRequired));
            }
            Vec::new()
        }
        Action::LlmDone { id, result } => on_llm_done(state, id, result),
        Action::LoginStarted(view) => {
            if state.phase == Phase::LoggingIn {
                state.login = view;
            }
            Vec::new()
        }
        Action::LoginDone(result) => on_login_done(state, result),
        Action::Paste(text) => on_paste(state, &text),
        Action::Key(key) => on_key(state, key),
    }
}

fn on_key(state: &mut State, key: KeyAction) -> Vec<Effect> {
    // 크기 부족 상태: Esc와 Ctrl+C만 처리 (PRD-UI-009)
    if state.too_small && !matches!(key, KeyAction::Cancel | KeyAction::Quit) {
        return Vec::new();
    }
    match state.phase {
        Phase::LoginRequired => match key {
            KeyAction::Submit => {
                state.phase = Phase::LoggingIn;
                state.login = LoginView::default();
                state.login_paste.clear();
                state.status = None;
                vec![Effect::StartLogin]
            }
            KeyAction::Cancel | KeyAction::Quit => vec![Effect::Exit(ExitOutcome::Cancelled)],
            _ => Vec::new(),
        },
        Phase::LoggingIn => match key {
            KeyAction::Cancel => {
                state.phase = Phase::LoginRequired;
                state.status = Some(Status::info(Msg::LoginCancelled));
                vec![Effect::CancelLogin]
            }
            KeyAction::Quit => vec![Effect::CancelLogin, Effect::Exit(ExitOutcome::Cancelled)],
            KeyAction::Submit => {
                let text = state.login_paste.text().trim().to_string();
                if text.is_empty() {
                    Vec::new()
                } else {
                    vec![Effect::SubmitRedirect(text)]
                }
            }
            KeyAction::Edit(op) => {
                state.login_paste.apply(op);
                Vec::new()
            }
            _ => Vec::new(),
        },
        Phase::Requesting => match key {
            KeyAction::Cancel => cancel_pending(state),
            KeyAction::Quit => {
                let mut effects = cancel_pending(state);
                effects.push(Effect::Exit(ExitOutcome::Cancelled));
                effects
            }
            KeyAction::ToggleFocus => {
                toggle_focus(state);
                Vec::new()
            }
            KeyAction::Edit(op) if is_move(op) => {
                focused(state).apply(op);
                Vec::new()
            }
            _ => Vec::new(),
        },
        Phase::ConfirmDanger => match key {
            KeyAction::Submit => vec![Effect::Exit(ExitOutcome::Accepted(state.command.text()))],
            KeyAction::Quit => vec![Effect::Exit(ExitOutcome::Cancelled)],
            _ => {
                state.phase = Phase::Ready;
                state.status = None;
                Vec::new()
            }
        },
        Phase::Ready => on_ready_key(state, key),
    }
}

fn on_ready_key(state: &mut State, key: KeyAction) -> Vec<Effect> {
    match key {
        KeyAction::Cancel | KeyAction::Quit => vec![Effect::Exit(ExitOutcome::Cancelled)],
        KeyAction::ToggleFocus => {
            toggle_focus(state);
            Vec::new()
        }
        KeyAction::Submit => match state.focus {
            // 빈 질문에서 Enter는 현재 명령어 확정 (CR-001)
            Focus::Question if state.question.is_blank() && state.has_command() => {
                accept_command(state)
            }
            Focus::Question => submit_question(state),
            Focus::Command => accept_command(state),
        },
        KeyAction::InsertNewline => {
            if state.focus == Focus::Command && state.command.apply(EditOp::Newline) {
                after_command_edit(state);
            }
            Vec::new()
        }
        KeyAction::Explain => {
            if state.has_command() {
                start_request(state, Operation::Explain, None)
            } else {
                Vec::new()
            }
        }
        KeyAction::Undo | KeyAction::Redo => {
            // 빈 질문 영역의 Undo/Redo는 명령어 대상 (AI 반영 되돌리기, CR-001)
            let on_command = state.focus == Focus::Command || state.question.is_empty();
            let buffer = if on_command {
                &mut state.command
            } else {
                &mut state.question
            };
            let changed = if key == KeyAction::Undo {
                buffer.undo()
            } else {
                buffer.redo()
            };
            if changed && on_command {
                after_command_edit(state);
            }
            Vec::new()
        }
        KeyAction::Edit(op) => {
            // 질문 영역은 한 줄 입력
            if state.focus == Focus::Question && op == EditOp::Newline {
                return Vec::new();
            }
            if state.focus == Focus::Question && op == EditOp::Down && state.has_command() {
                state.focus = Focus::Command;
                return Vec::new();
            }
            if state.focus == Focus::Command && op == EditOp::Up && state.command.cursor().0 == 0 {
                state.focus = Focus::Question;
                return Vec::new();
            }
            let changed = focused(state).apply(op);
            if changed && state.focus == Focus::Command {
                after_command_edit(state);
            }
            Vec::new()
        }
        KeyAction::Other => Vec::new(),
    }
}

fn on_paste(state: &mut State, text: &str) -> Vec<Effect> {
    match state.phase {
        Phase::LoggingIn => {
            // 콜백 주소 붙여넣기는 Enter 없이 바로 제출 (CR-002)
            if text.contains("/auth/callback") {
                state.login_paste.clear();
                return vec![Effect::SubmitRedirect(text.trim().to_string())];
            }
            state.login_paste.insert_str(text.trim());
        }
        Phase::Ready | Phase::ConfirmDanger => {
            if state.phase == Phase::ConfirmDanger {
                state.phase = Phase::Ready;
                state.status = None;
            }
            match state.focus {
                // 질문 영역 붙여넣기는 한 줄로 결합
                Focus::Question => {
                    let joined = text.replace("\r\n", " ").replace(['\n', '\r'], " ");
                    state.question.insert_str(&joined);
                }
                Focus::Command => {
                    if state.command.insert_str(text) {
                        after_command_edit(state);
                    }
                }
            }
        }
        _ => {}
    }
    Vec::new()
}

fn is_move(op: EditOp) -> bool {
    matches!(
        op,
        EditOp::Left
            | EditOp::Right
            | EditOp::Up
            | EditOp::Down
            | EditOp::Home
            | EditOp::End
            | EditOp::WordLeft
            | EditOp::WordRight
    )
}

fn focused(state: &mut State) -> &mut super::editor::TextBuffer {
    match state.focus {
        Focus::Question => &mut state.question,
        Focus::Command => &mut state.command,
    }
}

fn toggle_focus(state: &mut State) {
    state.focus = match state.focus {
        Focus::Question => Focus::Command,
        Focus::Command => Focus::Question,
    };
}

fn after_command_edit(state: &mut State) {
    if !state.explanation.text.is_empty() {
        state.explanation.stale = true;
    }
    state.findings = safety::analyze(&state.command.text(), state.env.shell);
    if matches!(
        state.status.as_ref().map(|s| s.msg),
        Some(Msg::StatusConfirmDanger)
    ) {
        state.status = None;
    }
}

fn submit_question(state: &mut State) -> Vec<Effect> {
    if state.question.is_blank() {
        return Vec::new();
    }
    let question = state.question.text().trim().to_string();
    let op = if state.has_command() {
        Operation::Revise
    } else {
        Operation::Generate
    };
    start_request(state, op, Some(question))
}

fn accept_command(state: &mut State) -> Vec<Effect> {
    if !state.has_command() {
        return Vec::new();
    }
    state.findings = safety::analyze(&state.command.text(), state.env.shell);
    if safety::max_level(&state.findings) == Some(Level::High) {
        state.phase = Phase::ConfirmDanger;
        state.status = Some(Status::warn(Msg::StatusConfirmDanger));
        return Vec::new();
    }
    vec![Effect::Exit(ExitOutcome::Accepted(state.command.text()))]
}

fn start_request(state: &mut State, op: Operation, question: Option<String>) -> Vec<Effect> {
    let id = state.next_request_id;
    state.next_request_id += 1;
    let command = state.has_command().then(|| state.command.text());
    let skip = state.conversation.len().saturating_sub(HISTORY_LIMIT);
    let request = LlmRequest {
        id,
        op,
        env: state.env.clone(),
        question: question.clone(),
        command: if op == Operation::Generate {
            None
        } else {
            command.clone()
        },
        history: state.conversation[skip..].to_vec(),
    };
    state.pending = Some(Pending {
        id,
        op,
        return_phase: Phase::Ready,
        question,
        command_sent: request.command.clone(),
    });
    state.phase = Phase::Requesting;
    state.command.set_read_only(true);
    state.status = None;
    vec![Effect::StartRequest(request)]
}

fn cancel_pending(state: &mut State) -> Vec<Effect> {
    let Some(pending) = state.pending.take() else {
        return Vec::new();
    };
    state.phase = pending.return_phase;
    state.command.set_read_only(false);
    state.status = Some(Status::info(Msg::StatusCancelled));
    vec![Effect::CancelRequest(pending.id)]
}

fn on_llm_done(
    state: &mut State,
    id: u64,
    result: Result<crate::llm::types::LlmResponse, LlmError>,
) -> Vec<Effect> {
    // 취소되었거나 이전 요청의 결과는 폐기 (SRS-ASYNC-003)
    let Some(pending) = state.pending.as_ref().filter(|p| p.id == id).cloned() else {
        return Vec::new();
    };
    state.pending = None;
    state.command.set_read_only(false);
    state.phase = pending.return_phase;
    match result {
        Ok(response) => {
            match pending.op {
                Operation::Generate | Operation::Revise => {
                    if let Some(cmd) = response.command.as_deref().filter(|c| !c.trim().is_empty())
                    {
                        state.command.replace_undoable(cmd);
                        state.focus = Focus::Question;
                    }
                    state.last_question = pending.question.clone();
                    state.question.clear();
                }
                Operation::Explain => {}
            }
            state.explanation = Explanation {
                text: response.explanation.clone(),
                stale: false,
            };
            state.notes = response.notes.clone();
            state.findings = safety::analyze(&state.command.text(), state.env.shell);
            state.status = None;
            state.conversation.push(Turn {
                op: pending.op,
                question: pending.question,
                command_sent: pending.command_sent,
                response,
            });
            Vec::new()
        }
        Err(LlmError::Cancelled) => Vec::new(),
        Err(LlmError::AuthRequired) => {
            state.phase = Phase::LoginRequired;
            state.status = Some(error_status(&LlmError::AuthRequired));
            Vec::new()
        }
        Err(err) => {
            state.status = Some(error_status(&err));
            Vec::new()
        }
    }
}

fn on_login_done(state: &mut State, result: Result<LoginInfo, LoginFailure>) -> Vec<Effect> {
    if state.phase != Phase::LoggingIn {
        return Vec::new();
    }
    state.login_paste.clear();
    // 잘못된 붙여넣기: 로그인 대기와 URL 표시 유지
    if matches!(result, Err(LoginFailure::InvalidPaste)) {
        state.status = Some(Status::error(Msg::LoginInvalidPaste, Vec::new()));
        return Vec::new();
    }
    state.login = LoginView::default();
    match result {
        Ok(info) => {
            state.phase = Phase::Ready;
            state.status = if !info.plan_enabled {
                Some(Status::error(Msg::LoginPlanDisabled, Vec::new()))
            } else if info.show_plan_notice {
                Some(Status::info(Msg::PlanNotice))
            } else {
                None
            };
            if state.auto_submit && info.plan_enabled {
                state.auto_submit = false;
                return submit_question(state);
            }
            Vec::new()
        }
        Err(failure) => {
            state.phase = Phase::LoginRequired;
            let msg = match failure {
                LoginFailure::Cancelled => Msg::LoginCancelled,
                LoginFailure::Denied => Msg::LoginDenied,
                LoginFailure::StateMismatch => Msg::LoginStateMismatch,
                LoginFailure::InvalidPaste => {
                    // 잘못된 붙여넣기는 로그인 대기 유지
                    state.phase = Phase::LoggingIn;
                    Msg::LoginInvalidPaste
                }
                LoginFailure::Timeout => Msg::LoginTimeout,
                LoginFailure::PlanDisabled => Msg::LoginPlanDisabled,
                LoginFailure::Other(detail) => {
                    state.status = Some(Status::error(Msg::ErrInternal, vec![("detail", detail)]));
                    return Vec::new();
                }
            };
            state.status = Some(Status::error(msg, Vec::new()));
            Vec::new()
        }
    }
}

/// `LlmError` → 상태줄 메시지 (원인 + 조치, SRS-ERR-002).
pub fn error_status(err: &LlmError) -> Status {
    let (msg, args) = error_message(err);
    Status::error(msg, args)
}

pub fn error_message(err: &LlmError) -> (Msg, Vec<(&'static str, String)>) {
    match err {
        LlmError::AuthRequired => (Msg::ErrAuthRequired, vec![]),
        LlmError::AuthFailed { .. } => (Msg::ErrAuthFailed, vec![]),
        LlmError::NotEligible => (Msg::ErrNotEligible, vec![]),
        LlmError::ClientNotEnabled => (Msg::ErrClientNotEnabled, vec![]),
        LlmError::PlanNotGranted => (Msg::LoginPlanDisabled, vec![]),
        LlmError::PermissionConfig { code } => {
            (Msg::ErrPermissionConfig, vec![("code", code.clone())])
        }
        LlmError::RegionOrPolicy { detail } => {
            (Msg::ErrRegionOrPolicy, vec![("detail", detail.clone())])
        }
        LlmError::UsageLimit => (Msg::ErrUsageLimit, vec![]),
        LlmError::Unsupported { param } => (
            Msg::ErrUnsupported,
            vec![("param", param.clone().unwrap_or_else(|| "-".into()))],
        ),
        LlmError::ModelNotFound => (Msg::ErrModelNotFound, vec![]),
        LlmError::Transient { .. } => (Msg::ErrTransient, vec![]),
        LlmError::Network { detail } => (Msg::ErrNetwork, vec![("detail", detail.clone())]),
        LlmError::InvalidResponse { .. } => (Msg::ErrInvalidResponse, vec![]),
        LlmError::Timeout => (Msg::ErrTimeout, vec![]),
        LlmError::Cancelled => (Msg::StatusCancelled, vec![]),
    }
}

#[cfg(test)]
mod tests;
