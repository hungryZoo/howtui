//! Inline 화면 렌더러 (PRD 2절, SRS 7.2절). 상태를 읽기만 한다.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::text::{truncate, wrap_buffer, wrap_words};
use crate::app::editor::display_width;
use crate::app::{
    Focus, Hit, MIN_HEIGHT, MIN_WIDTH, Phase, SettingsEntry, SettingsView, State, StatusKind,
};
use crate::i18n::{Msg, tr_in, trf_in};
use crate::llm::types::Operation;
use crate::safety::Level;

const SPINNER: [&str; 4] = ["|", "/", "-", "\\"];

fn dim() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// 클릭 대상 영역 (화면 좌표). 런타임이 마우스 좌표로 찾는다 (CR-005).
pub type Hits = Vec<(Rect, Hit)>;

pub fn render(frame: &mut Frame, state: &State) -> Hits {
    let mut hits = Hits::new();
    let area = frame.area();
    if state.too_small || area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        let msg = trf_in(
            state.lang,
            Msg::StatusTooSmall,
            &[
                ("w", &MIN_WIDTH.to_string()),
                ("h", &MIN_HEIGHT.to_string()),
            ],
        );
        frame.render_widget(
            Paragraph::new(truncate(&msg, area.width)).style(Style::default().fg(Color::Yellow)),
            row(area, 0),
        );
        return hits;
    }
    match (state.phase, &state.settings) {
        (Phase::LoginRequired | Phase::LoggingIn, _) => render_login(frame, state, area),
        (_, Some(view)) => render_settings(frame, state, view, area, &mut hits),
        _ => render_main(frame, state, area, &mut hits),
    }
    hits
}

/// 설정 화면 (CR-005): 제목과 에이전트 1행, 모델 목록과 로그아웃, 안내 1행.
fn render_settings(
    frame: &mut Frame,
    state: &State,
    view: &SettingsView,
    area: Rect,
    hits: &mut Hits,
) {
    let lang = state.lang;
    let width = area.width;
    let close = tr_in(lang, Msg::SettingsClose);
    let close_w = display_width(close) as u16;
    let mut head = format!(
        "{}  {}: ",
        tr_in(lang, Msg::SettingsTitle),
        tr_in(lang, Msg::SettingsAgent)
    );
    if let Some(d) = &view.data {
        head.push_str(&d.agent);
        if let Some(acc) = &d.account {
            head.push_str(&format!(" ({acc})"));
        }
    }
    frame.render_widget(
        Paragraph::new(Span::styled(
            truncate(&head, width.saturating_sub(close_w + 1)),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        row(area, 0),
    );
    if width > close_w + 1 {
        let r = Rect {
            x: area.x + width - close_w,
            y: area.y,
            width: close_w,
            height: 1,
        };
        frame.render_widget(Paragraph::new(Span::styled(close, dim())), r);
        hits.push((r, Hit::CloseSettings));
    }

    let model_label = tr_in(lang, Msg::SettingsModel);
    let lw = display_width(model_label) + 2;
    let list_rows = (area.height.saturating_sub(2) as usize).max(1);
    let entries = view.entries();
    let start = (view.cursor + 1).saturating_sub(list_rows);
    if let Some(d) = &view.data {
        for (n, (i, entry)) in entries
            .iter()
            .enumerate()
            .skip(start)
            .take(list_rows)
            .enumerate()
        {
            let prefix = if i == 0 {
                format!("{model_label}  ")
            } else {
                " ".repeat(lw)
            };
            let (text, base) = match entry {
                SettingsEntry::Model(m) => {
                    let model = &d.models[*m];
                    let mark = if d.current.as_deref() == Some(model.id.as_str()) {
                        "(*)"
                    } else {
                        "( )"
                    };
                    let name = if model.name == model.id {
                        model.id.clone()
                    } else {
                        format!("{}  {}", model.name, model.id)
                    };
                    (format!("{mark} {name}"), Style::default())
                }
                SettingsEntry::SignOut => (
                    tr_in(lang, Msg::SettingsSignOut).to_string(),
                    Style::default().fg(Color::Red),
                ),
            };
            let style = if i == view.cursor {
                base.add_modifier(Modifier::REVERSED)
            } else {
                base
            };
            let r = row(area, 1 + n as u16);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(prefix, dim()),
                    Span::styled(truncate(&text, width.saturating_sub(lw as u16)), style),
                ])),
                r,
            );
            hits.push((r, Hit::SettingsItem(i)));
        }
    }

    let (msg, style) = if let Some(err) = &view.error {
        let args: Vec<(&str, &str)> = err.args.iter().map(|(k, v)| (*k, v.as_str())).collect();
        (
            trf_in(lang, err.msg, &args),
            Style::default().fg(Color::Red),
        )
    } else if view.data.is_none() || view.applying {
        let spin = SPINNER[state.spinner % SPINNER.len()];
        let what = if view.applying {
            Msg::SettingsApplying
        } else {
            Msg::SettingsLoading
        };
        (
            format!("{spin} {}", tr_in(lang, what)),
            Style::default().fg(Color::Yellow),
        )
    } else {
        (tr_in(lang, Msg::HintSettings).to_string(), dim())
    };
    frame.render_widget(
        Paragraph::new(Span::styled(truncate(&msg, width), style)),
        row(area, area.height - 1),
    );
}

