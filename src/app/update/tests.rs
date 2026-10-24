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
    // 콜백 주소 붙여넣기는 Enter 없이 제출 (CR-002)
    assert_eq!(
        update(
            &mut s,
            Action::Paste("  http://127.0.0.1:1455/auth/callback?code=x&state=y \n".into()),
        ),
        vec![Effect::SubmitRedirect(
            "http://127.0.0.1:1455/auth/callback?code=x&state=y".into()
        )]
    );
    // 그 외 붙여넣기는 입력란에 넣고 Enter로 제출
    assert!(update(&mut s, Action::Paste("abc".into())).is_empty());
    assert_eq!(
        key(&mut s, KeyAction::Submit),
        vec![Effect::SubmitRedirect("abc".into())]
    );
    s.login_paste.clear();
    update(
        &mut s,
        Action::LoginStarted(LoginView {
            url: Some("https://auth/x".into()),
            port: Some(1455),
            browser_failed: true,
        }),
    );
    update(&mut s, Action::LoginDone(Err(LoginFailure::InvalidPaste)));
    assert_eq!(s.phase, Phase::LoggingIn);
    assert_eq!(
        s.login.url.as_deref(),
        Some("https://auth/x"),
        "잘못된 붙여넣기 후 URL 표시 유지"
    );
    assert!(s.login_paste.is_empty());
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
fn success_applies_command_and_keeps_question_focus() {
    let mut s = ready();
    generate(&mut s, "list", "ls -la");
    assert_eq!(s.phase, Phase::Ready);
    assert_eq!(
        s.focus,
        Focus::Question,
        "응답 후 다음 요청 입력 대기 (CR-001)"
    );
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
    // 사용자 직접 편집 (↓로 명령어 영역 이동)
    key(&mut s, KeyAction::Edit(EditOp::Down));
    assert_eq!(s.focus, Focus::Command);
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
    key(&mut s, KeyAction::Edit(EditOp::Down));
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
    key(&mut s, KeyAction::ToggleFocus);
    key(&mut s, KeyAction::Edit(EditOp::Home));
    key(&mut s, KeyAction::Edit(EditOp::KillToEnd));
    type_text(&mut s, "rm -r x");
    assert_eq!(crate::safety::max_level(&s.findings), Some(Level::High));
}

#[test]
fn empty_question_enter_accepts_command() {
    let mut s = ready();
    generate(&mut s, "list", "ls -la");
    assert_eq!(
        key(&mut s, KeyAction::Submit),
        vec![Effect::Exit(ExitOutcome::Accepted("ls -la".into()))]
    );
}

#[test]
fn empty_question_enter_on_high_risk_needs_second_enter() {
    let mut s = ready();
    generate(&mut s, "remove", "rm -rf build");
    assert!(key(&mut s, KeyAction::Submit).is_empty());
    assert_eq!(s.phase, Phase::ConfirmDanger);
    assert_eq!(
        key(&mut s, KeyAction::Submit),
        vec![Effect::Exit(ExitOutcome::Accepted("rm -rf build".into()))]
    );
}

#[test]
fn arrows_move_between_question_and_command() {
    let mut s = ready();
    // 명령어가 없으면 ↓ 이동 없음
    key(&mut s, KeyAction::Edit(EditOp::Down));
    assert_eq!(s.focus, Focus::Question);
    generate(&mut s, "two lines", "a\nb");
    key(&mut s, KeyAction::Edit(EditOp::Down));
    assert_eq!(s.focus, Focus::Command);
    // 명령어 둘째 줄의 ↑는 줄 이동, 첫 줄의 ↑는 질문 영역 이동
    key(&mut s, KeyAction::Edit(EditOp::Down));
    assert_eq!((s.focus, s.command.cursor().0), (Focus::Command, 1));
    key(&mut s, KeyAction::Edit(EditOp::Up));
    assert_eq!((s.focus, s.command.cursor().0), (Focus::Command, 0));
    key(&mut s, KeyAction::Edit(EditOp::Up));
    assert_eq!(s.focus, Focus::Question);
}

fn settings_data() -> crate::llm::types::ProviderSettings {
    use crate::llm::types::{ModelChoice, ProviderSettings};
    ProviderSettings {
        agent: "ChatGPT".into(),
        account: None,
        models: vec![
            ModelChoice {
                id: "a".into(),
                name: "A".into(),
            },
            ModelChoice {
                id: "b".into(),
                name: "B".into(),
            },
        ],
        current: Some("b".into()),
        can_sign_out: true,
    }
}

