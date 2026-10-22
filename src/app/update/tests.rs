//! PRD 3절 상태 전이 표 행 단위 테스트 (TC-UI-006, TC-UI-007, TC-EDIT-006-008, TC-LLM-002-004).

use super::*;
use crate::i18n::Lang;
use crate::llm::types::{EnvContext, LlmResponse, OsKind, ProviderLabel};
use crate::shell::ShellKind;

fn env() -> EnvContext {
    EnvContext {
        os: OsKind::Linux,
        arch: "x86_64",
        shell: ShellKind::Bash,
        shell_version: None,
    }
}

fn ready() -> State {
    State::new(Lang::En, env(), true)
}

fn key(state: &mut State, k: KeyAction) -> Vec<Effect> {
    update(state, Action::Key(k))
}

fn type_text(state: &mut State, text: &str) {
    for c in text.chars() {
        key(state, KeyAction::Edit(EditOp::Insert(c)));
    }
}

fn response(cmd: Option<&str>, expl: &str) -> LlmResponse {
    LlmResponse {
        command: cmd.map(str::to_string),
        explanation: expl.into(),
        notes: vec![],
        model: "m".into(),
    }
}

fn request_of(effects: &[Effect]) -> LlmRequest {
    match effects {
        [Effect::StartRequest(r)] => r.clone(),
        other => panic!("StartRequest 기대, 실제 {other:?}"),
    }
}

/// 질문 제출 후 성공 응답까지 진행.
fn generate(state: &mut State, question: &str, cmd: &str) {
    type_text(state, question);
    let req = request_of(&key(state, KeyAction::Submit));
    update(
        state,
        Action::LlmDone {
            id: req.id,
            result: Ok(response(Some(cmd), "explains")),
        },
    );
}

#[test]
fn start_without_credentials_requires_login() {
    let s = State::new(Lang::En, env(), false);
    assert_eq!(s.phase, Phase::LoginRequired);
}

#[test]
fn start_with_credentials_waits_for_question() {
    let s = ready();
    assert_eq!((s.phase, s.focus), (Phase::Ready, Focus::Question));
}

#[test]
fn login_required_enter_starts_login() {
    let mut s = State::new(Lang::En, env(), false);
    assert_eq!(key(&mut s, KeyAction::Submit), vec![Effect::StartLogin]);
    assert_eq!(s.phase, Phase::LoggingIn);
}

#[test]
fn login_success_shows_plan_notice_once() {
    let mut s = State::new(Lang::En, env(), false);
    key(&mut s, KeyAction::Submit);
    update(
        &mut s,
        Action::LoginDone(Ok(LoginInfo {
            email: None,
            plan_enabled: true,
            show_plan_notice: true,
        })),
    );
    assert_eq!(s.phase, Phase::Ready);
    assert_eq!(s.status.as_ref().map(|st| st.msg), Some(Msg::PlanNotice));
}

#[test]
fn login_esc_and_failure_return_to_login_required() {
    let mut s = State::new(Lang::En, env(), false);
    key(&mut s, KeyAction::Submit);
    assert_eq!(key(&mut s, KeyAction::Cancel), vec![Effect::CancelLogin]);
    assert_eq!(s.phase, Phase::LoginRequired);
    key(&mut s, KeyAction::Submit);
    update(&mut s, Action::LoginDone(Err(LoginFailure::Denied)));
    assert_eq!(s.phase, Phase::LoginRequired);
    assert_eq!(s.status.as_ref().map(|st| st.msg), Some(Msg::LoginDenied));
}

#[test]
fn login_paste_submits_redirect_and_invalid_paste_keeps_waiting() {
    let mut s = State::new(Lang::En, env(), false);
    key(&mut s, KeyAction::Submit);
    update(
        &mut s,
        Action::Paste("  http://127.0.0.1:1455/auth/callback?code=x&state=y \n".into()),
    );
    assert_eq!(
        key(&mut s, KeyAction::Submit),
        vec![Effect::SubmitRedirect(
            "http://127.0.0.1:1455/auth/callback?code=x&state=y".into()
        )]
    );
    update(&mut s, Action::LoginDone(Err(LoginFailure::InvalidPaste)));
    assert_eq!(s.phase, Phase::LoggingIn);
}

