//! 설정, 데이터 경로 규칙 (SRS 12.1절, ADR-0014).

use std::path::PathBuf;

pub type EnvFn<'a> = &'a dyn Fn(&str) -> Option<String>;

pub fn process_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

fn home(env: EnvFn) -> Option<PathBuf> {
    #[cfg(windows)]
    let var = env("USERPROFILE");
    #[cfg(not(windows))]
    let var = env("HOME");
    var.map(PathBuf::from).or_else(std::env::home_dir)
}

/// 설정 파일 경로.
pub fn config_file(env: EnvFn) -> Option<PathBuf> {
    if let Some(p) = env("HOWTUI_CONFIG") {
        return Some(PathBuf::from(p));
    }
    #[cfg(windows)]
    {
        env("APPDATA").map(|d| PathBuf::from(d).join("howtui").join("config.toml"))
    }
    #[cfg(not(windows))]
    {
        let base = env("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| home(env).map(|h| h.join(".config")))?;
        Some(base.join("howtui").join("config.toml"))
    }
}

/// 상태, 자격증명 파일, 잠금 파일 디렉터리.
pub fn data_dir(env: EnvFn) -> Option<PathBuf> {
    if let Some(p) = env("HOWTUI_DATA_DIR") {
        return Some(PathBuf::from(p));
    }
    #[cfg(windows)]
    {
        env("LOCALAPPDATA").map(|d| PathBuf::from(d).join("howtui"))
    }
    #[cfg(not(windows))]
    {
        let base = env("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home(env).map(|h| h.join(".local").join("share")))?;
        Some(base.join("howtui"))
    }
}

/// 홈 디렉터리 경로를 `~`로 치환 (doctor 마스킹용).
pub fn tilde(path: &std::path::Path, env: EnvFn) -> String {
    let shown = path.display().to_string();
    match home(env) {
        Some(h) => {
            let h = h.display().to_string();
            if !h.is_empty() && shown.starts_with(&h) {
                format!("~{}", &shown[h.len()..])
            } else {
                shown
            }
        }
        None => shown,
    }
}

#[cfg(all(test, not(windows)))]
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
    fn config_path_rules() {
        let env = env_of(&[("HOME", "/home/u")]);
        assert_eq!(
            config_file(&env).unwrap(),
            PathBuf::from("/home/u/.config/howtui/config.toml")
        );
        let env = env_of(&[("HOME", "/home/u"), ("XDG_CONFIG_HOME", "/x")]);
        assert_eq!(
            config_file(&env).unwrap(),
            PathBuf::from("/x/howtui/config.toml")
        );
        let env = env_of(&[("HOME", "/home/u"), ("HOWTUI_CONFIG", "/c.toml")]);
        assert_eq!(config_file(&env).unwrap(), PathBuf::from("/c.toml"));
    }

    #[test]
    fn data_path_rules() {
        let env = env_of(&[("HOME", "/home/u")]);
        assert_eq!(
            data_dir(&env).unwrap(),
            PathBuf::from("/home/u/.local/share/howtui")
        );
        let env = env_of(&[("HOME", "/home/u"), ("HOWTUI_DATA_DIR", "/d")]);
        assert_eq!(data_dir(&env).unwrap(), PathBuf::from("/d"));
    }

    #[test]
    fn tilde_masks_home() {
        let env = env_of(&[("HOME", "/home/u")]);
        assert_eq!(
            tilde(std::path::Path::new("/home/u/.config/x"), &env),
            "~/.config/x"
        );
        assert_eq!(tilde(std::path::Path::new("/etc/x"), &env), "/etc/x");
    }
}