fn row(area: Rect, y: u16) -> Rect {
    Rect {
        x: area.x,
        y: area.y + y,
        width: area.width,
        height: 1,
    }
}

fn label_width(state: &State) -> u16 {
    [
        Msg::LabelQuestion,
        Msg::LabelCommand,
        Msg::LabelExplain,
        Msg::LabelWarning,
    ]
    .iter()
    .map(|m| display_width(tr_in(state.lang, *m)))
    .max()
    .unwrap_or(4) as u16
        + 3
}

fn label(state: &State, msg: Msg, width: u16, style: Style) -> Span<'static> {
    // "레이블 + 공백 + ' > '" 전체 폭을 width로 맞춘다
    let text = tr_in(state.lang, msg);
    let pad = (width as usize).saturating_sub(display_width(text) + 3);
    Span::styled(format!("{text}{} > ", " ".repeat(pad)), style)
}

fn render_main(frame: &mut Frame, state: &State, area: Rect, hits: &mut Hits) {
    let lw = label_width(state);
    let content_w = area.width.saturating_sub(lw).max(1);
    let total = area.height;

    // 질문 영역 (1-2행)
    let q_focus = state.focus == Focus::Question;
    let q_wrapped = wrap_buffer(
        state.question.lines(),
        Some(state.question.cursor()),
        content_w,
    );
    let q_rows_needed = q_wrapped.rows.len().clamp(1, 2) as u16;

    // 경고 영역
    let warn_rows: u16 = if state.findings.is_empty() { 0 } else { 1 };

    // 명령어 영역
    let c_wrapped = wrap_buffer(
        state.command.lines(),
        Some(state.command.cursor()),
        content_w,
    );
    let fixed = 1 + q_rows_needed + warn_rows;
    let c_max = total.saturating_sub(fixed).max(1);
    let c_rows = (c_wrapped.rows.len() as u16).clamp(1, c_max);
    let expl_rows = total.saturating_sub(fixed + c_rows);

    let mut y = 0;
    // 질문
    let q_style = if q_focus {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        dim()
    };
    let q_start = q_wrapped
        .cursor
        .map(|(r, _)| r + 1)
        .unwrap_or(1)
        .saturating_sub(q_rows_needed as usize);
    for i in 0..q_rows_needed {
        let mut spans = vec![if i == 0 {
            label(state, Msg::LabelQuestion, lw, q_style)
        } else {
            Span::raw(" ".repeat(lw as usize))
        }];
        if state.question.is_empty() && i == 0 {
            let placeholder = if state.has_command() {
                Msg::PlaceholderFollowUp
            } else {
                Msg::PlaceholderQuestion
            };
            let text = match (&state.last_question, state.has_command()) {
                (Some(prev), true) if state.phase != Phase::Requesting => {
                    format!("{} ({prev})", tr_in(state.lang, placeholder))
                }
                _ => tr_in(state.lang, placeholder).to_string(),
            };
            spans.push(Span::styled(
                truncate(&text, content_w),
                dim().add_modifier(Modifier::ITALIC),
            ));
        } else if let Some(text) = q_wrapped.rows.get(q_start + i as usize) {
            spans.push(Span::raw(text.clone()));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), row(area, y));
        hits.push((row(area, y), Hit::Question));
        y += 1;
    }
    if q_focus
        && matches!(state.phase, Phase::Ready | Phase::ConfirmDanger)
        && let Some((r, c)) = q_wrapped.cursor
    {
        let visible = r.saturating_sub(q_start) as u16;
        frame.set_cursor_position((area.x + lw + c, area.y + visible.min(q_rows_needed - 1)));
    }

    // 명령어
    let c_focus = state.focus == Focus::Command;
    let c_style = if c_focus {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        dim()
    };
    let cursor_row = c_wrapped.cursor.map(|(r, _)| r).unwrap_or(0);
    let c_start = (cursor_row + 1).saturating_sub(c_rows as usize);
    for i in 0..c_rows {
        let mut spans = vec![if i == 0 {
            label(state, Msg::LabelCommand, lw, c_style)
        } else {
            Span::raw(" ".repeat(lw as usize))
        }];
        if let Some(text) = c_wrapped.rows.get(c_start + i as usize) {
            let style = if state.phase == Phase::Requesting {
                dim()
            } else {
                Style::default()
            };
            spans.push(Span::styled(text.clone(), style));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), row(area, y));
        hits.push((row(area, y), Hit::Command));
        y += 1;
    }
    if c_focus
        && matches!(state.phase, Phase::Ready | Phase::ConfirmDanger)
        && let Some((r, c)) = c_wrapped.cursor
    {
        let visible = (r.saturating_sub(c_start) as u16).min(c_rows - 1);
        frame.set_cursor_position((area.x + lw + c, area.y + y - c_rows + visible));
    }

    // 경고
    if let Some(top) = state.findings.first() {
        let (level_msg, color) = match top.level {
            Level::High => (Msg::RiskHigh, Color::Red),
            Level::Caution => (Msg::RiskCaution, Color::Yellow),
        };
        let mut text = format!(
            "{}: {} ({})",
            tr_in(state.lang, level_msg),
            tr_in(state.lang, top.category.msg()),
            top.evidence
        );
        if state.findings.len() > 1 {
            text.push_str(&format!(" +{}", state.findings.len() - 1));
        }
        let spans = vec![
            label(
                state,
                Msg::LabelWarning,
                lw,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(truncate(&text, content_w), Style::default().fg(color)),
        ];
        frame.render_widget(Paragraph::new(Line::from(spans)), row(area, y));
        y += 1;
    }

    // 설명과 모델 참고
    if expl_rows > 0 {
        let mut text_rows = Vec::new();
        if !state.explanation.text.is_empty() {
            text_rows.extend(wrap_words(&state.explanation.text, content_w));
        }
        for note in &state.notes {
            text_rows.extend(wrap_words(
                &format!("{}: {note}", tr_in(state.lang, Msg::LabelNotes)),
                content_w,
            ));
        }
        let expl_style = if state.explanation.stale {
            dim()
        } else {
            Style::default()
        };
        let shown = (expl_rows as usize).min(text_rows.len());
        for i in 0..shown {
            let first = i == 0;
            let mut content = text_rows[i].clone();
            if i + 1 == shown && text_rows.len() > shown {
                content = truncate(&format!("{content} …"), content_w);
            }
            let spans = vec![
                if first {
                    label(state, Msg::LabelExplain, lw, dim())
                } else {
                    Span::raw(" ".repeat(lw as usize))
                },
                Span::styled(content, expl_style),
            ];
            frame.render_widget(Paragraph::new(Line::from(spans)), row(area, y + i as u16));
        }
    }

    render_status(frame, state, row(area, area.height - 1), hits);
}