#[test]
fn login_with_plan_disabled_reports_and_does_not_auto_submit() {
    let mut s = State::new(Lang::En, env(), false);
    type_text(&mut s, "x");
    s.auto_submit = true;
    s.phase = Phase::LoggingIn;
    let effects = update(
        &mut s,
        Action::LoginDone(Ok(LoginInfo {
            email: None,
            plan_enabled: false,
            show_plan_notice: false,
        })),
    );
    assert!(effects.is_empty());
    assert_eq!(
        s.status.as_ref().map(|st| st.msg),
        Some(Msg::LoginPlanDisabled)
    );
}

#[test]
fn auto_submit_after_login() {
    let mut s = State::new(Lang::En, env(), false);
    s.question.set_text("list files");
    s.auto_submit = true;
    s.phase = Phase::LoggingIn;
    let effects = update(
        &mut s,
        Action::LoginDone(Ok(LoginInfo {
            email: None,
            plan_enabled: true,
            show_plan_notice: false,
        })),
    );
    assert_eq!(request_of(&effects).question.as_deref(), Some("list files"));
}

#[test]
fn question_enter_without_command_generates() {
    let mut s = ready();
    type_text(&mut s, "find big files");
    let req = request_of(&key(&mut s, KeyAction::Submit));
    assert_eq!(req.op, Operation::Generate);
    assert_eq!(req.question.as_deref(), Some("find big files"));
    assert_eq!(req.command, None);
    assert_eq!(s.phase, Phase::Requesting);
}

#[test]
fn empty_question_is_ignored() {
    let mut s = ready();
    type_text(&mut s, "   ");
    assert!(key(&mut s, KeyAction::Submit).is_empty());
    assert_eq!(s.phase, Phase::Ready);
}

#[test]
fn success_applies_command_and_moves_focus() {
    let mut s = ready();
    generate(&mut s, "list", "ls -la");
    assert_eq!(s.phase, Phase::Ready);
    assert_eq!(s.focus, Focus::Command);
    assert_eq!(s.command.text(), "ls -la");
    assert_eq!(
        s.explanation,
        Explanation {
            text: "explains".into(),
            stale: false
        }
    );
    assert!(s.question.is_empty());
    assert_eq!(s.last_question.as_deref(), Some("list"));
    assert_eq!(s.conversation.len(), 1);
}

#[test]
fn follow_up_revises_with_user_edit_and_undo_restores_edit() {
    let mut s = ready();
    generate(&mut s, "list", "ls -la");
    // 사용자 직접 편집
    key(&mut s, KeyAction::Edit(EditOp::End));
    type_text(&mut s, " /tmp");
    assert!(s.explanation.stale, "편집 후 설명 갱신 필요 표시");
    key(&mut s, KeyAction::ToggleFocus);
    type_text(&mut s, "sort by size");
    let req = request_of(&key(&mut s, KeyAction::Submit));
    assert_eq!(req.op, Operation::Revise);
    assert_eq!(req.command.as_deref(), Some("ls -la /tmp"));
    assert_eq!(req.history.len(), 1);
    // 요청 중 편집 잠금
    s.focus = Focus::Command;
    key(&mut s, KeyAction::Edit(EditOp::Insert('x')));
    assert_eq!(s.command.text(), "ls -la /tmp");
    update(
        &mut s,
        Action::LlmDone {
            id: req.id,
            result: Ok(response(Some("ls -laS /tmp"), "sorted")),
        },
    );
    assert_eq!(s.command.text(), "ls -laS /tmp");
    key(&mut s, KeyAction::Undo);
    assert_eq!(s.command.text(), "ls -la /tmp", "AI 반영은 Undo 1단계");
}

