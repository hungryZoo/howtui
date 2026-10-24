//! Fullscreen 화면 렌더러 (PRD 5.9절, SRS 7.3절). 상태를 읽기만 한다.
//!
//! 상단: 세션 대화 이력 스크롤 영역. 하단: Inline과 같은 질문, 명령어, 경고, 설명, 상태줄.
//! 로그인, 설정, 업데이트, 크기 부족 화면은 Inline 렌더러를 전체 화면에 그대로 사용 (PRD-FS-002).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::inline::{self, Hits, dim, label, label_width, row};
use super::text::{truncate, wrap_buffer, wrap_words};
use crate::app::editor::display_width;
use crate::app::{MIN_HEIGHT, MIN_WIDTH, Phase, State};
use crate::i18n::{Msg, tr_in, trf_in};

/// 하단 편집 영역 행 수 (질문, 명령어, 경고, 설명, 상태줄)
pub const BOTTOM_ROWS: u16 = 8;
/// 대화 기록 영역 최소 행 수 (제목, 내용 1행 이상, 구분선). 미달 시 하단 영역만 표시
const HISTORY_MIN_ROWS: u16 = 4;

/// 대화 기록 스크롤 범위: 최대 스크롤 위치와 한 쪽 행 수. 런타임이 `Action::HistoryLayout`으로 전달.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HistoryLayout {
    pub max: u16,
    pub page: u16,
}

pub fn render(frame: &mut Frame, state: &State) -> (Hits, HistoryLayout) {
    let area = frame.area();
    let main_view = !(state.too_small
        || area.width < MIN_WIDTH
        || area.height < MIN_HEIGHT
        || matches!(state.phase, Phase::LoginRequired | Phase::LoggingIn)
        || state.update_view.is_some()
        || state.settings.is_some());
    if !main_view || area.height < BOTTOM_ROWS + HISTORY_MIN_ROWS {
        let hits = inline::render(frame, state);
        return (hits, HistoryLayout::default());
    }
    let history_h = area.height - BOTTOM_ROWS;
    let history = Rect {
        height: history_h,
        ..area
    };
    let bottom = Rect {
        y: area.y + history_h,
        height: BOTTOM_ROWS,
        ..area
    };
    let layout = render_history(frame, state, history);
    let mut hits = Hits::new();
    inline::render_main(frame, state, bottom, &mut hits);
    (hits, layout)
}

/// 대화 기록 전체 행 (화면 폭 기준 줄바꿈 완료)
fn history_lines(state: &State, width: u16) -> Vec<Line<'static>> {
    let lw = label_width(state);
    let content_w = width.saturating_sub(lw).max(1);
    let mut lines = Vec::new();
    let push =
        |lines: &mut Vec<Line<'static>>, msg: Option<(Msg, Style)>, text: &str, style: Style| {
            for (i, part) in wrap_words(text, content_w).into_iter().enumerate() {
                let head = match (i, msg) {
                    (0, Some((m, s))) => label(state, m, lw, s),
                    _ => Span::raw(" ".repeat(lw as usize)),
                };
                lines.push(Line::from(vec![head, Span::styled(part, style)]));
            }
        };
    let q_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let c_style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);
    for (n, turn) in state.conversation.iter().enumerate() {
        if n > 0 {
            lines.push(Line::default());
        }
        match &turn.question {
            Some(q) => push(
                &mut lines,
                Some((Msg::LabelQuestion, q_style)),
                q,
                Style::default(),
            ),
            None => push(
                &mut lines,
                Some((Msg::LabelQuestion, q_style)),
                tr_in(state.lang, Msg::HistoryExplainRequest),
                dim(),
            ),
        }
        let command = turn
            .response
            .command
            .as_deref()
            .or(turn.command_sent.as_deref());
        if let Some(cmd) = command {
            // 명령어는 공백 보존 문자 단위 줄바꿈 (편집 영역과 동일)
            let src: Vec<String> = cmd.split('\n').map(str::to_string).collect();
            for (i, part) in wrap_buffer(&src, None, content_w)
                .rows
                .into_iter()
                .enumerate()
            {
                let head = if i == 0 {
                    label(state, Msg::LabelCommand, lw, c_style)
                } else {
                    Span::raw(" ".repeat(lw as usize))
                };
                lines.push(Line::from(vec![head, Span::raw(part)]));
            }
        }
        if !turn.response.explanation.is_empty() {
            push(
                &mut lines,
                Some((Msg::LabelExplain, dim())),
                &turn.response.explanation,
                Style::default(),
            );
        }
        for note in &turn.response.notes {
            push(
                &mut lines,
                None,
                &format!("{}: {note}", tr_in(state.lang, Msg::LabelNotes)),
                dim(),
            );
        }
    }
    lines
}

fn render_history(frame: &mut Frame, state: &State, area: Rect) -> HistoryLayout {
    // 제목 1행 + 내용 + 구분선 1행
    let content_h = area.height.saturating_sub(2).max(1);
    let lines = history_lines(state, area.width);
    let total = lines.len() as u16;
    let max = total.saturating_sub(content_h);
    let scroll = state.history_scroll.min(max);
    let layout = HistoryLayout {
        max,
        page: content_h.saturating_sub(1).max(1),
    };

    // 제목: "── 대화 기록 ───── 위로 N행"
    let title = format!("── {} ", tr_in(state.lang, Msg::HistoryTitle));
    let right = if scroll > 0 {
        format!(
            " {} ──",
            trf_in(
                state.lang,
                Msg::HistoryScrolled,
                &[("n", &scroll.to_string())]
            )
        )
    } else {
        String::new()
    };
    let fill = (area.width as usize).saturating_sub(display_width(&title) + display_width(&right));
    let title_line = truncate(&format!("{title}{}{right}", "─".repeat(fill)), area.width);
    frame.render_widget(
        Paragraph::new(title_line).style(dim().add_modifier(Modifier::BOLD)),
        row(area, 0),
    );

    let body = Rect {
        y: area.y + 1,
        height: content_h,
        ..area
    };
    if lines.is_empty() {
        frame.render_widget(
            Paragraph::new(truncate(tr_in(state.lang, Msg::HistoryEmpty), area.width))
                .style(dim().add_modifier(Modifier::ITALIC)),
            row(body, 0),
        );
    } else {
        // 최신 기록이 아래에 오도록 끝 기준으로 표시
        let start = total.saturating_sub(content_h).saturating_sub(scroll) as usize;
        let shown: Vec<Line<'static>> = lines
            .into_iter()
            .skip(start)
            .take(content_h as usize)
            .collect();
        frame.render_widget(Paragraph::new(shown), body);
    }
    frame.render_widget(
        Paragraph::new("─".repeat(area.width as usize)).style(dim()),
        row(area, area.height - 1),
    );
    layout
}

#[cfg(test)]
mod tests;
