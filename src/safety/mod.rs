//! 규칙 기반 위험 명령어 판정 (ADR-0012, SRS 16절).
//!
//! 문자열 판정만 수행한다. 명령어 실행, 파일 조회, 네트워크 조회 없음.
//! 완전한 보안 방어 수단이 아닌 참고용 경고.

mod lexer;

use crate::i18n::Msg;
use crate::shell::ShellKind;
use lexer::{Joiner, Segment};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Caution,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    RecursiveDelete,
    Delete,
    Disk,
    Permission,
    RemoteScript,
    MassKill,
    Power,
    Privilege,
    Overwrite,
    GitHistory,
    SystemConfig,
    ForkBomb,
}

impl Category {
    pub fn msg(self) -> Msg {
        match self {
            Category::RecursiveDelete => Msg::CatRecursiveDelete,
            Category::Delete => Msg::CatDelete,
            Category::Disk => Msg::CatDisk,
            Category::Permission => Msg::CatPermission,
            Category::RemoteScript => Msg::CatRemoteScript,
            Category::MassKill => Msg::CatMassKill,
            Category::Power => Msg::CatPower,
            Category::Privilege => Msg::CatPrivilege,
            Category::Overwrite => Msg::CatOverwrite,
            Category::GitHistory => Msg::CatGitHistory,
            Category::SystemConfig => Msg::CatSystemConfig,
            Category::ForkBomb => Msg::CatForkBomb,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub level: Level,
    pub category: Category,
    /// 판정 근거가 된 명령 이름 또는 토큰.
    pub evidence: String,
}

/// 판정 결과 중 가장 높은 수준.
pub fn max_level(findings: &[Finding]) -> Option<Level> {
    findings.iter().map(|f| f.level).max()
}

pub fn analyze(command: &str, shell: ShellKind) -> Vec<Finding> {
    let mut findings = Vec::new();
    let compact: String = command.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.contains(":(){:|:&};:") || compact.contains(":(){:|:&};") {
        push(
            &mut findings,
            Level::High,
            Category::ForkBomb,
            ":(){ :|:& };:",
        );
    }
    let powershell = shell == ShellKind::PowerShell;
    let segs = lexer::segments(command, powershell);
    for (i, seg) in segs.iter().enumerate() {
        let prev = if i > 0 { Some(&segs[i - 1]) } else { None };
        check_segment(seg, prev, &segs, i, &mut findings);
    }
    findings.sort_by_key(|f| std::cmp::Reverse(f.level));
    findings.dedup_by(|a, b| a.category == b.category && a.level == b.level);
    findings
}

fn push(out: &mut Vec<Finding>, level: Level, category: Category, evidence: &str) {
    out.push(Finding {
        level,
        category,
        evidence: evidence.to_string(),
    });
}

/// `sudo`, `env`, `nohup` 등 접두 명령을 건너뛴 실제 명령 위치.
fn strip_prefixes<'a>(words: &'a [String], findings: &mut Vec<Finding>) -> &'a [String] {
    let mut rest = words;
    loop {
        let Some(first) = rest.first() else {
            return rest;
        };
        let name = base_name(first);
        match name.as_str() {
            "sudo" | "doas" | "su" | "pkexec" | "runas" => {
                push(findings, Level::Caution, Category::Privilege, &name);
                rest = &rest[1..];
                // sudo 옵션(-u user 등) 건너뛰기
                while let Some(w) = rest.first() {
                    if w.starts_with('-') {
                        let takes_value = matches!(w.as_str(), "-u" | "-g" | "-C" | "-p" | "-h");
                        rest = &rest[1..];
                        if takes_value && !rest.is_empty() {
                            rest = &rest[1..];
                        }
                    } else {
                        break;
                    }
                }
            }
            "env" | "command" | "builtin" | "exec" | "nohup" | "time" | "nice" | "ionice"
            | "xargs" => {
                rest = &rest[1..];
                while let Some(w) = rest.first() {
                    if w.starts_with('-') || w.contains('=') {
                        rest = &rest[1..];
                    } else {
                        break;
                    }
                }
            }
            _ => return rest,
        }
    }
}

