//! 화면 줄바꿈 계산. 표시 폭은 Unicode 기준, 탭은 1칸.

use unicode_segmentation::UnicodeSegmentation;

use crate::app::editor::{display_width, grapheme_width};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wrapped {
    pub rows: Vec<String>,
    /// (화면 행, 화면 열)
    pub cursor: Option<(usize, u16)>,
}

/// 편집 버퍼 줄을 문자 단위로 줄바꿈하고 커서의 화면 위치를 계산한다.
pub fn wrap_buffer(lines: &[String], cursor: Option<(usize, usize)>, width: u16) -> Wrapped {
    let width = width.max(1) as usize;
    let mut rows = Vec::new();
    let mut cursor_pos = None;
    for (li, line) in lines.iter().enumerate() {
        let mut row = String::new();
        let mut row_w = 0;
        let graphemes: Vec<&str> = line.graphemes(true).collect();
        for (gi, g) in graphemes.iter().enumerate() {
            let w = grapheme_width(g);
            if row_w + w > width && row_w > 0 {
                rows.push(std::mem::take(&mut row));
                row_w = 0;
            }
            if cursor == Some((li, gi)) {
                cursor_pos = Some((rows.len(), row_w as u16));
            }
            row.push_str(if *g == "\t" { " " } else { g });
            row_w += w;
        }
        if cursor == Some((li, graphemes.len())) {
            if row_w >= width {
                rows.push(std::mem::take(&mut row));
                row_w = 0;
            }
            cursor_pos = Some((rows.len(), row_w as u16));
        }
        rows.push(row);
    }
    if rows.is_empty() {
        rows.push(String::new());
    }
    Wrapped {
        rows,
        cursor: cursor_pos,
    }
}

/// 단어 단위 줄바꿈 (긴 단어와 공백 없는 문장은 문자 단위).
pub fn wrap_words(text: &str, width: u16) -> Vec<String> {
    let width = width.max(1) as usize;
    let mut out = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        let mut line_w = 0;
        for word in paragraph.split(' ') {
            let w = display_width(word);
            let sep = if line.is_empty() { 0 } else { 1 };
            if line_w + sep + w <= width {
                if sep == 1 {
                    line.push(' ');
                }
                line.push_str(word);
                line_w += sep + w;
                continue;
            }
            if !line.is_empty() {
                out.push(std::mem::take(&mut line));
                line_w = 0;
            }
            if w <= width {
                line.push_str(word);
                line_w = w;
            } else {
                for g in word.graphemes(true) {
                    let gw = grapheme_width(g);
                    if line_w + gw > width && !line.is_empty() {
                        out.push(std::mem::take(&mut line));
                        line_w = 0;
                    }
                    line.push_str(g);
                    line_w += gw;
                }
            }
        }
        out.push(line);
    }
    out
}

/// 폭에 맞게 자르고 넘치면 말줄임표.
pub fn truncate(text: &str, width: u16) -> String {
    let width = width as usize;
    if display_width(text) <= width {
        return text.to_string();
    }
    let mut out = String::new();
    let mut w = 0;
    for g in text.graphemes(true) {
        let gw = grapheme_width(g);
        if w + gw + 1 > width {
            break;
        }
        out.push_str(g);
        w += gw;
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_and_tracks_cursor() {
        let lines = vec!["abcdef".to_string(), "gh".to_string()];
        let w = wrap_buffer(&lines, Some((0, 4)), 4);
        assert_eq!(w.rows, vec!["abcd", "ef", "gh"]);
        assert_eq!(w.cursor, Some((1, 0)));
        let w = wrap_buffer(&lines, Some((1, 2)), 4);
        assert_eq!(w.cursor, Some((2, 2)));
    }

    #[test]
    fn cursor_at_full_row_end_moves_to_next_row() {
        let w = wrap_buffer(&["abcd".to_string()], Some((0, 4)), 4);
        assert_eq!(w.rows, vec!["abcd", ""]);
        assert_eq!(w.cursor, Some((1, 0)));
    }

    #[test]
    fn wide_chars_do_not_split() {
        let w = wrap_buffer(&["한글한".to_string()], Some((0, 3)), 5);
        assert_eq!(w.rows, vec!["한글", "한"]);
        assert_eq!(w.cursor, Some((1, 2)));
    }

    #[test]
    fn word_wrap() {
        assert_eq!(
            wrap_words("one two three four", 9),
            vec!["one two", "three", "four"]
        );
        assert_eq!(wrap_words("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
        assert_eq!(
            wrap_words("현재 디렉터리 이하 파일", 10),
            vec!["현재", "디렉터리", "이하 파일"]
        );
    }

    #[test]
    fn truncation() {
        assert_eq!(truncate("hello world", 8), "hello w…");
        assert_eq!(truncate("한글한글", 5), "한글…");
        assert_eq!(truncate("ok", 5), "ok");
    }
}