#[test]
fn explain_updates_only_explanation() {
    let mut s = ready();
    generate(&mut s, "list", "ls -la");
    key(&mut s, KeyAction::Edit(EditOp::Backspace));
    let req = request_of(&key(&mut s, KeyAction::Explain));
    assert_eq!(req.op, Operation::Explain);
    assert_eq!(req.command.as_deref(), Some("ls -l"));
    update(
        &mut s,
        Action::LlmDone {
            id: req.id,
            result: Ok(response(None, "long listing")),
        },
    );
    assert_eq!(s.command.text(), "ls -l");
    assert_eq!(
        s.explanation,
        Explanation {
            text: "long listing".into(),
            stale: false
        }
    );
}

#[test]
fn esc_cancels_request_and_late_response_is_dropped() {
    let mut s = ready();
    generate(&mut s, "list", "ls");
    key(&mut s, KeyAction::ToggleFocus);
    type_text(&mut s, "more");
    let req = request_of(&key(&mut s, KeyAction::Submit));
    assert_eq!(
        key(&mut s, KeyAction::Cancel),
        vec![Effect::CancelRequest(req.id)]
    );
    assert_eq!(s.phase, Phase::Ready);
    assert_eq!(
        s.status.as_ref().map(|st| st.msg),
        Some(Msg::StatusCancelled)
    );
    update(
        &mut s,
        Action::LlmDone {
            id: req.id,
            result: Ok(response(Some("rm -rf /"), "late")),
        },
    );
    assert_eq!(s.command.text(), "ls", "취소 후 도착 응답 폐기");
    assert_eq!(s.question.text(), "more", "취소 시 질문 보존");
}

#[test]
fn error_keeps_previous_state_and_reports() {
    let mut s = ready();
    generate(&mut s, "list", "ls");
    key(&mut s, KeyAction::ToggleFocus);
    type_text(&mut s, "more");
    let req = request_of(&key(&mut s, KeyAction::Submit));
    update(
        &mut s,
        Action::LlmDone {
            id: req.id,
            result: Err(LlmError::UsageLimit),
        },
    );
    assert_eq!(s.phase, Phase::Ready);
    assert_eq!(s.command.text(), "ls");
    assert_eq!(s.question.text(), "more");
    assert_eq!(s.status.as_ref().map(|st| st.msg), Some(Msg::ErrUsageLimit));
    // 재시도 가능
    assert!(matches!(
        key(&mut s, KeyAction::Submit).as_slice(),
        [Effect::StartRequest(_)]
    ));
}

#[test]
fn auth_required_error_returns_to_login() {
    let mut s = ready();
    type_text(&mut s, "q");
    let req = request_of(&key(&mut s, KeyAction::Submit));
    update(
        &mut s,
        Action::LlmDone {
            id: req.id,
            result: Err(LlmError::AuthRequired),
        },
    );
    assert_eq!(s.phase, Phase::LoginRequired);
}

#[test]
fn accept_safe_command_exits_with_text() {
    let mut s = ready();
    generate(&mut s, "list", "ls -la");
    assert_eq!(
        key(&mut s, KeyAction::Submit),
        vec![Effect::Exit(ExitOutcome::Accepted("ls -la".into()))]
    );
}

#[test]
fn high_risk_command_needs_second_enter() {
    let mut s = ready();
    generate(&mut s, "clean", "rm -rf build");
    assert!(key(&mut s, KeyAction::Submit).is_empty());
    assert_eq!(s.phase, Phase::ConfirmDanger);
    assert_eq!(
        key(&mut s, KeyAction::Submit),
        vec![Effect::Exit(ExitOutcome::Accepted("rm -rf build".into()))]
    );
}

#[test]
fn other_key_leaves_danger_confirmation() {
    let mut s = ready();
    generate(&mut s, "clean", "rm -rf build");
    key(&mut s, KeyAction::Submit);
    key(&mut s, KeyAction::Cancel);
    assert_eq!(s.phase, Phase::Ready, "위험 확인 중 Esc는 편집 복귀");
    key(&mut s, KeyAction::Submit);
    key(&mut s, KeyAction::Other);
    assert_eq!(s.phase, Phase::Ready);
}

