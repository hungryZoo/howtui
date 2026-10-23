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

/// 대상 파일: Bash `~/.bashrc` (macOS는 로그인 쉘이 읽는 `~/.bash_profile`), Zsh `${ZDOTDIR:-~}/.zshrc`,
/// PowerShell 현재 사용자 Profile (Windows는 PowerShell 7과 Windows PowerShell 5.1 둘 다, 문서 폴더 기준).
pub fn rc_paths(shell: ShellKind, env: &dyn Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    let Some(home) = crate::platform::paths::home(env) else {
        return Vec::new();
    };
    match shell {
        ShellKind::Bash if cfg!(target_os = "macos") => vec![home.join(".bash_profile")],
        ShellKind::Bash => vec![home.join(".bashrc")],
        ShellKind::Zsh => vec![
            env("ZDOTDIR")
                .map(PathBuf::from)
                .unwrap_or(home)
                .join(".zshrc"),
        ],
        ShellKind::PowerShell => powershell_profiles(&home, env),
    }
}

const PS_PROFILE: &str = "Microsoft.PowerShell_profile.ps1";

fn powershell_profiles(home: &Path, env: &dyn Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    // 설치 시험용 경로 (실제 사용자 Profile 보호)
    if let Some(p) = env("HOWTUI_TEST_PS_PROFILE") {
        return vec![PathBuf::from(p)];
    }
    #[cfg(windows)]
    {
        let docs = documents_dir().unwrap_or_else(|| home.join("Documents"));
        vec![
            docs.join("PowerShell").join(PS_PROFILE),
            docs.join("WindowsPowerShell").join(PS_PROFILE),
        ]
    }
    #[cfg(not(windows))]
    {
        let config = env("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        vec![config.join("powershell").join(PS_PROFILE)]
    }
}

/// 문서 폴더 (OneDrive 등으로 옮겨진 경우 포함).
#[cfg(windows)]
fn documents_dir() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::{FOLDERID_Documents, SHGetKnownFolderPath};
    let mut ptr: *mut u16 = std::ptr::null_mut();
    // SAFETY: 성공과 실패 모두 반환 버퍼는 CoTaskMemFree로 해제 (API 규약)
    let hr =
        unsafe { SHGetKnownFolderPath(&FOLDERID_Documents, 0, std::ptr::null_mut(), &mut ptr) };
    let result = if hr >= 0 && !ptr.is_null() {
        // SAFETY: 성공 시 NUL 종료 UTF-16 문자열
        let len = (0..).take_while(|&i| unsafe { *ptr.add(i) } != 0).count();
        let wide = unsafe { std::slice::from_raw_parts(ptr, len) };
        Some(PathBuf::from(std::ffi::OsString::from_wide(wide)))
    } else {
        None
    };
    unsafe { CoTaskMemFree(ptr as *const core::ffi::c_void) };
    result
}

/// 쉘 작은따옴표 문자열.
fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// 표식 구간 본문. `init_args`는 `init <쉘>` 뒤에 붙일 옵션 (앞 공백 포함).
/// `path_dir`가 있으면 PATH에 없을 때만 앞에 추가 (설치 직후 새 디렉터리가 PATH에 없는 경우).
/// PowerShell은 설치 스크립트가 사용자 PATH를 등록하므로 PATH 줄 없음 (ASCII 전용, 5.1 호환).
pub fn block(shell: ShellKind, init_args: &str, path_dir: Option<&Path>) -> String {
    let name = shell.name();
    let mut out =
        format!("{BEGIN}\n# Added at your request. Remove with: howtui init {name} --uninstall\n");
    if shell == ShellKind::PowerShell {
        out.push_str(&format!(
            "if (Get-Command -Name howtui -CommandType Application -ErrorAction SilentlyContinue) {{ howtui init powershell{init_args} | Out-String | Invoke-Expression }}\n{END}\n"
        ));
        return out;
    }
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
    if let Some(dir) = path.parent()
        && !dir.as_os_str().is_empty()
    {
        // PowerShell Profile 폴더는 처음에 없을 수 있음
        fs::create_dir_all(dir)?;
    }
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
    fn rc_path_rules() {
        let env = |k: &str| match k {
            "HOME" | "USERPROFILE" => Some("/h".to_string()),
            "ZDOTDIR" => Some("/z".to_string()),
            _ => None,
        };
        assert_eq!(
            rc_paths(ShellKind::Zsh, &env),
            vec![PathBuf::from("/z/.zshrc")]
        );
        assert!(rc_paths(ShellKind::Bash, &env)[0].starts_with("/h"));
        let ps = rc_paths(ShellKind::PowerShell, &env);
        assert!(!ps.is_empty());
        assert!(ps.iter().all(|p| p.ends_with(PS_PROFILE)));
        #[cfg(windows)]
        assert_eq!(ps.len(), 2, "PowerShell 7과 5.1");
        #[cfg(not(windows))]
        assert_eq!(
            ps,
            vec![PathBuf::from("/h/.config/powershell").join(PS_PROFILE)]
        );
        let test_env = |k: &str| (k == "HOWTUI_TEST_PS_PROFILE").then(|| "/t/p.ps1".to_string());
        assert_eq!(
            rc_paths(ShellKind::PowerShell, &test_env),
            vec![PathBuf::from("/t/p.ps1")]
        );
    }

    #[test]
    fn powershell_block_is_ascii_and_creates_profile_dir() {
        let b = block(ShellKind::PowerShell, " --key alt+h", None);
        assert!(b.is_ascii());
        assert!(b.contains("howtui init powershell --key alt+h | Out-String | Invoke-Expression"));
        let dir = tempfile::tempdir().unwrap();
        let profile = dir
            .path()
            .join("Documents")
            .join("PowerShell")
            .join(PS_PROFILE);
        assert_eq!(install(&profile, &b).unwrap(), Change::Added);
        assert_eq!(uninstall(&profile).unwrap(), Change::Removed);
        assert_eq!(fs::read_to_string(&profile).unwrap(), "");
    }
}
