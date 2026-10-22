//! 위험 판정용 단순 토큰화 (SRS-SAFE-002). 정확한 쉘 파서가 아님.
//!
//! 따옴표 내부는 하나의 인자로 묶고, 연결 연산자(`|`, `||`, `&&`, `;`, `&`, 줄바꿈)로 명령 단위를 나눈다.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Joiner {
    Start,
    Pipe,
    And,
    Or,
    Seq,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    /// 앞 명령과의 연결 방식.
    pub joiner: Joiner,
    /// 따옴표를 제거한 단어 목록 (리디렉션 제외).
    pub words: Vec<String>,
    /// 리디렉션 (연산자, 대상).
    pub redirects: Vec<(String, String)>,
}

fn finish_word(
    word: &mut String,
    in_word: &mut bool,
    seg: &mut Segment,
    pending: &mut Option<String>,
) {
    if *in_word {
        let w = std::mem::take(word);
        match pending.take() {
            Some(op) => seg.redirects.push((op, w)),
            None => seg.words.push(w),
        }
        *in_word = false;
    }
}

/// `powershell`이 참이면 백틱을 이스케이프와 줄 이어쓰기로 해석한다.
pub fn segments(input: &str, powershell: bool) -> Vec<Segment> {
    let mut out = Vec::new();
    let mut current = Segment {
        joiner: Joiner::Start,
        words: Vec::new(),
        redirects: Vec::new(),
    };
    let mut word = String::new();
    let mut in_word = false;
    let mut pending_redirect: Option<String> = None;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                in_word = true;
                for q in chars.by_ref() {
                    if q == '\'' {
                        break;
                    }
                    word.push(q);
                }
            }
            '"' => {
                in_word = true;
                while let Some(q) = chars.next() {
                    match q {
                        '"' => break,
                        '\\' => {
                            if let Some(n) = chars.next() {
                                word.push(n);
                            }
                        }
                        _ => word.push(q),
                    }
                }
            }
            '\\' if !powershell => {
                // 줄 끝 이어쓰기는 공백 처리, 그 외는 다음 문자를 그대로 사용.
                match chars.next() {
                    Some('\n') => {
                        finish_word(&mut word, &mut in_word, &mut current, &mut pending_redirect)
                    }
                    Some(n) => {
                        in_word = true;
                        word.push(n);
                    }
                    None => {}
                }
            }
            '`' if powershell => {
                // PowerShell 이어쓰기 또는 이스케이프
                match chars.peek() {
                    Some('\n') => {
                        chars.next();
                        finish_word(&mut word, &mut in_word, &mut current, &mut pending_redirect);
                    }
                    Some(_) => {
                        in_word = true;
                        word.push(chars.next().unwrap_or_default());
                    }
                    None => {}
                }
            }
            ' ' | '\t' | '\r' => {
                finish_word(&mut word, &mut in_word, &mut current, &mut pending_redirect)
            }
            '|' | '&' | ';' | '\n' => {
                // `>&`, `2>&1` 형태는 리디렉션 일부
                if c == '&' && pending_redirect.is_some() && !in_word {
                    in_word = true;
                    word.push('&');
                    continue;
                }
                finish_word(&mut word, &mut in_word, &mut current, &mut pending_redirect);
                let joiner = match (c, chars.peek()) {
                    ('|', Some('|')) => {
                        chars.next();
                        Joiner::Or
                    }
                    ('|', _) => Joiner::Pipe,
                    ('&', Some('&')) => {
                        chars.next();
                        Joiner::And
                    }
                    _ => Joiner::Seq,
                };
                if !current.words.is_empty() || !current.redirects.is_empty() {
                    out.push(std::mem::replace(
                        &mut current,
                        Segment {
                            joiner,
                            words: Vec::new(),
                            redirects: Vec::new(),
                        },
                    ));
                } else {
                    current.joiner = joiner;
                }
            }
            '>' | '<' => {
                // 앞 단어가 숫자 fd(`2>`)면 연산자에 포함
                let mut op = String::new();
                if in_word && word.chars().all(|d| d.is_ascii_digit()) {
                    op.push_str(&word);
                    word.clear();
                    in_word = false;
                } else {
                    finish_word(&mut word, &mut in_word, &mut current, &mut pending_redirect);
                }
                op.push(c);
                if c == '>' && chars.peek() == Some(&'>') {
                    chars.next();
                    op.push('>');
                }
                pending_redirect = Some(op);
            }
            _ => {
                in_word = true;
                word.push(c);
            }
        }
    }
    finish_word(&mut word, &mut in_word, &mut current, &mut pending_redirect);
    if !current.words.is_empty() || !current.redirects.is_empty() {
        out.push(current);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(s: &str) -> Vec<Vec<String>> {
        segments(s, s.contains('`'))
            .into_iter()
            .map(|s| s.words)
            .collect()
    }

    #[test]
    fn splits_on_operators() {
        let segs = segments("ls -la | grep x && echo ok; rm a || true", false);
        let joins: Vec<_> = segs.iter().map(|s| s.joiner.clone()).collect();
        assert_eq!(
            joins,
            vec![
                Joiner::Start,
                Joiner::Pipe,
                Joiner::And,
                Joiner::Seq,
                Joiner::Or
            ]
        );
        assert_eq!(segs[1].words, vec!["grep", "x"]);
    }

    #[test]
    fn quotes_group_words() {
        assert_eq!(
            words("echo 'rm -rf /' \"a b\""),
            vec![vec!["echo", "rm -rf /", "a b"]]
        );
    }

    #[test]
    fn redirects_are_separated() {
        let segs = segments("echo hi > out.txt 2>&1 >> log", false);
        assert_eq!(segs[0].words, vec!["echo", "hi"]);
        assert_eq!(
            segs[0].redirects,
            vec![
                (">".into(), "out.txt".into()),
                ("2>".into(), "&1".into()),
                (">>".into(), "log".into())
            ]
        );
    }

    #[test]
    fn line_continuations() {
        assert_eq!(
            words("printf a \\\n  | wc"),
            vec![vec!["printf", "a"], vec!["wc"]]
        );
        assert_eq!(
            words("Get-ChildItem `\n -Recurse"),
            vec![vec!["Get-ChildItem", "-Recurse"]]
        );
    }

    #[test]
    fn unicode_words() {
        assert_eq!(
            words("echo \"안녕 세계 🌏\""),
            vec![vec!["echo", "안녕 세계 🌏"]]
        );
    }
}
