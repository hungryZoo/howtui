//! API 키 저장과 삭제 명령 (P5, PRD-LLM-022, SRS-CLI-008). 키 값은 화면, 로그, 오류 메시지에 출력하지 않는다.

use std::io::{BufRead, IsTerminal, Write};
use std::process::ExitCode;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use super::provider::auth_manager;
use crate::auth::secret::Secret;
use crate::config::{Loaded, ProviderKind};
use crate::i18n::{Msg, tr, trf};
use crate::llm::keys;
use crate::platform::paths::process_env;

/// `set-key`, `remove-key` 대상: API Key Provider만 (chatgpt 제외)
pub fn api_agent_parser() -> impl clap::builder::TypedValueParser<Value = ProviderKind> {
    use clap::builder::TypedValueParser;
    clap::builder::PossibleValuesParser::new(
        ProviderKind::ALL
            .iter()
            .filter(|k| **k != ProviderKind::Chatgpt)
            .map(|k| k.id()),
    )
    .map(|s| ProviderKind::parse(&s).unwrap_or(ProviderKind::Openai))
}

/// 터미널: 화면 표시 없는 입력 (붙여넣기 허용). 그 외: 표준 입력 첫 줄.
fn read_key(provider: &str) -> std::io::Result<Option<String>> {
    let stdin = std::io::stdin();
    if !stdin.is_terminal() {
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        return Ok(Some(line));
    }
    eprintln!("{}", trf(Msg::KeyPrompt, &[("provider", provider)]));
    crossterm::terminal::enable_raw_mode()?;
    let _ = crossterm::execute!(std::io::stderr(), crossterm::event::EnableBracketedPaste);
    let result = (|| -> std::io::Result<Option<String>> {
        let mut value = String::new();
        loop {
            match event::read()? {
                Event::Paste(text) => value.push_str(&text),
                Event::Key(k) if k.kind != KeyEventKind::Release => match k.code {
                    KeyCode::Enter => return Ok(Some(value)),
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(None);
                    }
                    KeyCode::Backspace => {
                        value.pop();
                    }
                    KeyCode::Char(c) => value.push(c),
                    _ => {}
                },
                _ => {}
            }
        }
    })();
    let _ = crossterm::execute!(std::io::stderr(), crossterm::event::DisableBracketedPaste);
    let _ = crossterm::terminal::disable_raw_mode();
    eprintln!();
    result
}

pub fn set_key_command(loaded: &Loaded, kind: ProviderKind) -> ExitCode {
    let auth = match auth_manager() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{}", trf(Msg::ErrConfig, &[("detail", &e)]));
            return ExitCode::from(8);
        }
    };
    let value = match read_key(kind.id()) {
        Ok(Some(v)) => v,
        Ok(None) => {
            eprintln!("{}", tr(Msg::LoginCancelled));
            return ExitCode::from(130);
        }
        Err(e) => {
            eprintln!("{}", trf(Msg::ErrInternal, &[("detail", &e.to_string())]));
            return ExitCode::from(1);
        }
    };
    let value = value.trim();
    if value.is_empty() {
        eprintln!("{}", tr(Msg::KeyEmpty));
        return ExitCode::from(2);
    }
    let store = auth.secret_store(true);
    if let Err(e) = store.set_secret(kind.id(), &Secret::new(value)) {
        eprintln!("{}", trf(Msg::ErrInternal, &[("detail", &e.to_string())]));
        return ExitCode::from(1);
    }
    let mut st = auth.load_state();
    st.saved_api_keys.insert(kind.id().into());
    auth.save_state(&st);
    println!(
        "{}",
        trf(
            Msg::KeySaved,
            &[("provider", kind.id()), ("store", store.kind.name())]
        )
    );
    let cfg = loaded
        .config
        .provider
        .api(kind)
        .cloned()
        .unwrap_or_default();
    if let Some(env) = keys::env_name(kind, &cfg)
        && process_env(&env).is_some_and(|v| !v.trim().is_empty())
    {
        println!("{}", trf(Msg::KeyEnvShadow, &[("env", &env)]));
    }
    let _ = std::io::stdout().flush();
    ExitCode::SUCCESS
}

pub fn remove_key_command(kind: ProviderKind) -> ExitCode {
    let auth = match auth_manager() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{}", trf(Msg::ErrConfig, &[("detail", &e)]));
            return ExitCode::from(8);
        }
    };
    let mut st = auth.load_state();
    let listed = st.saved_api_keys.remove(kind.id());
    let removed = match auth.store_if_recorded() {
        Some(store) => match store.delete_secret(kind.id()) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{}", trf(Msg::ErrInternal, &[("detail", &e.to_string())]));
                return ExitCode::from(1);
            }
        },
        None => false,
    };
    auth.save_state(&st);
    if removed || listed {
        println!("{}", trf(Msg::KeyRemoved, &[("provider", kind.id())]));
    } else {
        println!("{}", trf(Msg::KeyNotFound, &[("provider", kind.id())]));
    }
    ExitCode::SUCCESS
}
