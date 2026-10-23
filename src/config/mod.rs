//! 사용자 설정 (SRS 12절). 파일 부재 시 기본값, 파일 생성 없음.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::i18n::LangSetting;

pub const MIN_HEIGHT: u16 = 3;
pub const MAX_HEIGHT: u16 = 40;
pub const DEFAULT_HEIGHT: u16 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UiMode {
    #[default]
    Inline,
    Fullscreen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    #[default]
    Chatgpt,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub mode: UiMode,
    pub inline_height: u16,
    pub language: LangSetting,
    /// 클릭 입력 (CR-005). 끄면 터미널 기본 마우스 선택과 스크롤 유지
    pub mouse: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            mode: UiMode::Inline,
            inline_height: DEFAULT_HEIGHT,
            language: LangSetting::Auto,
            mouse: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Deserialize)]
#[serde(default)]
pub struct ChatgptConfig {
    pub model: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Deserialize)]
#[serde(default)]
pub struct ProviderConfig {
    pub default: ProviderKind,
    pub chatgpt: ChatgptConfig,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ui: UiConfig,
    pub provider: ProviderConfig,
    pub context: ContextConfig,
}

/// 요청에 덧붙이는 정보 (CR-007). 기본값은 보내지 않음
#[derive(Clone, Debug, PartialEq, Eq, Default, Deserialize)]
#[serde(default)]
pub struct ContextConfig {
    /// 현재 폴더 목록 (이름, 종류, 크기) 전송 기본값
    pub folder: bool,
}

/// 설정 로드 결과. `unknown_keys`는 실행을 막지 않는 경고 대상.
#[derive(Clone, Debug, Default)]
pub struct Loaded {
    pub config: Config,
    pub path: Option<PathBuf>,
    pub exists: bool,
    pub unknown_keys: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct ConfigError {
    pub message: String,
}

/// 알려진 키 목록. 스키마 변경 시 함께 갱신.
const KNOWN_KEYS: &[&str] = &[
    "ui",
    "ui.mode",
    "ui.inline_height",
    "ui.language",
    "ui.mouse",
    "provider",
    "provider.default",
    "provider.chatgpt",
    "provider.chatgpt.model",
    "context",
    "context.folder",
];

pub fn load(path: Option<&Path>) -> Result<Loaded, ConfigError> {
    let Some(path) = path else {
        return Ok(Loaded::default());
    };
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Loaded {
                path: Some(path.to_path_buf()),
                ..Loaded::default()
            });
        }
        Err(e) => {
            return Err(ConfigError {
                message: format!("{}: {e}", path.display()),
            });
        }
    };
    let mut loaded = parse(&text).map_err(|message| ConfigError {
        message: format!("{}: {message}", path.display()),
    })?;
    loaded.path = Some(path.to_path_buf());
    loaded.exists = true;
    Ok(loaded)
}

pub fn parse(text: &str) -> Result<Loaded, String> {
    let table: toml::Table = toml::from_str(text).map_err(|e| describe_toml_error(text, &e))?;
    let mut unknown_keys = Vec::new();
    collect_unknown("", &table, &mut unknown_keys);
    let config: Config = toml::from_str(text).map_err(|e| describe_toml_error(text, &e))?;
    validate(&config)?;
    Ok(Loaded {
        config,
        path: None,
        exists: true,
        unknown_keys,
    })
}

fn validate(config: &Config) -> Result<(), String> {
    let h = config.ui.inline_height;
    if !(MIN_HEIGHT..=MAX_HEIGHT).contains(&h) {
        return Err(format!(
            "ui.inline_height = {h}: allowed range {MIN_HEIGHT}-{MAX_HEIGHT}"
        ));
    }
    if let Some(model) = &config.provider.chatgpt.model
        && model.trim().is_empty()
    {
        return Err("provider.chatgpt.model: empty value".into());
    }
    Ok(())
}

fn collect_unknown(prefix: &str, table: &toml::Table, out: &mut Vec<String>) {
    for (key, value) in table {
        let full = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        if !KNOWN_KEYS.contains(&full.as_str()) {
            out.push(full);
            continue;
        }
        if let toml::Value::Table(inner) = value {
            collect_unknown(&full, inner, out);
        }
    }
}

fn describe_toml_error(text: &str, err: &toml::de::Error) -> String {
    let message = err.message().to_string();
    match err.span() {
        Some(span) => {
            let (line, col) = line_col(text, span.start);
            format!("line {line}, column {col}: {message}")
        }
        None => message,
    }
}

fn line_col(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset.min(text.len())];
    let line = before.matches('\n').count() + 1;
    let col = before
        .rsplit('\n')
        .next()
        .map(|l| l.chars().count())
        .unwrap_or(0)
        + 1;
    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_uses_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let loaded = load(Some(&path)).unwrap();
        assert_eq!(loaded.config, Config::default());
        assert!(!loaded.exists);
        assert!(!path.exists(), "설정 파일 자동 생성 금지");
    }

    #[test]
    fn parses_full_schema() {
        let loaded = parse(
            "[ui]\nmode = \"inline\"\ninline_height = 8\nlanguage = \"ko\"\n[provider]\ndefault = \"chatgpt\"\n[provider.chatgpt]\nmodel = \"m1\"\n",
        )
        .unwrap();
        assert_eq!(loaded.config.ui.inline_height, 8);
        assert_eq!(loaded.config.ui.language, LangSetting::Ko);
        assert_eq!(loaded.config.provider.chatgpt.model.as_deref(), Some("m1"));
        assert!(loaded.unknown_keys.is_empty());
    }

    #[test]
    fn rejects_out_of_range_height() {
        for h in [2, 41] {
            let err = parse(&format!("[ui]\ninline_height = {h}\n")).unwrap_err();
            assert!(err.contains("ui.inline_height"), "{err}");
        }
    }

    #[test]
    fn rejects_wrong_types_and_values() {
        assert!(parse("[ui]\ninline_height = \"a\"\n").is_err());
        assert!(parse("[ui]\nlanguage = \"fr\"\n").is_err());
        assert!(parse("[provider]\ndefault = \"other\"\n").is_err());
    }

    #[test]
    fn syntax_error_reports_line_and_column() {
        let err = parse("[ui]\ninline_height = = 3\n").unwrap_err();
        assert!(err.starts_with("line 2, column"), "{err}");
    }

    #[test]
    fn unknown_keys_are_collected_not_fatal() {
        let loaded = parse("[ui]\ncolour = 1\n[extra]\nx = 1\n").unwrap();
        assert_eq!(
            loaded.unknown_keys,
            vec!["extra".to_string(), "ui.colour".to_string()]
        );
    }

    #[test]
    fn schema_has_no_secret_fields() {
        for key in KNOWN_KEYS {
            for word in ["token", "key", "secret", "password"] {
                assert!(!key.contains(word), "{key}");
            }
        }
    }

    #[test]
    fn schema_has_no_key_binding_fields() {
        assert!(
            !KNOWN_KEYS
                .iter()
                .any(|k| k.contains("bind") || k.contains("keymap"))
        );
    }
}