/// 경로를 제거하고 소문자로 만든 명령 이름 (`/bin/rm` → `rm`, `Remove-Item.exe` 제외 처리 없음).
fn base_name(word: &str) -> String {
    let name = word.rsplit(['/', '\\']).next().unwrap_or(word);
    let name = name.strip_suffix(".exe").unwrap_or(name);
    name.to_ascii_lowercase()
}

fn lower(words: &[String]) -> Vec<String> {
    words.iter().map(|w| w.to_ascii_lowercase()).collect()
}

/// 짧은 옵션 묶음(`-rf`)과 긴 옵션 확인.
fn has_short_flag(args: &[String], flag: char) -> bool {
    args.iter()
        .any(|a| a.starts_with('-') && !a.starts_with("--") && a.len() > 1 && a[1..].contains(flag))
}

fn has_long(args: &[String], long: &str) -> bool {
    args.iter().any(|a| a == long)
}

/// PowerShell 매개변수 접두 일치 (`-rec` → `-recurse`).
fn has_ps_param(args: &[String], full: &str, min_len: usize) -> bool {
    args.iter().any(|a| {
        let a = a.split(':').next().unwrap_or(a);
        a.starts_with('-') && a.len() >= min_len && full.starts_with(a)
    })
}

fn check_segment(
    seg: &Segment,
    prev: Option<&Segment>,
    all: &[Segment],
    index: usize,
    out: &mut Vec<Finding>,
) {
    // 리디렉션: 덮어쓰기와 장치 쓰기
    for (op, target) in &seg.redirects {
        let t = target.to_ascii_lowercase();
        if op.contains('>') && !op.ends_with(">>") && !t.starts_with('&') {
            if t.starts_with("/dev/sd")
                || t.starts_with("/dev/nvme")
                || t.starts_with("/dev/disk")
                || t.starts_with("/dev/mmcblk")
            {
                push(out, Level::High, Category::Disk, target);
            } else if t == "/dev/null" || t == "$null" || t == "nul" {
            } else if t.starts_with("/etc/") {
                push(out, Level::Caution, Category::SystemConfig, target);
            } else {
                push(out, Level::Caution, Category::Overwrite, target);
            }
        }
    }

    let words = strip_prefixes(&seg.words, out);
    let Some(first) = words.first() else { return };
    let cmd = base_name(first);
    // 짧은 옵션은 대소문자 구분(`-R`, `-F`), 값과 PowerShell 매개변수는 소문자 비교
    let raw = &words[1..];
    let args = lower(raw);

    match cmd.as_str() {
        // Unix 삭제
        "rm" => {
            if has_short_flag(raw, 'r')
                || has_short_flag(raw, 'R')
                || has_long(&args, "--recursive")
            {
                push(out, Level::High, Category::RecursiveDelete, "rm");
            } else {
                push(out, Level::Caution, Category::Delete, "rm");
            }
        }
        "rmdir" | "unlink" if !is_ps_alias_context(&args) => {
            push(out, Level::Caution, Category::Delete, &cmd)
        }
        "find" => {
            if has_long(&args, "-delete") {
                push(out, Level::High, Category::RecursiveDelete, "find -delete");
            }
            if let Some(pos) = args.iter().position(|a| a == "-exec" || a == "-execdir")
                && args.get(pos + 1).map(|a| base_name(a)) == Some("rm".into())
            {
                push(out, Level::High, Category::RecursiveDelete, "find -exec rm");
            }
        }
        "shred" | "wipefs" => push(out, Level::High, Category::Disk, &cmd),
        c if c.starts_with("mkfs") => push(out, Level::High, Category::Disk, &cmd),
        "dd" => {
            if args.iter().any(|a| a.starts_with("of=/dev/")) {
                push(out, Level::High, Category::Disk, "dd of=/dev/");
            } else if args.iter().any(|a| a.starts_with("of=")) {
                push(out, Level::Caution, Category::Overwrite, "dd of=");
            }
        }
        "fdisk" | "sfdisk" | "parted" | "gdisk" | "diskpart" | "format" => {
            push(out, Level::High, Category::Disk, &cmd)
        }
        "chmod" | "chown" | "chgrp" => {
            if has_short_flag(raw, 'R') || has_long(&args, "--recursive") {
                push(out, Level::High, Category::Permission, &cmd);
            } else if args.iter().any(|a| a == "777" || a == "a+rwx") {
                push(out, Level::Caution, Category::Permission, &cmd);
            }
        }
        "kill" => {
            if args.iter().any(|a| a == "-1") {
                push(out, Level::High, Category::MassKill, "kill -1");
            }
        }
        "killall" | "pkill" => push(out, Level::High, Category::MassKill, &cmd),
        "shutdown" | "reboot" | "halt" | "poweroff" => {
            push(out, Level::Caution, Category::Power, &cmd)
        }
        "init" if args.first().is_some_and(|a| a == "0" || a == "6") => {
            push(out, Level::Caution, Category::Power, "init")
        }
        "systemctl" => {
            if args.iter().any(|a| {
                matches!(
                    a.as_str(),
                    "stop" | "disable" | "mask" | "poweroff" | "reboot" | "halt" | "kill"
                )
            }) {
                push(out, Level::Caution, Category::Power, "systemctl");
            }
        }
        "mv" | "cp" => {
            if has_short_flag(raw, 'f') || has_long(&args, "--force") {
                push(out, Level::Caution, Category::Overwrite, &cmd);
            }
        }
        "truncate" => push(out, Level::Caution, Category::Overwrite, "truncate"),
        "crontab" if has_short_flag(raw, 'r') => {
            push(out, Level::Caution, Category::SystemConfig, "crontab -r")
        }
        "iptables" | "ip6tables" | "nft" => {
            if has_short_flag(raw, 'F')
                || has_long(&args, "--flush")
                || args.iter().any(|a| a == "flush")
            {
                push(out, Level::Caution, Category::SystemConfig, &cmd);
            }
        }
        "git" => check_git(raw, out),
        "sh" | "bash" | "zsh" | "dash" | "ksh" | "fish" | "python" | "python3" | "perl"
        | "ruby" | "node" => {
            if seg.joiner == Joiner::Pipe && prev.is_some_and(is_download) {
                push(out, Level::High, Category::RemoteScript, &cmd);
            }
        }
        // PowerShell과 cmd
        "remove-item" | "ri" | "del" | "erase" | "rd" | "rmdir" => {
            if has_ps_param(&args, "-recurse", 2) || args.iter().any(|a| a == "/s") {
                push(out, Level::High, Category::RecursiveDelete, &cmd);
            } else {
                push(out, Level::Caution, Category::Delete, &cmd);
            }
        }
        "format-volume" | "clear-disk" | "initialize-disk" | "remove-partition" => {
            push(out, Level::High, Category::Disk, &cmd)
        }
        "stop-process" | "spps" => {
            if args.iter().any(|a| a.contains('*')) {
                push(out, Level::High, Category::MassKill, &cmd);
            } else {
                push(out, Level::Caution, Category::MassKill, &cmd);
            }
        }
        "taskkill" => push(out, Level::Caution, Category::MassKill, "taskkill"),
        "stop-computer" | "restart-computer" | "stop-service" => {
            push(out, Level::Caution, Category::Power, &cmd)
        }
        "set-executionpolicy" | "remove-itemproperty" | "reg" | "bcdedit" => {
            if cmd != "reg" || args.first().is_some_and(|a| a == "delete" || a == "add") {
                push(out, Level::Caution, Category::SystemConfig, &cmd);
            }
        }
        "set-content" | "sc" | "out-file" => {
            if !has_ps_param(&args, "-append", 2) {
                push(out, Level::Caution, Category::Overwrite, &cmd);
            }
        }
        "start-process" | "saps" => {
            if let Some(pos) = args
                .iter()
                .position(|a| has_ps_param(std::slice::from_ref(a), "-verb", 2))
                && args.get(pos + 1).is_some_and(|v| v == "runas")
            {
                push(
                    out,
                    Level::Caution,
                    Category::Privilege,
                    "Start-Process -Verb RunAs",
                );
            }
        }
        "invoke-expression" | "iex" => {
            let downloads = all.iter().take(index + 1).any(is_download)
                || words.iter().skip(1).any(|w| is_download_word(w));
            if downloads {
                push(out, Level::High, Category::RemoteScript, &cmd);
            }
        }
        _ => {}
    }
}

