//! 현재 폴더 목록 (CR-007, CR-008, SRS-CTX-001). 사용자가 켠 경우에만 요청 직전에 읽는다.
//!
//! `ls -al`과 같은 정보(종류와 권한, 크기, 수정 시각, 이름, 숨김 항목 포함)를 1단계 항목만 만든다.
//! 파일 내용, 소유자와 그룹(계정 이름), 링크 대상(절대 경로), 폴더 경로, 하위 폴더 내용은 넣지 않는다.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::platform::time::format_utc_minutes;

/// 최대 항목 수와 전체 길이 (프롬프트 크기 상한)
const MAX_ENTRIES: usize = 150;
const MAX_CHARS: usize = 9000;
const MAX_NAME: usize = 100;

struct Entry {
    name: String,
    kind: Kind,
    mode: String,
    size: u64,
    modified: Option<u64>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Dir,
    File,
    Link,
    Other,
}

/// 이름의 제어 문자는 `?`로 바꾸고 길이를 제한 (줄바꿈 이름으로 목록 형식이 깨지지 않도록).
fn clean(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| if c.is_control() { '?' } else { c })
        .take(MAX_NAME)
        .collect();
    if name.chars().count() > MAX_NAME {
        out.push('…');
    }
    out
}

/// `ls -h` 형식 크기 (1024 단위)
fn human(size: u64) -> String {
    const UNITS: [&str; 5] = ["", "K", "M", "G", "T"];
    let mut v = size as f64;
    let mut unit = 0;
    while v >= 1024.0 && unit + 1 < UNITS.len() {
        v /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        size.to_string()
    } else {
        format!("{v:.1}{}", UNITS[unit])
    }
}

fn type_char(kind: &Kind) -> char {
    match kind {
        Kind::Dir => 'd',
        Kind::File => '-',
        Kind::Link => 'l',
        Kind::Other => '?',
    }
}

/// 종류와 권한 10자 (Unix `drwxr-xr-x`). Windows는 읽기 전용 여부만 (`-r--r--r--`, `-rw-rw-rw-`).
fn mode_string(kind: &Kind, meta: &fs::Metadata) -> String {
    let mut s = String::with_capacity(10);
    s.push(type_char(kind));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let m = meta.permissions().mode();
        for shift in [6, 3, 0] {
            let bits = (m >> shift) & 0o7;
            s.push(if bits & 4 != 0 { 'r' } else { '-' });
            s.push(if bits & 2 != 0 { 'w' } else { '-' });
            s.push(if bits & 1 != 0 { 'x' } else { '-' });
        }
    }
    #[cfg(not(unix))]
    {
        let rw = if meta.permissions().readonly() {
            "r--"
        } else {
            "rw-"
        };
        for _ in 0..3 {
            s.push_str(rw);
        }
    }
    s
}

fn modified_secs(meta: &fs::Metadata) -> Option<u64> {
    meta.modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

/// 폴더 목록 텍스트. 읽기 실패 시 원인 1줄.
pub fn folder_listing(dir: &Path) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    folder_listing_at(dir, now)
}

