//! `howtui init <쉘>` 스크립트 생성 (SRS-SH-001, SRS-SH-006, SRS-SH-013).
//!
//! 스크립트는 바이너리에 내장한다. `#@bind`와 `#@wrap` 구간은 옵션에 따라 포함하거나 제외한다.

use super::ShellKind;

const BASH: &str = include_str!("scripts/howtui.bash");
const ZSH: &str = include_str!("scripts/howtui.zsh");
const POWERSHELL: &str = include_str!("scripts/howtui.ps1");

/// 단축키: Ctrl 또는 Alt + 영문자 1개.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    pub alt: bool,
    pub letter: char,
}

impl Default for Key {
    fn default() -> Self {
        Key {
            alt: false,
            letter: 'g',
        }
    }
}

/// 쉘 기본 동작과 겹쳐 입력이 깨지는 Ctrl 조합 (Enter, Tab, Backspace, 종료, 일시정지, 흐름 제어).
const RESERVED_CTRL: &[char] = &['c', 'd', 'h', 'i', 'j', 'm', 'q', 's', 'z'];

#[derive(Debug, PartialEq, Eq)]
pub enum KeyError {
    Format,
    Reserved,
}

impl Key {
    /// `ctrl+g`, `Ctrl-G`, `alt+h` 형식.
    pub fn parse(text: &str) -> Result<Key, KeyError> {
        let lower = text.trim().to_ascii_lowercase();
        let (modifier, rest) = lower.split_once(['+', '-']).ok_or(KeyError::Format)?;
        let mut chars = rest.chars();
        let (Some(letter), None) = (chars.next(), chars.next()) else {
            return Err(KeyError::Format);
        };
        if !letter.is_ascii_lowercase() {
            return Err(KeyError::Format);
        }
        let alt = match modifier {
            "ctrl" | "control" | "c" => false,
            "alt" | "meta" | "m" => true,
            _ => return Err(KeyError::Format),
        };
        if !alt && RESERVED_CTRL.contains(&letter) {
            return Err(KeyError::Reserved);
        }
        Ok(Key { alt, letter })
    }

    pub fn name(self) -> String {
        let m = if self.alt { "Alt" } else { "Ctrl" };
        format!("{m}+{}", self.letter.to_ascii_uppercase())
    }

    fn bash(self) -> String {
        if self.alt {
            format!("\\e{}", self.letter)
        } else {
            format!("\\C-{}", self.letter)
        }
    }

    fn zsh(self) -> String {
        if self.alt {
            format!("^[{}", self.letter)
        } else {
            format!("^{}", self.letter.to_ascii_uppercase())
        }
    }

    fn powershell(self) -> String {
        let m = if self.alt { "Alt" } else { "Ctrl" };
        format!("{m}+{}", self.letter)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct InitOptions {
    pub shell: ShellKind,
    pub key: Key,
    pub bind: bool,
    pub wrap: bool,
}

pub fn script(opts: InitOptions) -> String {
    let (template, key) = match opts.shell {
        ShellKind::Bash => (BASH, opts.key.bash()),
        ShellKind::Zsh => (ZSH, opts.key.zsh()),
        ShellKind::PowerShell => (POWERSHELL, opts.key.powershell()),
    };
    let mut out = String::with_capacity(template.len());
    let mut keep = true;
    for line in template.lines() {
        match line.trim() {
            "#@bind" => keep = opts.bind,
            "#@wrap" => keep = opts.wrap,
            "#@end-bind" | "#@end-wrap" => keep = true,
            _ if keep => {
                out.push_str(line);
                out.push('\n');
            }
            _ => {}
        }
    }
    out.replace("@KEY_BASH@", &key)
        .replace("@KEY_ZSH@", &key)
        .replace("@KEY_PS@", &key)
        .replace("@KEY_NAME@", &opts.key.name())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(shell: ShellKind) -> InitOptions {
        InitOptions {
            shell,
            key: Key::default(),
            bind: true,
            wrap: true,
        }
    }

    #[test]
    fn key_parsing() {
        assert_eq!(Key::parse("ctrl+g"), Ok(Key::default()));
        assert_eq!(
            Key::parse(" Alt-H "),
            Ok(Key {
                alt: true,
                letter: 'h'
            })
        );
        assert_eq!(Key::parse("ctrl+c"), Err(KeyError::Reserved));
        assert_eq!(Key::parse("ctrl+m"), Err(KeyError::Reserved));
        assert_eq!(Key::parse("g"), Err(KeyError::Format));
        assert_eq!(Key::parse("shift+g"), Err(KeyError::Format));
        assert_eq!(Key::parse("ctrl+gg"), Err(KeyError::Format));
        assert_eq!(Key::parse("ctrl+1"), Err(KeyError::Format));
    }

    #[test]
    fn default_scripts_bind_ctrl_g_and_define_wrapper() {
        let bash = script(opts(ShellKind::Bash));
        assert!(bash.contains(r#"bind -m "$__howtui_km" -x '"\C-g": __howtui_widget'"#));
        assert!(bash.contains("howtui() {"));
        assert!(bash.contains("eval \"$(howtui init bash)\""));
        let zsh = script(opts(ShellKind::Zsh));
        assert!(zsh.contains("bindkey -M viins '^G' __howtui_widget"));
        assert!(zsh.contains("print -rz --"));
        let ps = script(opts(ShellKind::PowerShell));
        assert!(ps.contains("Chord = 'Ctrl+g'"));
        assert!(ps.contains("function global:howtui {"));
        for s in [&bash, &zsh, &ps] {
            assert!(!s.contains("#@"), "구간 표식 제거");
            assert!(!s.contains("@KEY_"), "자리표시자 치환");
            assert!(s.contains("Ctrl+G"));
            assert!(s.contains("never edits"));
        }
    }

    #[test]
    fn options_remove_sections_and_change_key() {
        let alt_h = Key {
            alt: true,
            letter: 'h',
        };
        let o = |shell| InitOptions {
            shell,
            key: alt_h,
            bind: false,
            wrap: true,
        };
        let bash = script(o(ShellKind::Bash));
        assert!(!bash.contains("__howtui_widget"));
        assert!(bash.contains("howtui() {"));
        let o = |shell| InitOptions {
            shell,
            key: alt_h,
            bind: true,
            wrap: false,
        };
        let bash = script(o(ShellKind::Bash));
        assert!(bash.contains(r#"'"\eh": __howtui_widget'"#));
        assert!(!bash.contains("howtui() {"));
        assert!(!bash.contains("__howtui_prefill"));
        let zsh = script(o(ShellKind::Zsh));
        assert!(zsh.contains("bindkey -M emacs '^[h' __howtui_widget"));
        assert!(!zsh.contains("howtui() {"));
        let ps = script(o(ShellKind::PowerShell));
        assert!(ps.contains("Chord = 'Alt+h'"));
        assert!(!ps.contains("function global:howtui {"));
    }

    #[test]
    fn powershell_script_is_ascii() {
        // Windows PowerShell 5.1은 BOM 없는 스크립트를 ANSI 코드 페이지로 해석
        for (bind, wrap) in [(true, true), (false, false)] {
            let s = script(InitOptions {
                shell: ShellKind::PowerShell,
                key: Key::default(),
                bind,
                wrap,
            });
            assert!(s.is_ascii());
        }
    }
}
