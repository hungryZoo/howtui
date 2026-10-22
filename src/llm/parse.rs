//! 응답 파싱 4단계 파이프라인 (SRS 5.3절).

use super::types::{LlmError, LlmResponse, Operation};

#[derive(serde::Deserialize)]
struct Raw {
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    explanation: Option<String>,
    #[serde(default)]
    notes: Option<Vec<String>>,
}

pub fn parse_response(text: &str, op: Operation, model: &str) -> Result<LlmResponse, LlmError> {
    let parsed = parse_json(text.trim())
        .or_else(|| fenced_json(text).and_then(|j| parse_json(&j)))
        .or_else(|| brace_span(text).and_then(|j| parse_json(&j)))
        .or_else(|| {
            if op == Operation::Explain {
                None
            } else {
                code_block_fallback(text)
            }
        });
    let mut resp = parsed.ok_or_else(|| LlmError::InvalidResponse {
        detail: "no JSON object or code block".into(),
    })?;
    resp.model = model.to_string();
    resp.command = resp
        .command
        .map(|c| clean_command(&c))
        .filter(|c| !c.is_empty());
    if op == Operation::Explain {
        resp.command = None;
    }
    if resp.explanation.trim().is_empty() && resp.command.is_none() {
        return Err(LlmError::InvalidResponse {
            detail: "empty response".into(),
        });
    }
    Ok(resp)
}

fn parse_json(text: &str) -> Option<LlmResponse> {
    let raw: Raw = serde_json::from_str(text).ok()?;
    if raw.command.is_none() && raw.explanation.is_none() {
        return None;
    }
    Some(LlmResponse {
        command: raw.command,
        explanation: raw.explanation.unwrap_or_default().trim().to_string(),
        notes: raw
            .notes
            .unwrap_or_default()
            .into_iter()
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty())
            .collect(),
        model: String::new(),
    })
}

fn fenced_json(text: &str) -> Option<String> {
    let start = text.find("```json")?;
    let body = &text[start + 7..];
    let end = body.find("```")?;
    Some(body[..end].trim().to_string())
}

fn brace_span(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    (end > start).then(|| text[start..=end].to_string())
}

/// 3단계: 첫 코드 블록을 명령어, 나머지를 설명으로 분리.
fn code_block_fallback(text: &str) -> Option<LlmResponse> {
    let start = text.find("```")?;
    let after = &text[start + 3..];
    let body_start = after.find('\n')? + 1;
    let body = &after[body_start..];
    let end = body.find("```")?;
    let command = body[..end].to_string();
    let rest = format!("{}{}", &text[..start], &body[end + 3..]);
    Some(LlmResponse {
        command: Some(command),
        explanation: rest.split_whitespace().collect::<Vec<_>>().join(" "),
        notes: vec![],
        model: String::new(),
    })
}

/// 공통 후처리: CRLF를 LF로, 앞뒤 빈 줄 제거, 바깥 코드 펜스 제거.
pub fn clean_command(command: &str) -> String {
    let mut c = command.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = c.trim();
    if trimmed.starts_with("```") && trimmed.ends_with("```") && trimmed.len() >= 6 {
        let inner = &trimmed[3..trimmed.len() - 3];
        let inner = inner
            .split_once('\n')
            .map(|(first, rest)| {
                if first.trim().chars().all(char::is_alphanumeric) {
                    rest
                } else {
                    inner
                }
            })
            .unwrap_or(inner);
        c = inner.to_string();
    }
    c.trim_matches(|ch: char| ch == '\n' || ch == ' ' || ch == '\t')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_json() {
        let r = parse_response(
            r#"{"command":"ls -la","explanation":"lists","notes":["a"]}"#,
            Operation::Generate,
            "m",
        )
        .unwrap();
        assert_eq!(r.command.as_deref(), Some("ls -la"));
        assert_eq!(r.explanation, "lists");
        assert_eq!(r.notes, vec!["a"]);
        assert_eq!(r.model, "m");
    }

    #[test]
    fn fenced_json_block() {
        let text = "Here you go:\n```json\n{\"command\": \"du -sh *\", \"explanation\": \"sizes\", \"notes\": []}\n```";
        assert_eq!(
            parse_response(text, Operation::Generate, "m")
                .unwrap()
                .command
                .as_deref(),
            Some("du -sh *")
        );
    }

    #[test]
    fn brace_span_with_prose() {
        let text =
            "Sure. {\"command\": \"pwd\", \"explanation\": \"prints dir\", \"notes\": []} Done.";
        assert_eq!(
            parse_response(text, Operation::Generate, "m")
                .unwrap()
                .command
                .as_deref(),
            Some("pwd")
        );
    }

    #[test]
    fn code_block_fallback_splits_command_and_text() {
        let text = "Use this:\n```bash\nfind . -size +100M\n```\nIt finds big files.";
        let r = parse_response(text, Operation::Revise, "m").unwrap();
        assert_eq!(r.command.as_deref(), Some("find . -size +100M"));
        assert_eq!(r.explanation, "Use this: It finds big files.");
    }

    #[test]
    fn unparseable_is_error() {
        assert!(matches!(
            parse_response("I cannot help", Operation::Generate, "m"),
            Err(LlmError::InvalidResponse { .. })
        ));
        assert!(matches!(
            parse_response("{}", Operation::Generate, "m"),
            Err(LlmError::InvalidResponse { .. })
        ));
    }

    #[test]
    fn explain_drops_command() {
        let r = parse_response(
            r#"{"command":"ls","explanation":"x","notes":[]}"#,
            Operation::Explain,
            "m",
        )
        .unwrap();
        assert_eq!(r.command, None);
    }

    #[test]
    fn non_command_answer_is_allowed() {
        let r = parse_response(
            r#"{"command":null,"explanation":"A pipe connects programs.","notes":[]}"#,
            Operation::Generate,
            "m",
        )
        .unwrap();
        assert_eq!(r.command, None);
    }

    #[test]
    fn cleans_crlf_and_fences_and_keeps_unicode_multiline() {
        assert_eq!(clean_command("```bash\nls\r\n```"), "ls");
        assert_eq!(
            clean_command("\nprintf '%s\\n' \"안녕\" \\\n  | grep 안\n"),
            "printf '%s\\n' \"안녕\" \\\n  | grep 안"
        );
    }
}
