//! Inline 렌더링 검사 (TC-UI-003-005, TC-AUTH-017, TC-SAFE-005, TC-I18N-004). ratatui TestBackend 사용.

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::render;
use crate::app::editor::EditOp;
use crate::app::{Action, KeyAction, State, update};
use crate::i18n::Lang;
use crate::llm::types::{EnvContext, LlmResponse, OsKind, ProviderLabel};
use crate::shell::ShellKind;

fn state(lang: Lang) -> State {
    let mut s = State::new(
        lang,
        EnvContext {
            os: OsKind::Linux,
            arch: "x86_64",
            shell: ShellKind::Bash,
            shell_version: None,
        },
        true,
    );
    s.provider = ProviderLabel {
        model: Some("model-a".into()),
        chatgpt_plan: true,
    };
    s
}

fn screen(s: &State, w: u16, h: u16) -> Vec<String> {
    screen_hits(s, w, h).0
}

fn screen_hits(s: &State, w: u16, h: u16) -> (Vec<String>, super::Hits) {
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    let mut hits = Vec::new();
    term.draw(|f| hits = render(f, s)).unwrap();
    let buf = term.backend().buffer().clone();
    let lines = (0..h)
        .map(|y| {
            let mut line = String::new();
            let mut x = 0;
            while x < w {
                let cell = &buf[(x, y)];
                line.push_str(cell.symbol());
                x += cell
                    .symbol()
                    .chars()
                    .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0))
                    .sum::<usize>()
                    .max(1) as u16;
            }
            line.trim_end().to_string()
        })
        .collect();
    (lines, hits)
}

fn with_response(lang: Lang, cmd: &str, explanation: &str, notes: Vec<String>) -> State {
    let mut s = state(lang);
    for c in "find big files".chars() {
        update(&mut s, Action::Key(KeyAction::Edit(EditOp::Insert(c))));
    }
    let effects = update(&mut s, Action::Key(KeyAction::Submit));
    let id = match &effects[0] {
        crate::app::Effect::StartRequest(r) => r.id,
        _ => unreachable!(),
    };
    update(
        &mut s,
        Action::LlmDone {
            id,
            result: Ok(LlmResponse {
                command: Some(cmd.into()),
                explanation: explanation.into(),
                notes,
                model: "model-a".into(),
            }),
        },
    );
    s
}

#[test]
fn main_layout_shows_all_areas() {
    let s = with_response(
        Lang::En,
        "find . -type f -size +100M",
        "Finds files larger than 100 MB.",
        vec!["Large directories take time".into()],
    );
    let lines = screen(&s, 80, 10);
    assert!(lines[0].starts_with("Ask     > "), "{lines:?}");
    assert!(
        lines[1].starts_with("Command > find . -type f -size +100M"),
        "{lines:?}"
    );
    assert!(
        lines[2].starts_with("About   > Finds files larger than 100 MB."),
        "{lines:?}"
    );
    assert!(
        lines[3].contains("Notes: Large directories take time"),
        "모델 참고 표시 {lines:?}"
    );
    let status = &lines[9];
    assert!(
        status.contains("Enter accept")
            && status.contains("model-a")
            && status.contains("Using ChatGPT plan"),
        "{status}"
    );
}

#[test]
fn layout_priority_on_short_screen() {
    let s = with_response(
        Lang::En,
        "a\nb\nc\nd\ne\nf\ng\nh",
        "long explanation",
        vec![],
    );
    let lines = screen(&s, 60, 6);
    assert!(lines[0].starts_with("Ask"));
    assert!(lines[1].starts_with("Command >"));
    // 명령어는 커서(마지막 줄) 위치까지 스크롤, 설명 영역 0줄
    assert!(lines[4].ends_with('h'), "{lines:?}");
    assert!(!lines.iter().any(|l| l.contains("long explanation")));
    assert!(lines[5].contains("bash"));
}

#[test]
fn requesting_shows_progress_and_cancel_hint() {
    let mut s = state(Lang::En);
    s.question.set_text("q");
    update(&mut s, Action::Key(KeyAction::Submit));
    let lines = screen(&s, 80, 6);
    assert!(
        lines[5].contains("Generating") && lines[5].contains("Esc cancel"),
        "{lines:?}"
    );
}

#[test]
fn warning_line_for_risky_command() {
    let s = with_response(Lang::En, "rm -rf ./build", "Deletes build output.", vec![]);
    let lines = screen(&s, 80, 8);
    assert!(
        lines[2].starts_with("Warning > High risk: recursive delete (rm)"),
        "{lines:?}"
    );
}