fn render_login(frame: &mut Frame, state: &State, area: Rect) {
    let width = area.width;
    let mut lines: Vec<Line> = Vec::new();
    if state.phase == Phase::LoginRequired {
        lines.push(Line::styled(
            truncate(tr_in(state.lang, Msg::LoginRequired), width),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        let spin = SPINNER[state.spinner % SPINNER.len()];
        lines.push(Line::styled(
            format!("{spin} {}", tr_in(state.lang, Msg::LoginWaiting)),
            Style::default().fg(Color::Yellow),
        ));
        // 로그인 전 전송 범위 고지 (PRD-PRIV-003)
        for part in wrap_words(tr_in(state.lang, Msg::LoginPrivacyShort), width) {
            lines.push(Line::styled(part, dim()));
        }
        if let Some(url) = &state.login.url {
            lines.push(Line::raw(truncate(
                tr_in(state.lang, Msg::LoginOpenUrl),
                width,
            )));
            // URL은 복사 가능하도록 문자 단위로 줄바꿈해 전체 표시
            for part in wrap_words(url, width) {
                lines.push(Line::styled(part, Style::default().fg(Color::Blue)));
            }
            for part in wrap_words(tr_in(state.lang, Msg::LoginPasteHint), width) {
                lines.push(Line::styled(part, dim()));
            }
        }
    }
    // 아래쪽 2행: 붙여넣기 입력, 상태줄
    let reserved = if state.phase == Phase::LoggingIn {
        2
    } else {
        1
    };
    let max_lines = area.height.saturating_sub(reserved) as usize;
    for (i, line) in lines.into_iter().take(max_lines).enumerate() {
        frame.render_widget(Paragraph::new(line), row(area, i as u16));
    }
    if state.phase == Phase::LoggingIn {
        let y = area.height - 2;
        let text = state.login_paste.text();
        let content = if text.is_empty() {
            Span::styled(
                truncate(
                    tr_in(state.lang, Msg::PlaceholderLoginPaste),
                    width.saturating_sub(2),
                ),
                dim().add_modifier(Modifier::ITALIC),
            )
        } else {
            // 긴 URL은 끝부분 표시
            let shown: String = {
                let w = display_width(&text);
                let max = width.saturating_sub(3) as usize;
                if w > max {
                    text.chars()
                        .skip(text.chars().count().saturating_sub(max))
                        .collect()
                } else {
                    text.clone()
                }
            };
            Span::raw(shown)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("> ", Style::default().fg(Color::Cyan)),
                content.clone(),
            ])),
            row(area, y),
        );
        let cursor_x = if text.is_empty() {
            2
        } else {
            2 + display_width(&content.content) as u16
        };
        frame.set_cursor_position((area.x + cursor_x.min(width - 1), area.y + y));
    }
    render_status(frame, state, row(area, area.height - 1), &mut Hits::new());
}