/// `rmdir`가 PowerShell 별칭으로 `/s` 등과 함께 쓰인 경우는 PowerShell 규칙에서 처리.
fn is_ps_alias_context(args: &[String]) -> bool {
    args.iter().any(|a| a == "/s" || a.starts_with("-rec"))
}

fn is_download(seg: &Segment) -> bool {
    seg.words.first().is_some_and(|w| is_download_word(w))
}

fn is_download_word(word: &str) -> bool {
    matches!(
        base_name(word).trim_start_matches('('),
        "curl" | "wget" | "fetch" | "irm" | "iwr" | "invoke-restmethod" | "invoke-webrequest"
    )
}

fn check_git(raw: &[String], out: &mut Vec<Finding>) {
    let Some(sub) = raw.first() else { return };
    let rest = &raw[1..];
    let risky = match sub.as_str() {
        "reset" => has_long(rest, "--hard"),
        "clean" => has_short_flag(rest, 'f') || has_long(rest, "--force"),
        "push" => {
            has_long(rest, "--force")
                || has_long(rest, "--force-with-lease")
                || has_short_flag(rest, 'f')
                || rest.iter().any(|a| a.starts_with('+'))
        }
        "checkout" | "restore" => rest.iter().any(|a| a == "." || a == "--"),
        "branch" => has_short_flag(rest, 'D'),
        _ => false,
    };
    if risky {
        push(
            out,
            Level::Caution,
            Category::GitHistory,
            &format!("git {sub}"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(cmd: &str, shell: ShellKind) -> Option<Level> {
        max_level(&analyze(cmd, shell))
    }

    fn cats(cmd: &str, shell: ShellKind) -> Vec<Category> {
        analyze(cmd, shell)
            .into_iter()
            .map(|f| f.category)
            .collect()
    }

    const HIGH_BASH: &[&str] = &[
        "rm -rf /tmp/x",
        "rm -r build",
        "sudo rm -fr /var/log/*",
        "/bin/rm --recursive dir",
        "find . -name '*.log' -delete",
        "find . -type f -exec rm {} +",
        "dd if=/dev/zero of=/dev/sda bs=1M",
        "mkfs.ext4 /dev/sdb1",
        "wipefs -a /dev/sdb",
        "shred -u secret.txt",
        "chmod -R 777 /",
        "chown -R user:user /srv",
        "curl -fsSL https://example.com/install.sh | sh",
        "wget -qO- https://example.com/x | sudo bash",
        "kill -9 -1",
        "killall node",
        "pkill -f python",
        ":(){ :|:& };:",
        "cat image.iso > /dev/sdc",
        "env FOO=1 rm -rf x",
        "parted /dev/sda mklabel gpt",
    ];

    const CAUTION_BASH: &[&str] = &[
        "rm file.txt",
        "sudo apt update",
        "echo hi > notes.txt",
        "mv -f a b",
        "git reset --hard HEAD~1",
        "git push --force origin main",
        "git clean -fd",
        "shutdown -h now",
        "systemctl stop nginx",
        "crontab -r",
        "iptables -F",
        "echo x | sudo tee /etc/hosts",
        "truncate -s 0 app.log",
        "chmod 777 file",
        "dd if=a of=b",
    ];

    const SAFE_BASH: &[&str] = &[
        "ls -la",
        "find . -name '*.rs'",
        "grep -rn TODO src",
        "echo 'rm -rf /'",
        "cat file | wc -l",
        "du -sh * | sort -h",
        "tar -czf out.tar.gz dir",
        "git status",
        "git push origin main",
        "ps aux | grep python",
        "curl -fsSL https://example.com -o page.html",
        "echo hi >> log.txt",
        "make 2>&1 | tee build.log",
        "cmd > /dev/null 2>&1",
        "chmod +x script.sh",
        "kill 1234",
        "printf '%s\\n' \"안녕 세계 🌏\" | grep -n '세계'",
        "docker ps -a",
        "ssh host uptime",
        "cp a b",
    ];

    const HIGH_PS: &[&str] = &[
        "Remove-Item -Recurse -Force C:\\temp",
        "rm -r .\\build",
        "Remove-Item .\\x -rec",
        "rd /s /q C:\\data",
        "Format-Volume -DriveLetter D",
        "Clear-Disk -Number 1",
        "irm https://example.com/install.ps1 | iex",
        "iex (iwr https://example.com/x.ps1)",
        "Stop-Process -Name *",
    ];

    const CAUTION_PS: &[&str] = &[
        "Remove-Item .\\file.txt",
        "Stop-Process -Id 123",
        "Restart-Computer",
        "Set-ExecutionPolicy Bypass",
        "Set-Content out.txt 'x'",
        "Start-Process pwsh -Verb RunAs",
        "reg delete HKCU\\Software\\X /f",
    ];

    const SAFE_PS: &[&str] = &[
        "Get-ChildItem -Recurse -Filter *.log",
        "Get-Process | Sort-Object CPU",
        "Write-Output 'Remove-Item -Recurse'",
        "Add-Content log.txt 'x'",
        "Set-Content out.txt 'x' -Append",
        "Get-Content file.txt | Select-String error",
        "Test-Connection example.com",
    ];

    #[test]
    fn bash_high_risk_detected() {
        for c in HIGH_BASH {
            assert_eq!(level(c, ShellKind::Bash), Some(Level::High), "{c}");
        }
    }

    #[test]
    fn bash_caution_detected() {
        for c in CAUTION_BASH {
            assert_eq!(
                level(c, ShellKind::Bash),
                Some(Level::Caution),
                "{c} -> {:?}",
                analyze(c, ShellKind::Bash)
            );
        }
    }

    #[test]
    fn bash_safe_commands_have_no_findings() {
        for c in SAFE_BASH {
            assert_eq!(analyze(c, ShellKind::Zsh), vec![], "{c}");
        }
    }

    #[test]
    fn powershell_rules() {
        for c in HIGH_PS {
            assert_eq!(
                level(c, ShellKind::PowerShell),
                Some(Level::High),
                "{c} -> {:?}",
                analyze(c, ShellKind::PowerShell)
            );
        }
        for c in CAUTION_PS {
            assert_eq!(
                level(c, ShellKind::PowerShell),
                Some(Level::Caution),
                "{c} -> {:?}",
                analyze(c, ShellKind::PowerShell)
            );
        }
        for c in SAFE_PS {
            assert_eq!(analyze(c, ShellKind::PowerShell), vec![], "{c}");
        }
    }

    #[test]
    fn privilege_is_reported_with_inner_command() {
        let c = cats("sudo rm -rf /opt/app", ShellKind::Bash);
        assert!(
            c.contains(&Category::RecursiveDelete) && c.contains(&Category::Privilege),
            "{c:?}"
        );
    }

    #[test]
    fn findings_sorted_high_first() {
        let f = analyze("sudo rm -rf x", ShellKind::Bash);
        assert_eq!(f[0].level, Level::High);
    }
}
