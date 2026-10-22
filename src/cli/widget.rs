//! 확정 명령어 전달 경로 선택 (SRS 9.2절 위젯 프로토콜, SRS 9.6절 다음 입력 전달).

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::i18n::{Msg, tr, trf};
use crate::output;

/// 위젯 모드 입력 (쉘 스크립트가 환경변수로 전달).
#[derive(Debug, PartialEq, Eq)]
pub struct WidgetInput {
    pub output: PathBuf,
    pub buffer: String,
    pub lbuffer: Option<String>,
    /// 쉘이 bracketed paste를 쓰는 중이면 종료 시 해제하지 않음
    pub keep_paste: bool,
}

impl WidgetInput {
    /// `HOWTUI_OUTPUT_FILE` 미설정 시 None (종료 코드 2).
    pub fn from_env(env: &dyn Fn(&str) -> Option<String>) -> Option<WidgetInput> {
        let output = env("HOWTUI_OUTPUT_FILE").filter(|p| !p.is_empty())?;
        Some(WidgetInput {
            output: PathBuf::from(output),
            buffer: env("HOWTUI_BUFFER").unwrap_or_default(),
            lbuffer: env("HOWTUI_LBUFFER").filter(|s| !s.is_empty()),
            keep_paste: env("HOWTUI_BRACKETED_PASTE").as_deref() == Some("1"),
        })
    }

    pub fn initial_command(&self) -> Option<(String, Option<String>)> {
        (!self.buffer.is_empty()).then(|| (self.buffer.clone(), self.lbuffer.clone()))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Destination {
    Stdout,
    /// 쉘 함수 `howtui`가 다음 프롬프트 입력란에 넣을 파일 (CR-003)
    NextInput(PathBuf),
    Widget(PathBuf),
}

impl Destination {
    /// 다음 입력 전달은 stdout이 터미널일 때만. 캡처와 파이프, 리디렉션은 stdout 유지.
    pub fn select(
        widget: Option<&WidgetInput>,
        env: &dyn Fn(&str) -> Option<String>,
        stdout_is_terminal: bool,
    ) -> Destination {
        if let Some(w) = widget {
            return Destination::Widget(w.output.clone());
        }
        match env("HOWTUI_NEXT_INPUT_FILE").filter(|p| !p.is_empty()) {
            Some(path) if stdout_is_terminal => Destination::NextInput(PathBuf::from(path)),
            _ => Destination::Stdout,
        }
    }

    pub fn current(widget: Option<&WidgetInput>, env: &dyn Fn(&str) -> Option<String>) -> Self {
        Destination::select(widget, env, std::io::stdout().is_terminal())
    }

    pub fn deliver(&self, command: &str) -> ExitCode {
        match self {
            Destination::Stdout => stdout(command),
            Destination::NextInput(path) => match output::emit_file(path, command) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    tracing::warn!(error = %e, "next input file write failed");
                    stdout(command)
                }
            },
            Destination::Widget(path) => match output::emit_file(path, command) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    // 대체 경로: 터미널(위젯 모드 stderr)에 원인과 명령어 표시 (SRS-SH-009).
                    // 커서는 쉘 입력 줄 끝(TUI 시작 위치)이므로 줄을 바꾼 뒤 출력
                    eprintln!();
                    eprintln!(
                        "{}",
                        trf(Msg::ErrWidgetWrite, &[("detail", &e.to_string())])
                    );
                    eprintln!("{}", output::normalize(command).trim_end_matches('\n'));
                    ExitCode::from(1)
                }
            },
        }
    }
}

fn stdout(command: &str) -> ExitCode {
    match output::emit_stdout(command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", trf(Msg::ErrInternal, &[("detail", &e.to_string())]));
            ExitCode::from(1)
        }
    }
}

pub fn missing_output() -> ExitCode {
    eprintln!("{}", tr(Msg::ErrWidgetNoOutput));
    ExitCode::from(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |k: &str| {
            pairs
                .iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn widget_input_requires_output_file() {
        assert_eq!(
            WidgetInput::from_env(&env_of(&[("HOWTUI_BUFFER", "ls")])),
            None
        );
        let w = WidgetInput::from_env(&env_of(&[
            ("HOWTUI_OUTPUT_FILE", "/tmp/x"),
            ("HOWTUI_BUFFER", "ls -l"),
            ("HOWTUI_LBUFFER", "ls"),
            ("HOWTUI_BRACKETED_PASTE", "1"),
        ]))
        .unwrap();
        assert_eq!(
            w.initial_command(),
            Some(("ls -l".to_string(), Some("ls".to_string())))
        );
        assert!(w.keep_paste);
        let empty = WidgetInput::from_env(&env_of(&[
            ("HOWTUI_OUTPUT_FILE", "/tmp/x"),
            ("HOWTUI_LBUFFER", ""),
        ]))
        .unwrap();
        assert_eq!(empty.initial_command(), None);
        assert!(!empty.keep_paste);
    }

    #[test]
    fn next_input_only_when_stdout_is_terminal() {
        let env = env_of(&[("HOWTUI_NEXT_INPUT_FILE", "/tmp/n")]);
        assert_eq!(
            Destination::select(None, &env, true),
            Destination::NextInput(PathBuf::from("/tmp/n"))
        );
        assert_eq!(Destination::select(None, &env, false), Destination::Stdout);
        assert_eq!(
            Destination::select(None, &env_of(&[("HOWTUI_NEXT_INPUT_FILE", "")]), true),
            Destination::Stdout
        );
        let w = WidgetInput::from_env(&env_of(&[("HOWTUI_OUTPUT_FILE", "/tmp/w")])).unwrap();
        assert_eq!(
            Destination::select(Some(&w), &env, true),
            Destination::Widget(PathBuf::from("/tmp/w"))
        );
    }

    #[test]
    fn file_destinations_write_and_widget_failure_exits_1() {
        let dir = tempfile::tempdir().unwrap();
        let ok = dir.path().join("next");
        assert_eq!(
            Destination::NextInput(ok.clone()).deliver("ls -la"),
            ExitCode::SUCCESS
        );
        assert_eq!(std::fs::read(&ok).unwrap(), b"ls -la");
        let missing = dir.path().join("no/such/dir");
        assert_eq!(
            Destination::Widget(missing).deliver("ls"),
            ExitCode::from(1)
        );
    }
}
