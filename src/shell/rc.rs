//! rc 파일의 쉘 통합 표식 구간 추가와 제거 (CR-004, SRS-SH-015).
//!
//! 사용자가 `howtui init <쉘> --install`을 실행하거나 설치 스크립트 질문에 동의한 경우에만 호출한다.
//! 표식 구간 밖의 내용은 바꾸지 않는다.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::ShellKind;

pub const BEGIN: &str = "# >>> howtui shell integration >>>";
pub const END: &str = "# <<< howtui shell integration <<<";

#[derive(Debug, PartialEq, Eq)]
pub enum Change {
    Added,
    Updated,
    Unchanged,
    Removed,
    NotPresent,
}

/// 대상 rc 파일: Bash `~/.bashrc` (macOS는 로그인 쉘이 읽는 `~/.bash_profile`), Zsh `${ZDOTDIR:-~}/.zshrc`.
/// PowerShell은 Profile 경로가 버전과 문서 폴더 위치에 따라 달라 미지원 (None).
pub fn rc_path(shell: ShellKind, env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let home = crate::platform::paths::home(env)?;
    match shell {
        ShellKind::Bash if cfg!(target_os = "macos") => Some(home.join(".bash_profile")),
        ShellKind::Bash => Some(home.join(".bashrc")),
        ShellKind::Zsh => Some(
            env("ZDOTDIR")
                .map(PathBuf::from)
                .unwrap_or(home)
                .join(".zshrc"),
        ),
        ShellKind::PowerShell => None,
    }
}

/// 쉘 작은따옴표 문자열.
fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// 표식 구간 본문. `init_args`는 `init <쉘>` 뒤에 붙일 옵션 (앞 공백 포함).
/// `path_dir`가 있으면 PATH에 없을 때만 앞에 추가 (설치 직후 새 디렉터리가 PATH에 없는 경우).
pub fn block(shell: ShellKind, init_args: &str, path_dir: Option<&Path>) -> String {
    let name = shell.name();
    let mut out =
        format!("{BEGIN}\n# Added at your request. Remove with: howtui init {name} --uninstall\n");
    if let Some(dir) = path_dir {
        let dir = quote(&dir.display().to_string());
        out.push_str(&format!(
            "__howtui_dir={dir}; case \":$PATH:\" in *\":$__howtui_dir:\"*) ;; *) PATH=\"$__howtui_dir:$PATH\"; export PATH ;; esac; unset __howtui_dir\n"
        ));
    }
    out.push_str(&format!(
        "if command -v howtui >/dev/null 2>&1; then eval \"$(howtui init {name}{init_args})\"; fi\n{END}\n"
    ));
    out
}

/// (시작 줄, 끝 줄) 바이트 범위. 끝은 END 줄의 줄바꿈 다음.
fn find_block(text: &str) -> Option<(usize, usize)> {
    let mut offset = 0;
    let mut start = None;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_end();
        if start.is_none() && trimmed == BEGIN {
            start = Some(offset);
        } else if start.is_some() && trimmed == END {
            return Some((start?, offset + line.len()));
        }
        offset += line.len();
    }
    None
}

fn read(path: &Path) -> io::Result<String> {
    match fs::read_to_string(path) {
        Ok(t) => Ok(t),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e),
    }
}

/// 표식 구간이 없으면 끝에 추가, 다르면 교체, 같으면 그대로.
pub fn install(path: &Path, block: &str) -> io::Result<Change> {
    let text = read(path)?;
    if let Some((s, e)) = find_block(&text) {
        if &text[s..e] == block {
            return Ok(Change::Unchanged);
        }
        let updated = format!("{}{block}{}", &text[..s], &text[e..]);
        rewrite(path, &updated)?;
        return Ok(Change::Updated);
    }
    // 추가는 append: 심볼릭 링크와 권한 유지
    let mut add = String::new();
    if !text.is_empty() {
        if !text.ends_with('\n') {
            add.push('\n');
        }
        add.push('\n');
    }
    add.push_str(block);
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(add.as_bytes())?;
    file.flush()?;
    Ok(Change::Added)
}

/// 표식 구간과 바로 앞의 빈 줄 1개(추가 시 넣은 줄) 제거.
pub fn uninstall(path: &Path) -> io::Result<Change> {
    let text = read(path)?;
    let Some((mut s, e)) = find_block(&text) else {
        return Ok(Change::NotPresent);
    };
    if text[..s].ends_with("\n\n") {
        s -= 1;
    }
    let updated = format!("{}{}", &text[..s], &text[e..]);
    rewrite(path, &updated)?;
    Ok(Change::Removed)
}

