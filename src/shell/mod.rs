//! 대상 쉘 판정 (SRS 9.5절).

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ShellKind {
    Bash,
    Zsh,
    #[value(name = "powershell", alias = "pwsh")]
    PowerShell,
}

impl ShellKind {
    pub fn name(self) -> &'static str {
        match self {
            ShellKind::Bash => "bash",
            ShellKind::Zsh => "zsh",
            ShellKind::PowerShell => "powershell",
        }
    }

    /// LLM 지시문에 쓰는 표시 이름.
    pub fn display(self) -> &'static str {
        match self {
            ShellKind::Bash => "Bash",
            ShellKind::Zsh => "Zsh",
            ShellKind::PowerShell => "PowerShell",
        }
    }

    pub fn parse(value: &str) -> Option<ShellKind> {
        match value.trim().to_ascii_lowercase().as_str() {
            "bash" => Some(ShellKind::Bash),
            "zsh" => Some(ShellKind::Zsh),
            "powershell" | "pwsh" => Some(ShellKind::PowerShell),
            _ => None,
        }
    }

    /// OS 기본 조합 (PRD-PLAT-001).
    pub fn os_default() -> ShellKind {
        if cfg!(windows) {
            ShellKind::PowerShell
        } else if cfg!(target_os = "macos") {
            ShellKind::Zsh
        } else {
            ShellKind::Bash
        }
    }
}

impl fmt::Display for ShellKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellSource {
    Flag,
    IntegrationEnv,
    OsDefault,
}

/// 우선순위: `--shell` > `HOWTUI_SHELL` > OS 기본값. `$SHELL`은 로그인 쉘이므로 사용하지 않는다.
pub fn resolve(
    flag: Option<ShellKind>,
    env: &dyn Fn(&str) -> Option<String>,
) -> (ShellKind, ShellSource) {
    if let Some(kind) = flag {
        return (kind, ShellSource::Flag);
    }
    if let Some(kind) = env("HOWTUI_SHELL").as_deref().and_then(ShellKind::parse) {
        return (kind, ShellSource::IntegrationEnv);
    }
    (ShellKind::os_default(), ShellSource::OsDefault)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolution_order() {
        let env = |k: &str| (k == "HOWTUI_SHELL").then(|| "zsh".to_string());
        assert_eq!(
            resolve(Some(ShellKind::Bash), &env),
            (ShellKind::Bash, ShellSource::Flag)
        );
        assert_eq!(
            resolve(None, &env),
            (ShellKind::Zsh, ShellSource::IntegrationEnv)
        );
        let none = |_: &str| None;
        assert_eq!(
            resolve(None, &none),
            (ShellKind::os_default(), ShellSource::OsDefault)
        );
    }

    #[test]
    fn parses_aliases() {
        assert_eq!(ShellKind::parse("PWSH"), Some(ShellKind::PowerShell));
        assert_eq!(ShellKind::parse("fish"), None);
    }
}
