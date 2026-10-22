//! 키 이벤트 → KeyAction (PRD 4절 키 바인딩 표, SRS-TUI-002). 순수 함수.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::KeyAction;
use crate::app::editor::EditOp;

pub fn map_key(ev: KeyEvent) -> Option<KeyAction> {
    if ev.kind == KeyEventKind::Release {
        return None;
    }
    let m = ev.modifiers;
    let ctrl = m.contains(KeyModifiers::CONTROL);
    let alt = m.contains(KeyModifiers::ALT);
    let shift = m.contains(KeyModifiers::SHIFT);
    let action = match ev.code {
        KeyCode::Enter if alt || shift => KeyAction::InsertNewline,
        KeyCode::Enter => KeyAction::Submit,
        KeyCode::Tab | KeyCode::BackTab => KeyAction::ToggleFocus,
        KeyCode::Esc => KeyAction::Cancel,
        // AltGr 문자(Windows: Ctrl+Alt 조합)는 일반 입력
        KeyCode::Char(c) if ctrl && alt && !c.is_ascii_alphabetic() => {
            KeyAction::Edit(EditOp::Insert(c))
        }
        KeyCode::Char(c) if ctrl => match c.to_ascii_lowercase() {
            'c' => KeyAction::Quit,
            'j' => KeyAction::InsertNewline,
            'e' => KeyAction::Explain,
            'z' => KeyAction::Undo,
            'y' => KeyAction::Redo,
            'a' => KeyAction::Edit(EditOp::Home),
            'w' => KeyAction::Edit(EditOp::DeleteWordBack),
            'u' => KeyAction::Edit(EditOp::KillToStart),
            'k' => KeyAction::Edit(EditOp::KillToEnd),
            'h' => KeyAction::Edit(EditOp::Backspace),
            _ => KeyAction::Other,
        },
        KeyCode::Char(c) if alt => match c {
            'b' => KeyAction::Edit(EditOp::WordLeft),
            'f' => KeyAction::Edit(EditOp::WordRight),
            _ => KeyAction::Other,
        },
        KeyCode::Char(c) => KeyAction::Edit(EditOp::Insert(c)),
        KeyCode::Backspace if alt || ctrl => KeyAction::Edit(EditOp::DeleteWordBack),
        KeyCode::Backspace => KeyAction::Edit(EditOp::Backspace),
        KeyCode::Delete => KeyAction::Edit(EditOp::Delete),
        KeyCode::Left if ctrl || alt => KeyAction::Edit(EditOp::WordLeft),
        KeyCode::Right if ctrl || alt => KeyAction::Edit(EditOp::WordRight),
        KeyCode::Left => KeyAction::Edit(EditOp::Left),
        KeyCode::Right => KeyAction::Edit(EditOp::Right),
        KeyCode::Up => KeyAction::Edit(EditOp::Up),
        KeyCode::Down => KeyAction::Edit(EditOp::Down),
        KeyCode::Home => KeyAction::Edit(EditOp::Home),
        KeyCode::End => KeyAction::Edit(EditOp::End),
        _ => KeyAction::Other,
    };
    Some(action)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(code: KeyCode, m: KeyModifiers) -> Option<KeyAction> {
        map_key(KeyEvent::new(code, m))
    }

    #[test]
    fn table_bindings() {
        use KeyModifiers as M;
        let cases = [
            (k(KeyCode::Enter, M::NONE), KeyAction::Submit),
            (k(KeyCode::Enter, M::ALT), KeyAction::InsertNewline),
            (k(KeyCode::Enter, M::SHIFT), KeyAction::InsertNewline),
            (k(KeyCode::Char('j'), M::CONTROL), KeyAction::InsertNewline),
            (k(KeyCode::Tab, M::NONE), KeyAction::ToggleFocus),
            (k(KeyCode::BackTab, M::SHIFT), KeyAction::ToggleFocus),
            (k(KeyCode::Esc, M::NONE), KeyAction::Cancel),
            (k(KeyCode::Char('c'), M::CONTROL), KeyAction::Quit),
            (k(KeyCode::Char('e'), M::CONTROL), KeyAction::Explain),
            (k(KeyCode::Char('z'), M::CONTROL), KeyAction::Undo),
            (k(KeyCode::Char('y'), M::CONTROL), KeyAction::Redo),
            (
                k(KeyCode::Char('w'), M::CONTROL),
                KeyAction::Edit(EditOp::DeleteWordBack),
            ),
            (
                k(KeyCode::Char('u'), M::CONTROL),
                KeyAction::Edit(EditOp::KillToStart),
            ),
            (
                k(KeyCode::Char('k'), M::CONTROL),
                KeyAction::Edit(EditOp::KillToEnd),
            ),
            (
                k(KeyCode::Char('b'), M::ALT),
                KeyAction::Edit(EditOp::WordLeft),
            ),
            (
                k(KeyCode::Char('f'), M::ALT),
                KeyAction::Edit(EditOp::WordRight),
            ),
            (
                k(KeyCode::Left, M::CONTROL),
                KeyAction::Edit(EditOp::WordLeft),
            ),
            (
                k(KeyCode::Right, M::CONTROL),
                KeyAction::Edit(EditOp::WordRight),
            ),
            (k(KeyCode::Home, M::NONE), KeyAction::Edit(EditOp::Home)),
            (k(KeyCode::End, M::NONE), KeyAction::Edit(EditOp::End)),
            (
                k(KeyCode::Backspace, M::NONE),
                KeyAction::Edit(EditOp::Backspace),
            ),
            (k(KeyCode::Delete, M::NONE), KeyAction::Edit(EditOp::Delete)),
            (
                k(KeyCode::Char('한'), M::NONE),
                KeyAction::Edit(EditOp::Insert('한')),
            ),
            (
                k(KeyCode::Char('A'), M::SHIFT),
                KeyAction::Edit(EditOp::Insert('A')),
            ),
            (
                k(KeyCode::Char('@'), M::CONTROL | M::ALT),
                KeyAction::Edit(EditOp::Insert('@')),
            ),
            (k(KeyCode::F(5), M::NONE), KeyAction::Other),
        ];
        for (got, want) in cases {
            assert_eq!(got, Some(want));
        }
    }

    #[test]
    fn release_events_are_ignored() {
        let mut ev = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        ev.kind = KeyEventKind::Release;
        assert_eq!(map_key(ev), None);
    }
}