/// 심볼릭 링크 대상 경로에 임시 파일 기록 후 이름 변경, 기존 권한 유지.
fn rewrite(path: &Path, text: &str) -> io::Result<()> {
    let target = fs::canonicalize(path)?;
    let dir = target
        .parent()
        .ok_or_else(|| io::Error::other("rc file has no parent directory"))?;
    let name = target.file_name().and_then(|n| n.to_str()).unwrap_or("rc");
    let tmp = dir.join(format!(".{name}.howtui-{}", std::process::id()));
    let result = (|| {
        let perms = fs::metadata(&target)?.permissions();
        let mut file = fs::File::create(&tmp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::set_permissions(&tmp, perms)?;
        fs::rename(&tmp, &target)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bash_block(args: &str) -> String {
        block(ShellKind::Bash, args, None)
    }

    #[test]
    fn block_content() {
        let b = block(
            ShellKind::Zsh,
            " --key alt+h",
            Some(Path::new("/home/a b/.local/bin")),
        );
        assert!(b.starts_with(BEGIN));
        assert!(b.ends_with(&format!("{END}\n")));
        assert!(b.contains(r#"eval "$(howtui init zsh --key alt+h)""#));
        assert!(b.contains("howtui init zsh --uninstall"));
        assert!(b.contains("__howtui_dir='/home/a b/.local/bin'"));
        assert!(block(ShellKind::Bash, "", Some(Path::new("/x/it's"))).contains(r"'/x/it'\''s'"));
        assert!(!bash_block("").contains("PATH"));
    }

    #[test]
    fn install_appends_once_and_updates_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let rc = dir.path().join(".bashrc");
        fs::write(&rc, "export A=1").unwrap();
        assert_eq!(install(&rc, &bash_block("")).unwrap(), Change::Added);
        assert_eq!(
            fs::read_to_string(&rc).unwrap(),
            format!("export A=1\n\n{}", bash_block(""))
        );
        assert_eq!(install(&rc, &bash_block("")).unwrap(), Change::Unchanged);
        fs::write(
            &rc,
            format!("{}# after\n", fs::read_to_string(&rc).unwrap()),
        )
        .unwrap();
        assert_eq!(
            install(&rc, &bash_block(" --no-wrap")).unwrap(),
            Change::Updated
        );
        let text = fs::read_to_string(&rc).unwrap();
        assert_eq!(text.matches(BEGIN).count(), 1);
        assert!(text.contains("init bash --no-wrap"));
        assert!(text.starts_with("export A=1\n\n") && text.ends_with("# after\n"));
    }

    #[test]
    fn uninstall_restores_original_text() {
        let dir = tempfile::tempdir().unwrap();
        let rc = dir.path().join(".zshrc");
        assert_eq!(uninstall(&rc).unwrap(), Change::NotPresent);
        assert_eq!(install(&rc, &bash_block("")).unwrap(), Change::Added);
        assert_eq!(fs::read_to_string(&rc).unwrap(), bash_block(""));
        assert_eq!(uninstall(&rc).unwrap(), Change::Removed);
        assert_eq!(fs::read_to_string(&rc).unwrap(), "");
        let original = "alias ll='ls -l'\n# end\n";
        fs::write(&rc, original).unwrap();
        install(&rc, &bash_block("")).unwrap();
        assert_eq!(uninstall(&rc).unwrap(), Change::Removed);
        assert_eq!(fs::read_to_string(&rc).unwrap(), original);
    }

    #[cfg(unix)]
    #[test]
    fn rewrite_keeps_symlink_and_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("dotfiles-bashrc");
        let link = dir.path().join(".bashrc");
        fs::write(&real, "x\n").unwrap();
        fs::set_permissions(&real, fs::Permissions::from_mode(0o640)).unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        install(&link, &bash_block("")).unwrap();
        install(&link, &bash_block(" --no-bind")).unwrap();
        uninstall(&link).unwrap();
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_to_string(&real).unwrap(), "x\n");
        assert_eq!(
            fs::metadata(&real).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[test]
    fn rc_paths() {
        let env = |k: &str| match k {
            "HOME" | "USERPROFILE" => Some("/h".to_string()),
            "ZDOTDIR" => Some("/z".to_string()),
            _ => None,
        };
        assert_eq!(
            rc_path(ShellKind::Zsh, &env),
            Some(PathBuf::from("/z/.zshrc"))
        );
        assert!(rc_path(ShellKind::Bash, &env).unwrap().starts_with("/h"));
        assert_eq!(rc_path(ShellKind::PowerShell, &env), None);
    }
}