/// CR-005: Ctrl+O로 열기, 목록 수신 시 현재 모델에 커서, 다른 모델 선택, 반영 후 닫힘.
#[test]
fn settings_select_model_flow() {
    let mut s = ready();
    assert_eq!(key(&mut s, KeyAction::Settings), vec![Effect::LoadSettings]);
    assert!(s.is_busy(), "목록 수신 전 진행 표시");
    update(&mut s, Action::SettingsLoaded(Ok(settings_data())));
    assert_eq!(s.settings.as_ref().unwrap().cursor, 1);
    key(&mut s, KeyAction::Edit(EditOp::Up));
    assert_eq!(
        key(&mut s, KeyAction::Submit),
        vec![Effect::SelectModel("a".into())]
    );
    assert!(
        key(&mut s, KeyAction::Submit).is_empty(),
        "적용 중 중복 선택 없음"
    );
    update(
        &mut s,
        Action::ModelSelected(Ok(ProviderLabel {
            model: Some("a".into()),
            chatgpt_plan: true,
        })),
    );
    assert!(s.settings.is_none());
    assert_eq!(s.provider.model.as_deref(), Some("a"));
    assert_eq!(
        s.status.as_ref().map(|st| st.msg),
        Some(Msg::StatusModelChanged)
    );
}

/// 현재 모델 다시 선택은 닫기만, Esc와 Ctrl+O는 닫기, `/model` 입력으로 열기.
#[test]
fn settings_close_and_slash_command() {
    let mut s = ready();
    type_text(&mut s, "/model");
    assert_eq!(key(&mut s, KeyAction::Submit), vec![Effect::LoadSettings]);
    assert!(s.question.is_empty());
    update(&mut s, Action::SettingsLoaded(Ok(settings_data())));
    assert!(key(&mut s, KeyAction::Submit).is_empty());
    assert!(s.settings.is_none(), "현재 모델 선택 시 닫힘");
    key(&mut s, KeyAction::Settings);
    key(&mut s, KeyAction::Cancel);
    assert!(s.settings.is_none());
    assert_eq!(s.phase, Phase::Ready, "Esc는 종료가 아닌 닫기");
    key(&mut s, KeyAction::Settings);
    assert!(key(&mut s, KeyAction::Settings).is_empty());
    assert!(s.settings.is_none());
}

/// 클릭: 상태줄 모델 버튼으로 열기, 항목 클릭으로 선택, 로그아웃 후 로그인 필요 화면.
#[test]
fn settings_clicks_and_sign_out() {
    let mut s = ready();
    generate(&mut s, "list", "ls");
    update(&mut s, Action::Click(Hit::Command));
    assert_eq!(s.focus, Focus::Command);
    update(&mut s, Action::Click(Hit::Question));
    assert_eq!(s.focus, Focus::Question);
    assert_eq!(
        update(&mut s, Action::Click(Hit::OpenSettings)),
        vec![Effect::LoadSettings]
    );
    update(&mut s, Action::SettingsLoaded(Ok(settings_data())));
    update(&mut s, Action::Scroll(-1));
    assert_eq!(s.settings.as_ref().unwrap().cursor, 0);
    assert_eq!(
        update(&mut s, Action::Click(Hit::SettingsItem(4))),
        vec![Effect::SignOut]
    );
    update(&mut s, Action::SignedOut);
    assert!(s.settings.is_none());
    assert_eq!(s.phase, Phase::LoginRequired);
    assert_eq!(s.command.text(), "ls", "명령어 유지");
    // 요청 중에는 설정 화면 미개방
    let mut s = ready();
    type_text(&mut s, "q");
    key(&mut s, KeyAction::Submit);
    assert!(update(&mut s, Action::Click(Hit::OpenSettings)).is_empty());
}

/// 목록 조회 실패는 설정 화면 안에 표시, 인증 오류는 로그인 필요 화면.
#[test]
fn settings_load_errors() {
    let mut s = ready();
    key(&mut s, KeyAction::Settings);
    update(&mut s, Action::SettingsLoaded(Err(LlmError::ModelNotFound)));
    assert!(s.settings.as_ref().unwrap().error.is_some());
    assert!(!s.is_busy(), "오류 표시 중 진행 표시 없음");
    key(&mut s, KeyAction::Cancel);
    key(&mut s, KeyAction::Settings);
    update(&mut s, Action::SettingsLoaded(Err(LlmError::AuthRequired)));
    assert!(s.settings.is_none());
    assert_eq!(s.phase, Phase::LoginRequired);
}