#[test]
fn korean_labels_and_too_small_message() {
    let s = with_response(Lang::Ko, "ls -la", "파일 목록 표시", vec![]);
    let lines = screen(&s, 80, 6);
    assert!(lines[0].starts_with("질문"), "{lines:?}");
    assert!(
        lines[1].contains("명령") && lines[1].contains("ls -la"),
        "{lines:?}"
    );
    assert!(lines[2].contains("파일 목록 표시"));
    let mut tiny = state(Lang::Ko);
    update(
        &mut tiny,
        Action::Resize {
            width: 20,
            height: 2,
        },
    );
    let lines = screen(&tiny, 20, 2);
    assert!(lines[0].starts_with("터미널 크기 부족"), "{lines:?}");
}

#[test]
fn login_screen_shows_url_and_paste_field() {
    let mut s = State::new(
        Lang::En,
        EnvContext {
            os: OsKind::Linux,
            arch: "x86_64",
            shell: ShellKind::Bash,
            shell_version: None,
        },
        false,
    );
    assert!(screen(&s, 80, 6)[0].contains("Sign-in required"));
    update(&mut s, Action::Key(KeyAction::Submit));
    update(
        &mut s,
        Action::LoginStarted(crate::app::LoginView {
            url: Some("https://auth.openai.com/api/accounts/authorize?client_id=x".into()),
            port: Some(1455),
            browser_failed: true,
        }),
    );
    let lines = screen(&s, 80, 10);
    assert!(
        lines
            .iter()
            .any(|l| l.contains("https://auth.openai.com/api/accounts/authorize?client_id=x")),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("OpenAI")),
        "로그인 전 전송 범위 고지: {lines:?}"
    );
    assert!(
        lines[8].starts_with("> Paste the address here"),
        "{lines:?}"
    );
    assert!(lines[9].contains("Esc cancel"));
}

/// CR-005: 설정 화면 표시, 현재 모델 표시, 클릭 영역, 상태줄 모델 버튼.
#[test]
fn settings_view_lists_models_and_click_targets() {
    use crate::app::Hit;
    use crate::llm::types::{ModelChoice, ProviderSettings};
    let mut s = state(Lang::En);
    let (lines, hits) = screen_hits(&s, 80, 10);
    assert!(lines[9].contains("[folder: off]  [model-a]"), "{lines:?}");
    let folder = hits
        .iter()
        .find(|(_, h)| *h == Hit::ToggleFolder)
        .expect("folder button");
    assert!(
        lines[9]
            .chars()
            .skip(folder.0.x as usize)
            .collect::<String>()
            .starts_with("[folder: off]")
    );
    let open = hits
        .iter()
        .find(|(_, h)| *h == Hit::OpenSettings)
        .expect("model button");
    assert_eq!(open.0.y, 9);
    assert!(
        lines[9]
            .chars()
            .skip(open.0.x as usize)
            .collect::<String>()
            .starts_with("[model-a]"),
        "x={} line={:?}",
        open.0.x,
        lines[9]
    );
    assert!(hits.iter().any(|(r, h)| *h == Hit::Question && r.y == 0));

    update(&mut s, Action::Key(KeyAction::Settings));
    let lines = screen(&s, 80, 10);
    assert!(lines[0].starts_with("Settings  Agent:"), "{lines:?}");
    assert!(lines[9].contains("Loading models"), "{lines:?}");
    let data = ProviderSettings {
        agent: "ChatGPT".into(),
        account: Some("h***@example.com".into()),
        models: vec![
            ModelChoice {
                id: "gpt-6.1-sol".into(),
                name: "GPT-6.1-Sol".into(),
            },
            ModelChoice {
                id: "model-a".into(),
                name: "Model A".into(),
            },
        ],
        current: Some("model-a".into()),
        can_sign_out: true,
    };
    update(&mut s, Action::SettingsLoaded(Ok(data)));
    let (lines, hits) = screen_hits(&s, 80, 10);
    assert!(lines[0].contains("ChatGPT (h***@example.com)"), "{lines:?}");
    assert!(lines[0].ends_with("[Close]"));
    assert!(
        lines[1].starts_with("Model  ( ) GPT-6.1-Sol  gpt-6.1-sol"),
        "{lines:?}"
    );
    assert!(lines[2].contains("(*) Model A  model-a"), "{lines:?}");
    assert!(
        lines[3].contains("Send current folder listing (Ctrl+F): off"),
        "{lines:?}"
    );
    assert!(lines[4].contains("Sign out"), "{lines:?}");
    assert!(lines[9].contains("Enter choose"));
    let items: Vec<_> = hits
        .iter()
        .filter_map(|(r, h)| match h {
            Hit::SettingsItem(i) => Some((r.y, *i)),
            _ => None,
        })
        .collect();
    assert_eq!(items, vec![(1, 0), (2, 1), (3, 2), (4, 3)]);
    assert!(hits.iter().any(|(_, h)| *h == Hit::CloseSettings));
}
