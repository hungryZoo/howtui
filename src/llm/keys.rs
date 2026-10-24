//! API 키 조회 (P5, SRS 5.5절, SRS-LLM-015). 키 값은 `Secret`으로만 다루고 출력과 로그에 남기지 않는다.

use crate::auth::AuthManager;
use crate::auth::secret::Secret;
use crate::config::{ApiProviderConfig, ProviderKind};

/// 키 출처 (`auth status`, 설정 화면 표시용)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeySource {
    Env(String),
    Store,
    Missing,
}

#[derive(Clone, Debug)]
pub struct ResolvedKey {
    pub key: Option<Secret>,
    pub source: KeySource,
}

/// Provider별 기본 환경변수 이름. OpenAI 호환은 기본 없음
pub fn default_env(kind: ProviderKind) -> Option<&'static str> {
    match kind {
        ProviderKind::Openai => Some("OPENAI_API_KEY"),
        ProviderKind::Anthropic => Some("ANTHROPIC_API_KEY"),
        ProviderKind::Gemini => Some("GEMINI_API_KEY"),
        ProviderKind::Chatgpt | ProviderKind::OpenaiCompatible => None,
    }
}

/// 설정 `api_key_env` > 기본 이름
pub fn env_name(kind: ProviderKind, cfg: &ApiProviderConfig) -> Option<String> {
    cfg.api_key_env
        .clone()
        .or_else(|| default_env(kind).map(str::to_string))
}

/// 조회 순서: 환경변수 > 보안 저장소 (`store`가 `None`이면 저장소 생략).
pub fn resolve(
    kind: ProviderKind,
    cfg: &ApiProviderConfig,
    getenv: &dyn Fn(&str) -> Option<String>,
    store: Option<&AuthManager>,
) -> ResolvedKey {
    if let Some(name) = env_name(kind, cfg)
        && let Some(value) = getenv(&name).filter(|v| !v.trim().is_empty())
    {
        return ResolvedKey {
            key: Some(Secret::new(value.trim())),
            source: KeySource::Env(name),
        };
    }
    if let Some(auth) = store
        && auth.load_state().saved_api_keys.contains(kind.id())
    {
        match auth.secret_store(false).get_secret(kind.id()) {
            Ok(Some(key)) if !key.is_empty() => {
                return ResolvedKey {
                    key: Some(key),
                    source: KeySource::Store,
                };
            }
            Ok(_) => {}
            Err(e) => tracing::debug!(provider = kind.id(), error = %e, "api key lookup failed"),
        }
    }
    ResolvedKey {
        key: None,
        source: KeySource::Missing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k| pairs.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone())
    }

    #[test]
    fn env_then_store_then_missing() {
        let dir = tempfile::tempdir().unwrap();
        let auth = AuthManager::new(dir.path().to_path_buf(), crate::llm::http::client());
        let mut st = auth.load_state();
        st.credential_store = Some("file".into());
        auth.save_state(&st);
        let cfg = ApiProviderConfig::default();

        let r = resolve(ProviderKind::Openai, &cfg, &env(&[]), Some(&auth));
        assert_eq!(r.source, KeySource::Missing);
        assert!(r.key.is_none());

        auth.secret_store(true)
            .set_secret("openai", &Secret::new("sk-store"))
            .unwrap();
        // 저장 목록에 없으면 저장소 조회 생략
        let r = resolve(ProviderKind::Openai, &cfg, &env(&[]), Some(&auth));
        assert_eq!(r.source, KeySource::Missing);
        let mut st = auth.load_state();
        st.saved_api_keys.insert("openai".into());
        auth.save_state(&st);
        let r = resolve(ProviderKind::Openai, &cfg, &env(&[]), Some(&auth));
        assert_eq!(r.source, KeySource::Store);
        assert_eq!(r.key.unwrap().expose(), "sk-store");

        let r = resolve(
            ProviderKind::Openai,
            &cfg,
            &env(&[("OPENAI_API_KEY", " sk-env ")]),
            Some(&auth),
        );
        assert_eq!(r.source, KeySource::Env("OPENAI_API_KEY".into()));
        assert_eq!(r.key.as_ref().unwrap().expose(), "sk-env");
        assert!(!format!("{r:?}").contains("sk-env"), "Debug 출력에 키 없음");

        // 설정의 환경변수 이름 지정, 빈 값은 없음으로 처리
        let named = ApiProviderConfig {
            api_key_env: Some("WORK_KEY".into()),
            ..Default::default()
        };
        let r = resolve(
            ProviderKind::Openai,
            &named,
            &env(&[("WORK_KEY", "  "), ("OPENAI_API_KEY", "sk-env")]),
            None,
        );
        assert_eq!(r.source, KeySource::Missing);
        assert_eq!(env_name(ProviderKind::OpenaiCompatible, &cfg), None);
    }
}
