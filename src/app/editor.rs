//! 편집 버퍼 (ADR-0015, SRS-DATA-002, SRS-DATA-003).
//!
//! 커서는 (줄, grapheme 인덱스). 화면 열 계산은 Unicode 표시 폭 기준.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub const UNDO_LIMIT: usize = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditOp {
    Insert(char),
    Newline,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    WordLeft,
    WordRight,
    DeleteWordBack,
    KillToStart,
    KillToEnd,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Snapshot {
    lines: Vec<String>,
    row: usize,
    col: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LastEdit {
    None,
    Insert,
    Delete,
}

#[derive(Clone, Debug)]
pub struct TextBuffer {
    lines: Vec<String>,
    row: usize,
    col: usize,
    preferred_width: Option<usize>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    last_edit: LastEdit,
    read_only: bool,
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// grapheme 1개의 화면 폭. 탭은 1칸 공백으로 표시.
pub fn grapheme_width(g: &str) -> usize {
    if g == "\t" {
        1
    } else {
        UnicodeWidthStr::width(g)
    }
}

pub fn display_width(s: &str) -> usize {
    s.graphemes(true).map(grapheme_width).sum()
}

fn normalize_newlines(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            row: 0,
            col: 0,
            preferred_width: None,
            undo: Vec::new(),
            redo: Vec::new(),
            last_edit: LastEdit::None,
            read_only: false,
        }
    }

    pub fn from_text(text: &str) -> Self {
        let mut b = Self::new();
        b.set_text(text);
        b
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn is_empty(&self) -> bool {
        self.lines.len() == 1 && self.lines[0].is_empty()
    }

    pub fn is_blank(&self) -> bool {
        self.lines.iter().all(|l| l.trim().is_empty())
    }

    pub fn cursor(&self) -> (usize, usize) {
        (self.row, self.col)
    }

    /// 커서 앞까지의 화면 폭.
    pub fn cursor_display_col(&self) -> usize {
        self.line_graphemes(self.row)
            .iter()
            .take(self.col)
            .map(|g| grapheme_width(g))
            .sum()
    }

    pub fn read_only(&self) -> bool {
        self.read_only
    }

    pub fn set_read_only(&mut self, value: bool) {
        self.read_only = value;
    }

    /// Undo 이력 없이 내용 교체 (초기값 설정, 입력란 비우기용). 커서는 끝.
    pub fn set_text(&mut self, text: &str) {
        self.lines = normalize_newlines(text)
            .split('\n')
            .map(str::to_string)
            .collect();
        self.move_to_end_of_text();
        self.undo.clear();
        self.redo.clear();
        self.last_edit = LastEdit::None;
    }

    /// Undo 1단계로 기록하는 내용 교체 (AI 응답 반영, PRD-EDIT-006).
    pub fn replace_undoable(&mut self, text: &str) {
        self.push_undo();
        self.lines = normalize_newlines(text)
            .split('\n')
            .map(str::to_string)
            .collect();
        self.move_to_end_of_text();
        self.last_edit = LastEdit::None;
    }

    pub fn clear(&mut self) {
        self.set_text("");
    }

    /// 커서 위치를 지정한 문자열 길이 뒤로 이동 (쉘 입력란 커서 복원용).
    pub fn set_cursor_after_prefix(&mut self, prefix: &str) {
        let prefix = normalize_newlines(prefix);
        let text = self.text();
        if !text.starts_with(&prefix) {
            return;
        }
        let rows: Vec<&str> = prefix.split('\n').collect();
        self.row = rows.len() - 1;
        self.col = rows.last().map(|l| l.graphemes(true).count()).unwrap_or(0);
    }

    fn move_to_end_of_text(&mut self) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.row = self.lines.len() - 1;
        self.col = self.line_len(self.row);
        self.preferred_width = None;
    }

    fn line_graphemes(&self, row: usize) -> Vec<&str> {
        self.lines[row].graphemes(true).collect()
    }

    fn line_len(&self, row: usize) -> usize {
        self.lines[row].graphemes(true).count()
    }

    fn byte_index(&self, row: usize, col: usize) -> usize {
        self.lines[row]
            .grapheme_indices(true)
            .nth(col)
            .map(|(i, _)| i)
            .unwrap_or(self.lines[row].len())
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            lines: self.lines.clone(),
            row: self.row,
            col: self.col,
        }
    }

    fn push_undo(&mut self) {
        self.undo.push(self.snapshot());
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// 연속 입력과 연속 삭제는 Undo 1단계로 묶는다.
    fn begin_edit(&mut self, kind: LastEdit) {
        if self.last_edit != kind || kind == LastEdit::None {
            self.push_undo();
        }
        self.last_edit = kind;
    }

    pub fn undo(&mut self) -> bool {
        if self.read_only {
            return false;
        }
        match self.undo.pop() {
            Some(s) => {
                self.redo.push(self.snapshot());
                self.restore(s);
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        if self.read_only {
            return false;
        }
        match self.redo.pop() {
            Some(s) => {
                self.undo.push(self.snapshot());
                self.restore(s);
                true
            }
            None => false,
        }
    }

    fn restore(&mut self, s: Snapshot) {
        self.lines = s.lines;
        self.row = s.row.min(self.lines.len() - 1);
        self.col = s.col.min(self.line_len(self.row));
        self.last_edit = LastEdit::None;
        self.preferred_width = None;
    }

    /// 붙여넣기: 원문 그대로 삽입, Undo 1단계.
    pub fn insert_str(&mut self, text: &str) -> bool {
        if self.read_only || text.is_empty() {
            return false;
        }
        self.begin_edit(LastEdit::None);
        let text = normalize_newlines(text);
        let at = self.byte_index(self.row, self.col);
        let tail = self.lines[self.row].split_off(at);
        let mut parts = text.split('\n');
        if let Some(first) = parts.next() {
            self.lines[self.row].push_str(first);
        }
        for part in parts {
            self.row += 1;
            self.lines.insert(self.row, part.to_string());
        }
        self.col = self.line_len(self.row);
        self.lines[self.row].push_str(&tail);
        self.preferred_width = None;
        true
    }

    /// 편집 연산 적용. 내용이 바뀌면 true.
    pub fn apply(&mut self, op: EditOp) -> bool {
        let vertical = matches!(op, EditOp::Up | EditOp::Down);
        if !vertical {
            self.preferred_width = None;
        }
        let is_move = matches!(
            op,
            EditOp::Left
                | EditOp::Right
                | EditOp::Up
                | EditOp::Down
                | EditOp::Home
                | EditOp::End
                | EditOp::WordLeft
                | EditOp::WordRight
        );
        if is_move {
            self.last_edit = LastEdit::None;
            self.apply_move(op);
            return false;
        }
        if self.read_only {
            return false;
        }
        match op {
            EditOp::Insert(c) => {
                self.begin_edit(LastEdit::Insert);
                let at = self.byte_index(self.row, self.col);
                let before = self.line_len(self.row);
                self.lines[self.row].insert(at, c);
                // 결합 문자는 앞 grapheme에 합쳐지므로 grapheme 수 변화만큼 이동.
                let after = self.line_len(self.row);
                self.col += after.saturating_sub(before);
                true
            }
            EditOp::Newline => {
                self.begin_edit(LastEdit::None);
                let at = self.byte_index(self.row, self.col);
                let tail = self.lines[self.row].split_off(at);
                self.row += 1;
                self.lines.insert(self.row, tail);
                self.col = 0;
                true
            }
            EditOp::Backspace => {
                if self.col > 0 {
                    self.begin_edit(LastEdit::Delete);
                    let start = self.byte_index(self.row, self.col - 1);
                    let end = self.byte_index(self.row, self.col);
                    self.lines[self.row].replace_range(start..end, "");
                    self.col -= 1;
                    true
                } else if self.row > 0 {
                    self.begin_edit(LastEdit::Delete);
                    let line = self.lines.remove(self.row);
                    self.row -= 1;
                    self.col = self.line_len(self.row);
                    self.lines[self.row].push_str(&line);
                    true
                } else {
                    false
                }
            }
            EditOp::Delete => {
                if self.col < self.line_len(self.row) {
                    self.begin_edit(LastEdit::Delete);
                    let start = self.byte_index(self.row, self.col);
                    let end = self.byte_index(self.row, self.col + 1);
                    self.lines[self.row].replace_range(start..end, "");
                    true
                } else if self.row + 1 < self.lines.len() {
                    self.begin_edit(LastEdit::Delete);
                    let next = self.lines.remove(self.row + 1);
                    self.lines[self.row].push_str(&next);
                    true
                } else {
                    false
                }
            }
            EditOp::DeleteWordBack => {
                let start = self.word_start_before(self.col);
                if start == self.col {
                    return self.apply(EditOp::Backspace);
                }
                self.begin_edit(LastEdit::None);
                let s = self.byte_index(self.row, start);
                let e = self.byte_index(self.row, self.col);
                self.lines[self.row].replace_range(s..e, "");
                self.col = start;
                true
            }
            EditOp::KillToStart => {
                if self.col == 0 {
                    return false;
                }
                self.begin_edit(LastEdit::None);
                let e = self.byte_index(self.row, self.col);
                self.lines[self.row].replace_range(..e, "");
                self.col = 0;
                true
            }
            EditOp::KillToEnd => {
                if self.col == self.line_len(self.row) {
                    return false;
                }
                self.begin_edit(LastEdit::None);
                let s = self.byte_index(self.row, self.col);
                self.lines[self.row].truncate(s);
                true
            }
            _ => false,
        }
    }

    fn apply_move(&mut self, op: EditOp) {
        match op {
            EditOp::Left => {
                if self.col > 0 {
                    self.col -= 1;
                } else if self.row > 0 {
                    self.row -= 1;
                    self.col = self.line_len(self.row);
                }
            }
            EditOp::Right => {
                if self.col < self.line_len(self.row) {
                    self.col += 1;
                } else if self.row + 1 < self.lines.len() {
                    self.row += 1;
                    self.col = 0;
                }
            }
            EditOp::Up | EditOp::Down => {
                let target = if op == EditOp::Up {
                    self.row.checked_sub(1)
                } else {
                    Some(self.row + 1).filter(|r| *r < self.lines.len())
                };
                let Some(target) = target else {
                    self.col = if op == EditOp::Up {
                        0
                    } else {
                        self.line_len(self.row)
                    };
                    return;
                };
                let want = *self
                    .preferred_width
                    .get_or_insert(self.cursor_display_col());
                self.row = target;
                let mut width = 0;
                let mut col = 0;
                for g in self.line_graphemes(target) {
                    let w = grapheme_width(g);
                    if width + w > want {
                        break;
                    }
                    width += w;
                    col += 1;
                }
                self.col = col;
            }
            EditOp::Home => self.col = 0,
            EditOp::End => self.col = self.line_len(self.row),
            EditOp::WordLeft => {
                if self.col == 0 && self.row > 0 {
                    self.row -= 1;
                    self.col = self.line_len(self.row);
                } else {
                    self.col = self.word_start_before(self.col);
                }
            }
            EditOp::WordRight => {
                let len = self.line_len(self.row);
                if self.col == len && self.row + 1 < self.lines.len() {
                    self.row += 1;
                    self.col = 0;
                } else {
                    let gs = self.line_graphemes(self.row);
                    let mut c = self.col;
                    while c < len && is_space(gs[c]) {
                        c += 1;
                    }
                    while c < len && !is_space(gs[c]) {
                        c += 1;
                    }
                    self.col = c;
                }
            }
            _ => {}
        }
    }

    fn word_start_before(&self, col: usize) -> usize {
        let gs = self.line_graphemes(self.row);
        let mut c = col;
        while c > 0 && is_space(gs[c - 1]) {
            c -= 1;
        }
        while c > 0 && !is_space(gs[c - 1]) {
            c -= 1;
        }
        c
    }
}

fn is_space(g: &str) -> bool {
    g.chars().all(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(s: &str) -> TextBuffer {
        let mut b = TextBuffer::new();
        for c in s.chars() {
            if c == '\n' {
                b.apply(EditOp::Newline);
            } else {
                b.apply(EditOp::Insert(c));
            }
        }
        b
    }

    #[test]
    fn basic_editing_and_cursor() {
        let mut b = typed("ls -la");
        assert_eq!(b.text(), "ls -la");
        b.apply(EditOp::Home);
        b.apply(EditOp::Right);
        b.apply(EditOp::Right);
        b.apply(EditOp::Insert('X'));
        assert_eq!(b.text(), "lsX -la");
        b.apply(EditOp::Backspace);
        b.apply(EditOp::End);
        b.apply(EditOp::Backspace);
        assert_eq!(b.text(), "ls -l");
        b.apply(EditOp::Home);
        b.apply(EditOp::Delete);
        assert_eq!(b.text(), "s -l");
    }

    #[test]
    fn word_motion_and_kill() {
        let mut b = typed("git commit  -m msg");
        b.apply(EditOp::WordLeft);
        assert_eq!(b.cursor(), (0, 15));
        b.apply(EditOp::WordLeft);
        b.apply(EditOp::WordLeft);
        assert_eq!(b.cursor(), (0, 4));
        b.apply(EditOp::WordRight);
        assert_eq!(b.cursor(), (0, 10));
        b.apply(EditOp::KillToEnd);
        assert_eq!(b.text(), "git commit");
        b.apply(EditOp::DeleteWordBack);
        assert_eq!(b.text(), "git ");
        b.apply(EditOp::KillToStart);
        assert_eq!(b.text(), "");
    }

    #[test]
    fn multiline_navigation_and_join() {
        let mut b = typed("abc\nde\nfghij");
        assert_eq!(b.cursor(), (2, 5));
        b.apply(EditOp::Up);
        assert_eq!(b.cursor(), (1, 2));
        b.apply(EditOp::Up);
        assert_eq!(b.cursor(), (0, 3), "선호 열 유지");
        b.apply(EditOp::Down);
        b.apply(EditOp::Down);
        assert_eq!(b.cursor(), (2, 5));
        b.apply(EditOp::Home);
        b.apply(EditOp::Backspace);
        assert_eq!(b.text(), "abc\ndefghij");
        b.apply(EditOp::End);
        b.apply(EditOp::Up);
        b.apply(EditOp::End);
        b.apply(EditOp::Delete);
        assert_eq!(b.text(), "abcdefghij");
    }

    #[test]
    fn undo_groups_typing_and_steps() {
        let mut b = typed("hello");
        b.apply(EditOp::Backspace);
        b.apply(EditOp::Backspace);
        b.insert_str(" world");
        assert_eq!(b.text(), "hel world");
        assert!(b.undo());
        assert_eq!(b.text(), "hel");
        assert!(b.undo());
        assert_eq!(b.text(), "hello");
        assert!(b.undo());
        assert_eq!(b.text(), "");
        assert!(b.redo());
        assert_eq!(b.text(), "hello");
    }

    #[test]
    fn ai_replacement_is_single_undo_step() {
        let mut b = typed("ls");
        b.replace_undoable("ls -la");
        assert_eq!(b.text(), "ls -la");
        b.undo();
        assert_eq!(b.text(), "ls");
    }

    #[test]
    fn paste_keeps_newlines_and_normalizes_crlf() {
        let mut b = TextBuffer::new();
        b.insert_str("a\r\nb");
        assert_eq!(b.text(), "a\nb");
        assert_eq!(b.cursor(), (1, 1));
    }

    #[test]
    fn unicode_graphemes_and_width() {
        let mut b = typed("한글 😀 e\u{301}");
        assert_eq!(b.cursor(), (0, 6));
        assert_eq!(b.cursor_display_col(), 2 + 2 + 1 + 2 + 1 + 1);
        b.apply(EditOp::Backspace);
        assert_eq!(b.text(), "한글 😀 ");
        b.apply(EditOp::Left);
        b.apply(EditOp::Backspace);
        assert_eq!(b.text(), "한글  ");
        assert_eq!(display_width("한글"), 4);
    }

    #[test]
    fn vertical_move_respects_wide_chars() {
        let mut b = typed("한글한글\nabcdefgh");
        b.apply(EditOp::Up);
        assert_eq!(b.cursor(), (0, 4));
        b.apply(EditOp::Home);
        b.apply(EditOp::Right);
        b.apply(EditOp::Down);
        assert_eq!(b.cursor(), (1, 2));
    }

    #[test]
    fn read_only_blocks_changes_but_allows_moves() {
        let mut b = typed("abc");
        b.set_read_only(true);
        assert!(!b.apply(EditOp::Insert('x')));
        assert!(!b.apply(EditOp::Backspace));
        assert!(!b.insert_str("zz"));
        b.apply(EditOp::Home);
        assert_eq!(b.cursor(), (0, 0));
        assert_eq!(b.text(), "abc");
    }

    #[test]
    fn cursor_after_prefix() {
        let mut b = TextBuffer::from_text("tar -x file");
        b.set_cursor_after_prefix("tar -x");
        assert_eq!(b.cursor(), (0, 6));
    }

    #[test]
    fn undo_limit_is_enforced() {
        let mut b = TextBuffer::new();
        for _ in 0..(UNDO_LIMIT + 50) {
            b.apply(EditOp::Insert('a'));
            b.apply(EditOp::Home);
        }
        let mut count = 0;
        while b.undo() {
            count += 1;
        }
        assert_eq!(count, UNDO_LIMIT);
    }
}
