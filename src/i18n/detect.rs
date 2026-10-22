//! 언어 결정 규칙 (SRS-I18N-002): `HOWTUI_LANG` > 설정 `ui.language` > OS 로캘 > 영어.

use super::Lang;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LangSetting {
    #[default]
    Auto,
    En,
    Ko,
}

/// 환경변수 조회 함수를 받아 언어를 결정한다 (테스트에서 환경 주입 목적).
pub fn resolve_lang(setting: LangSetting, env: &dyn Fn(&str) -> Option<String>) -> Lang {
    if let Some(lang) = env("HOWTUI_LANG").as_deref().and_then(Lang::parse) {
        return lang;
    }
    match setting {
        LangSetting::En => Lang::En,
        LangSetting::Ko => Lang::Ko,
        LangSetting::Auto => detect_from_env(env)
            .or_else(detect_system_lang)
            .unwrap_or(Lang::En),
    }
}

/// Unix 로캘 환경변수 판정. 첫 번째 비어 있지 않은 값 기준.
fn detect_from_env(env: &dyn Fn(&str) -> Option<String>) -> Option<Lang> {
    let value = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|k| env(k))
        .find(|v| !v.is_empty())?;
    Some(if value.to_ascii_lowercase().starts_with("ko") {
        Lang::Ko
    } else {
        Lang::En
    })
}

#[cfg(windows)]
pub fn detect_system_lang() -> Option<Lang> {
    use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
    // LANGID 하위 10비트가 주 언어 ID, 한국어는 0x12.
    let langid = unsafe { GetUserDefaultUILanguage() };
    Some(if langid & 0x3ff == 0x12 {
        Lang::Ko
    } else {
        Lang::En
    })
}

#[cfg(not(windows))]
pub fn detect_system_lang() -> Option<Lang> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k| map.get(k).cloned()
    }

    #[test]
    fn env_override_wins_over_config() {
        let env = env_of(&[("HOWTUI_LANG", "ko")]);
        assert_eq!(resolve_lang(LangSetting::En, &env), Lang::Ko);
    }

    #[test]
    fn config_wins_over_locale() {
        let env = env_of(&[("LANG", "en_US.UTF-8")]);
        assert_eq!(resolve_lang(LangSetting::Ko, &env), Lang::Ko);
    }

    #[test]
    fn auto_uses_locale_order() {
        let env = env_of(&[("LANG", "ko_KR.UTF-8")]);
        assert_eq!(resolve_lang(LangSetting::Auto, &env), Lang::Ko);
        let env = env_of(&[("LC_ALL", "C"), ("LANG", "ko_KR.UTF-8")]);
        assert_eq!(resolve_lang(LangSetting::Auto, &env), Lang::En);
    }

    #[cfg(not(windows))]
    #[test]
    fn auto_defaults_to_english() {
        let env = env_of(&[]);
        assert_eq!(resolve_lang(LangSetting::Auto, &env), Lang::En);
    }

    #[test]
    fn invalid_env_value_is_ignored() {
        let env = env_of(&[("HOWTUI_LANG", "fr"), ("LANG", "ko_KR.UTF-8")]);
        assert_eq!(resolve_lang(LangSetting::Auto, &env), Lang::Ko);
    }
}