/// CR-007: 폴더 목록 전송 켜기와 끄기 (Ctrl+F, 상태줄 클릭, 설정 화면 항목), 요청 플래그, 저장 Effect.
#[test]
fn folder_toggle_sets_request_flag_and_saves() {
    let mut s = ready();
    type_text(&mut s, "a");
    assert!(
        !request_of(&key(&mut s, KeyAction::Submit)).include_folder,
        "기본 끔"
    );
    let mut s = ready();
    assert_eq!(
        key(&mut s, KeyAction::ToggleFolder),
        vec![Effect::SaveFolder(true)]
    );
    assert!(s.send_folder);
    assert_eq!(
        s.status.as_ref().map(|st| st.msg),
        Some(Msg::StatusFolderOn)
    );
    type_text(&mut s, "압축파일 풀어줘");
    let req = request_of(&key(&mut s, KeyAction::Submit));
    assert!(req.include_folder);
    assert_eq!(req.folder, None, "목록은 런타임이 채움");
    // 요청 중 클릭으로 끄기
    assert_eq!(
        update(&mut s, Action::Click(Hit::ToggleFolder)),
        vec![Effect::SaveFolder(false)]
    );
    // 설정 화면 항목 (모델 2개 다음)
    let mut s = ready();
    key(&mut s, KeyAction::Settings);
    update(&mut s, Action::SettingsLoaded(Ok(settings_data())));
    assert_eq!(
        update(&mut s, Action::Click(Hit::SettingsItem(2))),
        vec![Effect::SaveFolder(true)]
    );
    assert!(s.settings.is_some(), "토글은 설정 화면 유지");
}

fn update_info() -> UpdateInfo {
    UpdateInfo {
        current: "0.1.1".into(),
        latest: "0.2.0".into(),
        method: UpdateMethod::Script {
            dir: "/home/u/.local/bin".into(),
        },
    }
}

/// CR-009: 알림 클릭으로 업데이트 화면, Enter로 종료 후 적용, Esc 닫기.
#[test]
fn update_notice_click_and_confirm() {
    let mut s = ready();
    s.update = Some(update_info());
    assert!(update(&mut s, Action::Click(Hit::OpenUpdate)).is_empty());
    assert_eq!(s.update_view, Some(UpdateView::Ready(update_info())));
    key(&mut s, KeyAction::Cancel);
    assert!(s.update_view.is_none(), "Esc는 닫기");
    assert_eq!(s.phase, Phase::Ready);
    update(&mut s, Action::Click(Hit::OpenUpdate));
    assert_eq!(
        key(&mut s, KeyAction::Submit),
        vec![Effect::Exit(ExitOutcome::Update(update_info()))]
    );
}

/// 알림이 없을 때 `/update`와 설정 화면 항목은 즉시 확인, 결과 반영과 알림 갱신.
#[test]
fn update_check_from_slash_command_and_settings() {
    let mut s = ready();
    type_text(&mut s, "/update");
    assert_eq!(key(&mut s, KeyAction::Submit), vec![Effect::CheckUpdate]);
    assert_eq!(s.update_view, Some(UpdateView::Checking));
    assert!(s.is_busy());
    assert!(
        key(&mut s, KeyAction::Submit).is_empty(),
        "확인 중 Enter 무시"
    );
    update(
        &mut s,
        Action::UpdateChecked(UpdateView::UpToDate {
            current: "0.1.1".into(),
            latest: "0.1.1".into(),
        }),
    );
    assert!(matches!(s.update_view, Some(UpdateView::UpToDate { .. })));
    assert!(key(&mut s, KeyAction::Submit).is_empty());
    assert!(s.update_view.is_none(), "최신이면 Enter로 닫기");
    // 설정 화면 항목 (모델 2개, 폴더 다음)
    key(&mut s, KeyAction::Settings);
    update(&mut s, Action::SettingsLoaded(Ok(settings_data())));
    assert_eq!(
        update(&mut s, Action::Click(Hit::SettingsItem(3))),
        vec![Effect::CheckUpdate]
    );
    assert!(s.settings.is_none());
    update(
        &mut s,
        Action::UpdateChecked(UpdateView::Ready(update_info())),
    );
    assert_eq!(s.update, Some(update_info()), "상태줄 알림 갱신");
}