fn folder_listing_at(dir: &Path, now: u64) -> String {
    let read = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(e) => return format!("(cannot read the current folder: {})", e.kind()),
    };
    let mut entries = Vec::new();
    for item in read.flatten() {
        let raw = item.file_name().to_string_lossy().into_owned();
        // 심볼릭 링크는 따라가지 않음 (DirEntry::metadata는 링크 자체 정보)
        let Ok(meta) = item.metadata() else {
            entries.push(Entry {
                name: clean(&raw),
                kind: Kind::Other,
                mode: "?---------".into(),
                size: 0,
                modified: None,
            });
            continue;
        };
        let kind = if meta.file_type().is_symlink() {
            Kind::Link
        } else if meta.is_dir() {
            Kind::Dir
        } else if meta.is_file() {
            Kind::File
        } else {
            Kind::Other
        };
        entries.push(Entry {
            name: clean(&raw),
            mode: mode_string(&kind, &meta),
            size: if kind == Kind::File { meta.len() } else { 0 },
            modified: modified_secs(&meta),
            kind,
        });
    }
    entries.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    let total = entries.len();
    let mut out = format!(
        "listed {} UTC; mode, size, modified (UTC), name; owners and link targets omitted\n",
        format_utc_minutes(now)
    );
    let mut shown = 0;
    for e in &entries {
        let size = if e.kind == Kind::File {
            human(e.size)
        } else {
            "-".into()
        };
        let time = e
            .modified
            .map(format_utc_minutes)
            .unwrap_or_else(|| "-".into());
        let suffix = match e.kind {
            Kind::Dir => "/",
            Kind::Link => "@",
            _ => "",
        };
        let line = format!("{} {size:>7} {time} {}{suffix}", e.mode, e.name);
        if shown >= MAX_ENTRIES || out.len() + line.len() + 1 > MAX_CHARS {
            break;
        }
        out.push_str(&line);
        out.push('\n');
        shown += 1;
    }
    if total == 0 {
        out.push_str("(empty folder)\n");
    }
    if shown < total {
        out.push_str(&format!("(+{} more entries not listed)\n", total - shown));
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ls_al_like_listing_without_contents_owners_or_link_targets() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        fs::write(d.join("backup.tar.gz"), vec![0u8; 2048]).unwrap();
        fs::write(d.join("Notes.txt"), "secret content").unwrap();
        fs::write(d.join(".env"), "TOKEN=abc").unwrap();
        fs::create_dir(d.join("src")).unwrap();
        fs::write(d.join("src").join("inner.rs"), "x").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::os::unix::fs::symlink("/etc/passwd", d.join("link")).unwrap();
            fs::set_permissions(d.join("Notes.txt"), fs::Permissions::from_mode(0o640)).unwrap();
        }
        let text = folder_listing_at(d, 1_792_253_674);
        let lines: Vec<&str> = text.lines().collect();
        assert!(
            lines[0].starts_with("listed 2026-10-17 16:14 UTC;"),
            "{text}"
        );
        assert!(
            lines[1].starts_with('d') && lines[1].ends_with(" src/"),
            "폴더 먼저: {text}"
        );
        let line = |name: &str| {
            lines
                .iter()
                .find(|l| l.ends_with(name))
                .copied()
                .unwrap_or("")
        };
        assert!(
            line(" .env").contains("      9 "),
            "숨김 항목 포함 (이름과 크기): {text}"
        );
        assert!(line(" backup.tar.gz").contains("   2.0K "), "{text}");
        #[cfg(unix)]
        {
            assert!(
                line(" Notes.txt").starts_with("-rw-r-----      14 "),
                "{text}"
            );
            assert!(line(" link@").starts_with('l'), "{text}");
        }
        for absent in ["secret content", "TOKEN", "inner.rs", "passwd"] {
            assert!(!text.contains(absent), "{absent}");
        }
        assert!(!text.contains(&d.display().to_string()), "절대 경로 없음");
        // 소유자 이름 미포함
        if let Some(user) = std::env::var_os("USER").or_else(|| std::env::var_os("USERNAME")) {
            let user = user.to_string_lossy().to_string();
            if user.len() > 2 {
                assert!(!text.contains(&user), "owner name");
            }
        }
    }

    #[test]
    fn caps_entries_and_cleans_names() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..(MAX_ENTRIES + 5) {
            fs::write(dir.path().join(format!("f{i:03}")), "").unwrap();
        }
        let text = folder_listing(dir.path());
        assert!(text.contains("(+5 more entries not listed)"));
        assert!(text.len() <= MAX_CHARS + 100);
        assert_eq!(clean("a\nb\tc"), "a?b?c");
        assert!(clean(&"x".repeat(300)).ends_with('…'));
        let empty = tempfile::tempdir().unwrap();
        assert!(folder_listing(empty.path()).ends_with("(empty folder)"));
        assert!(folder_listing(&empty.path().join("missing")).starts_with("(cannot read"));
        assert_eq!(human(1023), "1023");
        assert_eq!(human(50_000_000), "47.7M");
    }
}