fn render_status(frame: &mut Frame, state: &State, area: Rect, hits: &mut Hits) {
    let lang = state.lang;
    let mut right = Vec::new();
    // 모델 이름은 설정 화면 버튼 (클릭, CR-005)
    let model_seg = state.provider.model.as_ref().map(|m| format!("[{m}]"));
    if let Some(seg) = &model_seg {
        right.push(seg.clone());
    }
    right.push(state.env.shell.name().to_string());
    if state.provider.chatgpt_plan {
        right.push(tr_in(lang, Msg::UsingChatGptPlan).to_string());
    }
    let right_text = right.join("  ");

    let (left_text, left_style) = if let Some(pending) = &state.pending {
        let spin = SPINNER[state.spinner % SPINNER.len()];
        let what = match pending.op {
            Operation::Generate => Msg::StatusGenerating,
            Operation::Revise => Msg::StatusRevising,
            Operation::Explain => Msg::StatusExplaining,
        };
        (
            format!(
                "{spin} {}  {}",
                tr_in(lang, what),
                tr_in(lang, Msg::HintRequesting)
            ),
            Style::default().fg(Color::Yellow),
        )
    } else if let Some(status) = &state.status {
        let args: Vec<(&str, &str)> = status.args.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let style = match status.kind {
            StatusKind::Info => Style::default(),
            StatusKind::Warn => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            StatusKind::Error => Style::default().fg(Color::Red),
        };
        (trf_in(lang, status.msg, &args), style)
    } else if state.explanation.stale && state.focus == Focus::Command {
        (tr_in(lang, Msg::StatusExplanationStale).to_string(), dim())
    } else {
        let hint = match (state.phase, state.focus) {
            (Phase::LoginRequired, _) => Msg::HintLogin,
            (Phase::LoggingIn, _) => Msg::HintRequesting,
            (_, Focus::Command) if state.has_command() => Msg::HintCommand,
            (_, Focus::Question) if state.has_command() => Msg::HintFollowUp,
            _ => Msg::HintQuestion,
        };
        (tr_in(lang, hint).to_string(), dim())
    };

    let width = area.width as usize;
    let right_w = display_width(&right_text);
    let left_max = width.saturating_sub(right_w + 2);
    let left = if left_max >= 10 {
        truncate(&left_text, left_max as u16)
    } else {
        truncate(&left_text, area.width)
    };
    let mut spans = vec![Span::styled(left.clone(), left_style)];
    if left_max >= 10 {
        let gap = width.saturating_sub(display_width(&left) + right_w);
        if let Some(seg) = &model_seg
            && state.phase == Phase::Ready
        {
            hits.push((
                Rect {
                    x: area.x + (display_width(&left) + gap) as u16,
                    y: area.y,
                    width: display_width(seg) as u16,
                    height: 1,
                },
                Hit::OpenSettings,
            ));
        }
        spans.push(Span::raw(" ".repeat(gap)));
        spans.push(Span::styled(right_text, dim()));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

#[cfg(test)]
mod tests;
