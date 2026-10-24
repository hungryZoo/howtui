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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
pub enum ProviderKind {
    #[default]
    Chatgpt,
    Openai,
    Anthropic,
    Gemini,
    OpenaiCompatible,
}

impl ProviderKind {
    pub const ALL: [ProviderKind; 5] = [
        ProviderKind::Chatgpt,
        ProviderKind::Openai,
        ProviderKind::Anthropic,
        ProviderKind::Gemini,
        ProviderKind::OpenaiCompatible,
    ];

    /// 설정 파일과 상태 파일에 쓰는 이름
    pub fn id(self) -> &'static str {
        match self {
            ProviderKind::Chatgpt => "chatgpt",
            ProviderKind::Openai => "openai",
            ProviderKind::Anthropic => "anthropic",
            ProviderKind::Gemini => "gemini",
            ProviderKind::OpenaiCompatible => "openai_compatible",
        }
    }

    pub fn parse(s: &str) -> Option<ProviderKind> {
        Self::ALL.into_iter().find(|k| k.id() == s)
    }

    /// 화면 표시 이름
    pub fn display(self) -> &'static str {
        match self {
            ProviderKind::Chatgpt => "ChatGPT",
            ProviderKind::Openai => "OpenAI",
            ProviderKind::Anthropic => "Anthropic",
            ProviderKind::Gemini => "Gemini",
            ProviderKind::OpenaiCompatible => "OpenAI compatible",
        }
    }
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

/// API Key Provider 설정 (P5, SRS-CFG-009). 키 값 필드 없음: 환경변수 이름만 지정
#[derive(Clone, Debug, PartialEq, Eq, Default, Deserialize)]
#[serde(default)]
pub struct ApiProviderConfig {
    pub model: Option<String>,
    /// 키를 읽을 환경변수 이름. 미지정 시 Provider 기본 이름
    pub api_key_env: Option<String>,
    /// OpenAI 호환 전용 (필수). 그 외 Provider는 무시
    pub base_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Deserialize)]
#[serde(default)]
pub struct ProviderConfig {
    /// 미지정 시 설정 화면 선택(상태 파일), 그다음 chatgpt
    pub default: Option<ProviderKind>,
    pub chatgpt: ChatgptConfig,
    pub openai: ApiProviderConfig,
    pub anthropic: ApiProviderConfig,
    pub gemini: ApiProviderConfig,
    pub openai_compatible: ApiProviderConfig,
}

impl ProviderConfig {
    pub fn api(&self, kind: ProviderKind) -> Option<&ApiProviderConfig> {
        match kind {
            ProviderKind::Chatgpt => None,
            ProviderKind::Openai => Some(&self.openai),
            ProviderKind::Anthropic => Some(&self.anthropic),
            ProviderKind::Gemini => Some(&self.gemini),
            ProviderKind::OpenaiCompatible => Some(&self.openai_compatible),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ui: UiConfig,
    pub provider: ProviderConfig,
    pub context: ContextConfig,
    pub update: UpdateConfig,
}

/// 새 버전 확인 (CR-009). `HOWTUI_NO_UPDATE_CHECK=1`도 같은 효과
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct UpdateConfig {
    /// 하루 1회 백그라운드 확인과 상태줄 알림
    pub check: bool,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self { check: true }
    }
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
    "provider.openai",
    "provider.openai.model",
    "provider.openai.api_key_env",
    "provider.anthropic",
    "provider.anthropic.model",
    "provider.anthropic.api_key_env",
    "provider.gemini",
    "provider.gemini.model",
    "provider.gemini.api_key_env",
    "provider.openai_compatible",
    "provider.openai_compatible.model",
    "provider.openai_compatible.api_key_env",
    "provider.openai_compatible.base_url",
    "context",
    "context.folder",
    "update",
    "update.check",
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
    reject_secrets(&table)?;
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
    for kind in ProviderKind::ALL {
        let Some(api) = config.provider.api(kind) else {
            continue;
        };
        let name = kind.id();
        if api.model.as_deref().is_some_and(|m| m.trim().is_empty()) {
            return Err(format!("provider.{name}.model: empty value"));
        }
        if let Some(env) = &api.api_key_env
            && (env.is_empty() || !env.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        {
            return Err(format!(
                "provider.{name}.api_key_env = {env:?}: use an environment variable name (letters, digits, _)"
            ));
        }
    }
    if let Some(url) = &config.provider.openai_compatible.base_url {
        let ok = url::Url::parse(url)
            .is_ok_and(|u| matches!(u.scheme(), "http" | "https") && u.host().is_some());
        if !ok {
            return Err(format!(
                "provider.openai_compatible.base_url = {url:?}: use an http:// or https:// address"
            ));
        }
    }
    Ok(())
}

/// 설정 파일의 비밀값 필드 거부 (PRD-CFG-007). 값은 메시지에 포함하지 않음
fn reject_secrets(table: &toml::Table) -> Result<(), String> {
    let Some(toml::Value::Table(provider)) = table.get("provider") else {
        return Ok(());
    };
    for (name, value) in provider {
        if let toml::Value::Table(inner) = value {
            for key in ["api_key", "key", "token", "secret", "password"] {
                if inner.contains_key(key) {
                    return Err(format!(
                        "provider.{name}.{key}: secrets are not read from the config file. Use `howtui auth set-key {name}` or `api_key_env`"
                    ));
                }
            }
        }
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
        // `*_env`는 환경변수 이름만 담는 필드 (SRS-CFG-006)
        for key in KNOWN_KEYS.iter().filter(|k| !k.ends_with("_env")) {
            for word in ["token", "key", "secret", "password"] {
                assert!(!key.contains(word), "{key}");
            }
        }
    }

    #[test]
    fn api_provider_schema_and_secret_rejection() {
        let loaded = parse(
            "[provider]\ndefault = \"openai_compatible\"\n[provider.openai]\nmodel = \"gpt-x\"\napi_key_env = \"WORK_OPENAI\"\n[provider.openai_compatible]\nbase_url = \"http://localhost:8080/v1\"\n",
        )
        .unwrap();
        let p = &loaded.config.provider;
        assert_eq!(p.default, Some(ProviderKind::OpenaiCompatible));
        assert_eq!(p.openai.model.as_deref(), Some("gpt-x"));
        assert_eq!(p.openai.api_key_env.as_deref(), Some("WORK_OPENAI"));
        assert!(loaded.unknown_keys.is_empty(), "{:?}", loaded.unknown_keys);
        assert_eq!(parse("").unwrap().config.provider.default, None);

        // 키 값 필드는 실행 중단, 오류 메시지에 값 미포함
        let err = parse("[provider.openai]\napi_key = \"sk-secret-123\"\n").unwrap_err();
        assert!(
            err.contains("provider.openai.api_key") && !err.contains("sk-secret-123"),
            "{err}"
        );
        assert!(parse("[provider.anthropic]\napi_key_env = \"BAD NAME\"\n").is_err());
        assert!(parse("[provider.openai_compatible]\nbase_url = \"ftp://x\"\n").is_err());
        assert!(parse("[provider.gemini]\nmodel = \" \"\n").is_err());
        for kind in ProviderKind::ALL {
            assert_eq!(ProviderKind::parse(kind.id()), Some(kind));
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
