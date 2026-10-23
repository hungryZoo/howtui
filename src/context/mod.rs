//! 현재 폴더 목록 (CR-007, SRS-CTX-001). 사용자가 켠 경우에만 요청 직전에 읽는다.
//!
//! 1단계 항목의 이름, 종류, 크기만 포함한다. 파일 내용, 숨김 항목, 절대 경로, 하위 폴더 내용은 읽지 않는다.

use std::fs;
use std::path::Path;

/// 최대 항목 수와 전체 길이 (프롬프트 크기 상한)
const MAX_ENTRIES: usize = 150;
const MAX_CHARS: usize = 6000;
const MAX_NAME: usize = 100;

struct Entry {
    name: String,
    kind: Kind,
    size: u64,
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

fn human(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = size as f64;
    let mut unit = 0;
    while v >= 1024.0 && unit + 1 < UNITS.len() {
        v /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{size} B")
    } else {
        format!("{v:.1} {}", UNITS[unit])
    }
}

/// 폴더 목록 텍스트. 읽기 실패 시 원인 1줄.
pub fn folder_listing(dir: &Path) -> String {
    let read = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(e) => return format!("(cannot read the current folder: {})", e.kind()),
    };
    let mut entries = Vec::new();
    let mut hidden = 0usize;
    for item in read.flatten() {
        let raw = item.file_name().to_string_lossy().into_owned();
        if raw.starts_with('.') {
            hidden += 1;
            continue;
        }
        // 심볼릭 링크는 따라가지 않음
        let (kind, size) = match item.metadata() {
            Ok(m) if m.file_type().is_symlink() => (Kind::Link, 0),
            Ok(m) if m.is_dir() => (Kind::Dir, 0),
            Ok(m) if m.is_file() => (Kind::File, m.len()),
            _ => (Kind::Other, 0),
        };
        entries.push(Entry {
            name: clean(&raw),
            kind,
            size,
        });
    }
    entries.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    let total = entries.len();
    let mut out = String::new();
    let mut shown = 0;
    for e in &entries {
        let line = match e.kind {
            Kind::Dir => format!("{}/", e.name),
            Kind::File => format!("{}  {}", e.name, human(e.size)),
            Kind::Link => format!("{}@", e.name),
            Kind::Other => e.name.clone(),
        };
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
    if hidden > 0 {
        out.push_str(&format!("({hidden} hidden entries not listed)\n"));
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_names_types_sizes_without_contents_or_hidden() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        fs::write(d.join("backup.tar.gz"), vec![0u8; 2048]).unwrap();
        fs::write(d.join("Notes.txt"), "secret content").unwrap();
        fs::write(d.join(".env"), "TOKEN=abc").unwrap();
        fs::create_dir(d.join("src")).unwrap();
        fs::write(d.join("src").join("inner.rs"), "x").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc/passwd", d.join("link")).unwrap();
        let text = folder_listing(d);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "src/", "폴더 먼저");
        assert!(lines.contains(&"backup.tar.gz  2.0 KB"));
        assert!(lines.contains(&"Notes.txt  14 B"));
        #[cfg(unix)]
        assert!(lines.contains(&"link@"));
        assert!(text.ends_with("(1 hidden entries not listed)"));
        for absent in ["secret content", "TOKEN", ".env", "inner.rs", "passwd"] {
            assert!(!text.contains(absent), "{absent}");
        }
        assert!(!text.contains(&d.display().to_string()), "절대 경로 없음");
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
        assert_eq!(folder_listing(empty.path()), "(empty folder)");
        assert!(folder_listing(&empty.path().join("missing")).starts_with("(cannot read"));
    }
}
