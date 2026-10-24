//! Fullscreen 렌더링 검사 (TC-FS-002, TC-FS-003). ratatui TestBackend 사용.

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::{BOTTOM_ROWS, HistoryLayout, render};
use crate::app::editor::EditOp;
use crate::app::{Action, Effect, KeyAction, State, update};
use crate::i18n::Lang;
use crate::llm::types::{EnvContext, LlmResponse, OsKind, ProviderLabel};
use crate::shell::ShellKind;

fn state() -> State {
    let mut s = State::new(
        Lang::En,
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

fn draw(s: &State, w: u16, h: u16) -> (Vec<String>, HistoryLayout) {
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    let mut layout = HistoryLayout::default();
    term.draw(|f| layout = render(f, s).1).unwrap();
    let buf = term.backend().buffer().clone();
    let lines = (0..h)
        .map(|y| {
            (0..w)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect();
    (lines, layout)
}

/// 질문 제출과 응답 반영 1회
fn ask(s: &mut State, question: &str, cmd: &str, explanation: &str) {
    for c in question.chars() {
        update(s, Action::Key(KeyAction::Edit(EditOp::Insert(c))));
    }
    let effects = update(s, Action::Key(KeyAction::Submit));
    let id = match &effects[0] {
        Effect::StartRequest(r) => r.id,
        other => panic!("{other:?}"),
    };
    update(
        s,
        Action::LlmDone {
            id,
            result: Ok(LlmResponse {
                command: Some(cmd.into()),
                explanation: explanation.into(),
                notes: Vec::new(),
                model: "model-a".into(),
            }),
        },
    );
}

/// 렌더러가 계산한 스크롤 범위를 런타임처럼 반영
fn apply(s: &mut State, l: HistoryLayout) {
    update(
        s,
        Action::HistoryLayout {
            max: l.max,
            page: l.page,
        },
    );
}

#[test]
fn layout_history_on_top_main_at_bottom() {
    let mut s = state();
    let (lines, layout) = draw(&s, 80, 24);
    assert!(lines[0].starts_with("── History "), "{lines:?}");
    assert!(lines[1].contains("Your questions and answers appear here"));
    assert_eq!(layout.max, 0);
    let bottom = (24 - BOTTOM_ROWS) as usize;
    assert!(lines[bottom].starts_with("Ask     > "), "{lines:?}");
    assert!(lines[bottom + 1].starts_with("Command >"), "{lines:?}");
    assert!(lines[23].contains("model-a"), "상태줄 {lines:?}");

    ask(
        &mut s,
        "find big files",
        "find . -size +100M",
        "Finds large files.",
    );
    let (lines, _) = draw(&s, 80, 24);
    assert!(
        lines[1].starts_with("Ask     > find big files"),
        "{lines:?}"
    );
    assert!(lines[2].starts_with("Command > find . -size +100M"));
    assert!(lines[3].starts_with("About   > Finds large files."));
    assert!(lines[bottom + 1].starts_with("Command > find . -size +100M"));

    // 여러 줄 명령어의 앞 공백 보존
    ask(&mut s, "pipe", "echo a \\\n  | cat", "Two lines.");
    let (lines, _) = draw(&s, 80, 24);
    assert!(lines.iter().any(|l| l == "            | cat"), "{lines:?}");
}

#[test]
fn long_history_scrolls_with_page_keys_and_wheel() {
    let mut s = state();
    let long = (1..=30)
        .map(|i| format!("line{i:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    for i in 0..10 {
        ask(&mut s, &format!("q{i}"), &format!("echo {i}"), &long);
    }
    assert_eq!(s.conversation.len(), 10);
    let (lines, layout) = draw(&s, 80, 24);
    apply(&mut s, layout);
    // 기록 10건 x (질문 1 + 명령어 1 + 설명 30) + 구분 빈 줄 9 = 329행, 내용 영역 14행
    assert_eq!(layout.max, 329 - 14);
    assert_eq!(layout.page, 13);
    // 최신 기록의 마지막 설명 줄이 내용 영역 마지막 행
    assert!(lines[14].ends_with("line30"), "{lines:?}");
    assert!(!lines[0].contains("rows up"));

    update(&mut s, Action::Key(KeyAction::HistoryUp));
    assert_eq!(s.history_scroll, 13);
    let (lines, _) = draw(&s, 80, 24);
    assert!(lines[0].contains("13 rows up"), "{lines:?}");
    assert!(lines[14].ends_with("line17"), "{lines:?}");

    update(&mut s, Action::Scroll(-1));
    assert_eq!(s.history_scroll, 16, "휠 위로 3행");
    update(&mut s, Action::Scroll(1));
    update(&mut s, Action::Key(KeyAction::HistoryDown));
    assert_eq!(s.history_scroll, 0);
    update(&mut s, Action::Key(KeyAction::HistoryDown));
    assert_eq!(s.history_scroll, 0, "최신 위치 아래로 이동 없음");

    // 맨 위까지 이동: 첫 기록의 질문 표시, 최대 위치 초과 없음
    for _ in 0..40 {
        update(&mut s, Action::Key(KeyAction::HistoryUp));
    }
    assert_eq!(s.history_scroll, layout.max);
    let (lines, _) = draw(&s, 80, 24);
    assert!(lines[1].starts_with("Ask     > q0"), "{lines:?}");

    // 새 응답 반영 시 최신 위치로 복귀
    ask(&mut s, "again", "echo again", "done");
    assert_eq!(s.history_scroll, 0);
    let (lines, _) = draw(&s, 80, 24);
    assert!(lines[14].starts_with("About   > done"), "{lines:?}");
}

#[test]
fn same_core_state_as_inline() {
    // Fullscreen과 Inline은 같은 State와 update 사용: 같은 입력 순서는 같은 상태 결과 (TC-FS-002)
    let mut a = state();
    let mut b = state();
    ask(&mut a, "list files", "ls -al", "Lists files.");
    ask(&mut b, "list files", "ls -al", "Lists files.");
    let (_, layout) = draw(&b, 80, 24);
    apply(&mut b, layout);
    assert_eq!(a.command.text(), b.command.text());
    assert_eq!(a.conversation, b.conversation);
    assert_eq!(a.phase, b.phase);
    assert_eq!(a.focus, b.focus);
}

#[test]
fn small_screen_and_other_views_use_inline_renderer() {
    let mut s = state();
    ask(&mut s, "list files", "ls -al", "Lists files.");
    // 기록 영역 최소 행 미달: 하단 영역만 표시
    let (lines, layout) = draw(&s, 80, BOTTOM_ROWS + 3);
    assert!(lines[0].starts_with("Ask     > "), "{lines:?}");
    assert_eq!(layout, HistoryLayout::default());
    // 설정 화면은 전체 영역 사용
    update(&mut s, Action::Key(KeyAction::Settings));
    let (lines, _) = draw(&s, 80, 24);
    assert!(!lines[0].starts_with("── History"), "{lines:?}");
}
