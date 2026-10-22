//! 확정 명령어 전달 (SRS 6.3절, ADR-0003). TUI 종료와 fd 복구 이후에만 호출한다.

use std::io::{self, Write};
use std::path::Path;

/// 명령어 정규화: CRLF를 LF로. 내부 공백과 Unicode는 그대로 보존.
pub fn normalize(command: &str) -> String {
    command.replace("\r\n", "\n").replace('\r', "\n")
}

/// stdout: 명령어 바이트 + LF 1개.
pub fn emit_stdout(command: &str) -> io::Result<()> {
    let mut out = io::stdout().lock();
    write_command(&mut out, command)
}

pub fn write_command(out: &mut dyn Write, command: &str) -> io::Result<()> {
    let text = normalize(command);
    out.write_all(text.trim_end_matches('\n').as_bytes())?;
    out.write_all(b"\n")?;
    out.flush()
}

/// 위젯 모드 출력 파일: UTF-8, BOM 없음, LF, 끝 줄바꿈 없음.
pub fn emit_file(path: &Path, command: &str) -> io::Result<()> {
    let text = normalize(command);
    crate::platform::fs::write_private_overwrite(path, text.trim_end_matches('\n').as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stdout_format_is_command_plus_single_lf() {
        let mut buf = Vec::new();
        write_command(&mut buf, "printf '%s\\n' \"안녕 🌏\" \\\r\n  | grep 안\n").unwrap();
        assert_eq!(
            buf,
            "printf '%s\\n' \"안녕 🌏\" \\\n  | grep 안\n".as_bytes()
        );
    }

    #[test]
    fn file_format_has_no_trailing_newline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out");
        emit_file(&path, "a\r\nb\n").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"a\nb");
    }
}