#[test]
fn ctrl_c_and_esc_cancel_everywhere() {
    let mut s = ready();
    assert_eq!(
        key(&mut s, KeyAction::Cancel),
        vec![Effect::Exit(ExitOutcome::Cancelled)]
    );
    let mut s = ready();
    type_text(&mut s, "q");
    let req = request_of(&key(&mut s, KeyAction::Submit));
    assert_eq!(
        key(&mut s, KeyAction::Quit),
        vec![
            Effect::CancelRequest(req.id),
            Effect::Exit(ExitOutcome::Cancelled)
        ]
    );
    let mut s = State::new(Lang::En, env(), false);
    assert_eq!(
        key(&mut s, KeyAction::Quit),
        vec![Effect::Exit(ExitOutcome::Cancelled)]
    );
}

#[test]
fn too_small_only_handles_cancel() {
    let mut s = ready();
    update(
        &mut s,
        Action::Resize {
            width: 25,
            height: 3,
        },
    );
    type_text(&mut s, "x");
    assert!(s.question.is_empty());
    assert!(key(&mut s, KeyAction::Submit).is_empty());
    update(
        &mut s,
        Action::Resize {
            width: 80,
            height: 20,
        },
    );
    type_text(&mut s, "x");
    assert_eq!(s.question.text(), "x");
    update(
        &mut s,
        Action::Resize {
            width: 25,
            height: 3,
        },
    );
    assert_eq!(
        key(&mut s, KeyAction::Cancel),
        vec![Effect::Exit(ExitOutcome::Cancelled)]
    );
}

#[test]
fn history_is_limited() {
    let mut s = ready();
    generate(&mut s, "q0", "c0");
    for i in 1..8 {
        key(&mut s, KeyAction::ToggleFocus);
        type_text(&mut s, &format!("q{i}"));
        let req = request_of(&key(&mut s, KeyAction::Submit));
        update(
            &mut s,
            Action::LlmDone {
                id: req.id,
                result: Ok(response(Some(&format!("c{i}")), "e")),
            },
        );
    }
    key(&mut s, KeyAction::ToggleFocus);
    type_text(&mut s, "last");
    let req = request_of(&key(&mut s, KeyAction::Submit));
    assert_eq!(req.history.len(), HISTORY_LIMIT);
    assert_eq!(req.history.last().unwrap().question.as_deref(), Some("q7"));
}

#[test]
fn newline_only_in_command_area() {
    let mut s = ready();
    type_text(&mut s, "q");
    key(&mut s, KeyAction::InsertNewline);
    assert_eq!(s.question.text(), "q");
    s.focus = Focus::Command;
    type_text(&mut s, "a");
    key(&mut s, KeyAction::InsertNewline);
    type_text(&mut s, "b");
    assert_eq!(s.command.text(), "a\nb");
}

#[test]
fn paste_into_question_is_single_line_and_into_command_keeps_lines() {
    let mut s = ready();
    update(&mut s, Action::Paste("find\nfiles".into()));
    assert_eq!(s.question.text(), "find files");
    s.focus = Focus::Command;
    update(&mut s, Action::Paste("a\r\nb".into()));
    assert_eq!(s.command.text(), "a\nb");
    assert_eq!(s.phase, Phase::Ready, "붙여넣기로 확정 미발생");
}

#[test]
fn provider_label_and_non_command_answer() {
    let mut s = ready();
    update(
        &mut s,
        Action::ProviderReady(ProviderLabel {
            model: Some("m1".into()),
            chatgpt_plan: true,
        }),
    );
    assert_eq!(s.provider.model.as_deref(), Some("m1"));
    type_text(&mut s, "what is a pipe");
    let req = request_of(&key(&mut s, KeyAction::Submit));
    update(
        &mut s,
        Action::LlmDone {
            id: req.id,
            result: Ok(response(None, "a pipe connects")),
        },
    );
    assert!(s.command.is_empty());
    assert_eq!(s.focus, Focus::Question);
}

#[test]
fn findings_follow_edits() {
    let mut s = ready();
    generate(&mut s, "list", "ls");
    assert!(s.findings.is_empty());
    key(&mut s, KeyAction::Edit(EditOp::Home));
    key(&mut s, KeyAction::Edit(EditOp::KillToEnd));
    type_text(&mut s, "rm -r x");
    assert_eq!(crate::safety::max_level(&s.findings), Some(Level::High));
}
