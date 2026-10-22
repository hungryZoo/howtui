//! 프롬프트 구성 (SRS 5.2절). 전송 필드는 이 모듈에서만 결정한다 (SRS-SEC-004).

use serde_json::json;

use super::types::{EnvContext, LlmRequest, LlmResponse, Operation};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prompt {
    pub instructions: String,
    pub messages: Vec<(Role, String)>,
}

pub fn instructions(env: &EnvContext) -> String {
    let shell = env.shell.display();
    let version = env
        .shell_version
        .as_deref()
        .map(|v| format!(" {v}"))
        .unwrap_or_default();
    format!(
        "You are howtui, a shell command assistant running inside the user's terminal.\n\
         Target environment: OS={os}, CPU architecture={arch}, shell={shell}{version}.\n\
         Reply with exactly one JSON object and nothing else:\n\
         {{\"command\": string or null, \"explanation\": string, \"notes\": [string]}}\n\
         Rules:\n\
         - \"command\" must be directly usable in {shell} on {os}: no Markdown code fences, no prompt symbols, no surrounding quotes.\n\
         - Prefer a single command line. Use several lines only when required, with valid {shell} line continuations.\n\
         - Keep the user's own edits unless the request asks to change them.\n\
         - \"explanation\": one to three short sentences about what the command does, written in the same language as the user's request.\n\
         - \"notes\": short warnings about destructive effects, required privileges or assumptions; an empty array when there are none.\n\
         - Never claim that you ran anything. Never invent secrets, tokens or passwords; use placeholders such as <PATH>.\n\
         - If the request is not about a shell command, set \"command\" to null and answer briefly in \"explanation\".",
        os = env.os.display(),
        arch = env.arch,
    )
}

fn fence(command: &str) -> String {
    format!("```\n{command}\n```")
}

/// 사용자 메시지 본문. 이력 재구성에도 같은 함수를 사용한다.
pub fn user_message(
    op: Operation,
    shell_name: &str,
    question: Option<&str>,
    command: Option<&str>,
) -> String {
    match op {
        Operation::Generate => question.unwrap_or_default().to_string(),
        Operation::Revise => format!(
            "Current command (it may include the user's own edits):\n{}\nRevise it according to this request: {}",
            fence(command.unwrap_or_default()),
            question.unwrap_or_default()
        ),
        Operation::Explain => format!(
            "Explain what this {shell_name} command does. Set \"command\" to null.\n{}",
            fence(command.unwrap_or_default())
        ),
    }
}

pub fn assistant_message(response: &LlmResponse) -> String {
    json!({ "command": response.command, "explanation": response.explanation, "notes": response.notes }).to_string()
}

pub fn build(req: &LlmRequest) -> Prompt {
    let shell = req.env.shell.display();
    let mut messages = Vec::new();
    for turn in &req.history {
        messages.push((
            Role::User,
            user_message(
                turn.op,
                shell,
                turn.question.as_deref(),
                turn.command_sent.as_deref(),
            ),
        ));
        messages.push((Role::Assistant, assistant_message(&turn.response)));
    }
    messages.push((
        Role::User,
        user_message(
            req.op,
            shell,
            req.question.as_deref(),
            req.command.as_deref(),
        ),
    ));
    Prompt {
        instructions: instructions(&req.env),
        messages,
    }
}

/// 구조화 출력용 JSON schema.
pub fn response_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "command": { "type": ["string", "null"] },
            "explanation": { "type": "string" },
            "notes": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["command", "explanation", "notes"],
        "additionalProperties": false
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::types::{OsKind, Turn};
    use crate::shell::ShellKind;

    fn env(shell: ShellKind) -> EnvContext {
        EnvContext {
            os: OsKind::Linux,
            arch: "aarch64",
            shell,
            shell_version: Some("5.2".into()),
        }
    }

    fn req(op: Operation, question: Option<&str>, command: Option<&str>) -> LlmRequest {
        LlmRequest {
            id: 1,
            op,
            env: env(ShellKind::Bash),
            question: question.map(str::to_string),
            command: command.map(str::to_string),
            history: vec![],
        }
    }

    #[test]
    fn instructions_name_environment_and_language_rule() {
        let text = instructions(&env(ShellKind::PowerShell));
        assert!(text.contains("OS=Linux"));
        assert!(text.contains("CPU architecture=aarch64"));
        assert!(text.contains("shell=PowerShell 5.2"));
        assert!(text.contains("same language as the user's request"));
    }

    #[test]
    fn generate_sends_question_only() {
        let p = build(&req(Operation::Generate, Some("find big files"), None));
        assert_eq!(p.messages, vec![(Role::User, "find big files".to_string())]);
    }

    #[test]
    fn revise_includes_edited_command() {
        let p = build(&req(Operation::Revise, Some("add -h"), Some("du -s /var")));
        assert!(p.messages[0].1.contains("du -s /var"));
        assert!(p.messages[0].1.contains("add -h"));
    }

    #[test]
    fn explain_requests_null_command() {
        let p = build(&req(Operation::Explain, None, Some("tar -xzf a.tgz")));
        assert!(p.messages[0].1.contains("Set \"command\" to null"));
        assert!(p.messages[0].1.contains("tar -xzf a.tgz"));
    }

    #[test]
    fn history_becomes_alternating_messages() {
        let mut r = req(Operation::Revise, Some("sort"), Some("ls"));
        r.history.push(Turn {
            op: Operation::Generate,
            question: Some("list".into()),
            command_sent: None,
            response: LlmResponse {
                command: Some("ls".into()),
                explanation: "lists".into(),
                notes: vec![],
                model: String::new(),
            },
        });
        let p = build(&r);
        let roles: Vec<Role> = p.messages.iter().map(|m| m.0).collect();
        assert_eq!(roles, vec![Role::User, Role::Assistant, Role::User]);
        let assistant: serde_json::Value = serde_json::from_str(&p.messages[1].1).unwrap();
        assert_eq!(assistant["command"], "ls");
    }

    /// 전송 내용 고정: 환경변수, 작업 디렉터리, 사용자 이름이 지시문에 포함되지 않음 (TC-PRIV-001 보조).
    #[test]
    fn instructions_contain_only_allowed_context() {
        let text = instructions(&env(ShellKind::Bash));
        let cwd = std::env::current_dir().unwrap().display().to_string();
        assert!(!text.contains(&cwd));
        if let Ok(home) = std::env::var("HOME") {
            assert!(!text.contains(&home));
        }
    }
}
