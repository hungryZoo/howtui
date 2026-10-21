//! Phase 0 기술 검증용 실험 프로그램 (SPIKE-01).
//!
//! 검증 대상
//! (1) Viewport::Inline 렌더링을 stdout 이외의 핸들로 출력할 때 stdout 순수성 유지 여부
//! (2) crossterm cursor::position()의 stdout 질의 문제와 fd 1 임시 전환 우회
//! (3) 비동기 요청 중 취소, 결과 반영, 직접 편집, 확정 후 stdout 출력
//! (4) Unicode 및 여러 줄 명령어의 바이트 단위 보존
//! (5) 취소, 오류, panic 시 터미널 복구
//!
//! 옵션
//!   --naive        fd 1 전환 없이 stderr 백엔드만 사용 (오염 재현용)
//!   --panic        첫 렌더링 직후 panic 발생 (복구 확인용)
//!   --height N     Inline 높이 (기본 7)
//!   --quiet        진단 줄 생략

use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::{cursor, execute, terminal};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{Terminal, TerminalOptions, Viewport};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

const MOCK_COMMAND: &str = "printf '%s\\n' \"안녕 세계 🌏\" \\\n  | grep -n '세계'";
const MOCK_DELAY: Duration = Duration::from_millis(1500);

#[derive(Clone, Copy, PartialEq)]
enum Focus {
    Ask,
    Command,
}

enum Msg {
    Input(Event),
    Llm(String),
}

struct App {
    focus: Focus,
    question: String,
    command: String,
    status: String,
    pending: Option<JoinHandle<()>>,
    spinner: usize,
}

/// Unix에서 fd 1을 /dev/tty로 임시 전환하고, 종료 시 원래 stdout으로 복구한다.
#[cfg(unix)]
struct StdoutRedirect {
    saved: i32,
}

#[cfg(unix)]
impl StdoutRedirect {
    fn to(tty: &std::fs::File) -> io::Result<Self> {
        use std::os::fd::AsRawFd;
        io::stdout().flush()?;
        let saved = unsafe { libc::dup(1) };
        if saved < 0 {
            return Err(io::Error::last_os_error());
        }
        if unsafe { libc::dup2(tty.as_raw_fd(), 1) } < 0 {
            let err = io::Error::last_os_error();
            unsafe { libc::close(saved) };
            return Err(err);
        }
        Ok(Self { saved })
    }

    fn restore(self) -> io::Result<()> {
        io::stdout().flush()?;
        let rc = unsafe { libc::dup2(self.saved, 1) };
        unsafe { libc::close(self.saved) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let naive = args.iter().any(|a| a == "--naive");
    let panic_test = args.iter().any(|a| a == "--panic");
    let quiet = args.iter().any(|a| a == "--quiet");
    let height = args
        .iter()
        .position(|a| a == "--height")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(7u16);

    // TTY 판정: 입력용 터미널이 없으면 TUI를 시작하지 않고 제어 시퀀스 없이 실패한다.
    let tty = match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("howtui-spike: 터미널(/dev/tty)을 열 수 없음: {e}");
            return ExitCode::from(3);
        }
    };
    if !quiet {
        eprintln!(
            "[spike] stdin_tty={} stdout_tty={} stderr_tty={} naive={naive}",
            io::stdin().is_terminal(),
            io::stdout().is_terminal(),
            io::stderr().is_terminal()
        );
    }

    #[cfg(unix)]
    let redirect = if naive {
        None
    } else {
        match StdoutRedirect::to(&tty) {
            Ok(r) => Some(r),
            Err(e) => {
                eprintln!("howtui-spike: stdout 전환 실패: {e}");
                return ExitCode::from(1);
            }
        }
    };

    let restore_tty = tty.try_clone().expect("tty clone");
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let mut out = &restore_tty;
        let _ = terminal::disable_raw_mode();
        let _ = execute!(out, cursor::Show);
        let _ = writeln!(out);
        default_hook(info);
    }));

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let result = rt.block_on(run(tty, height, panic_test));

    #[cfg(unix)]
    if let Some(r) = redirect {
        if let Err(e) = r.restore() {
            eprintln!("howtui-spike: stdout 복구 실패: {e}");
            return ExitCode::from(1);
        }
    }

    match result {
        Ok(Some(command)) => {
            let mut out = io::stdout().lock();
            let _ = out.write_all(command.as_bytes());
            let _ = out.write_all(b"\n");
            let _ = out.flush();
            ExitCode::SUCCESS
        }
        Ok(None) => ExitCode::from(130),
        Err(e) => {
            eprintln!("howtui-spike: 오류: {e}");
            ExitCode::from(1)
        }
    }
}

async fn run(mut tty: std::fs::File, height: u16, panic_test: bool) -> io::Result<Option<String>> {
    terminal::enable_raw_mode()?;
    // 프롬프트 줄 중간에서 시작하면 다음 줄에서 Inline 영역을 시작하고, 종료 시 원래 커서 위치로 복귀한다.
    let origin = cursor::position()?;
    if origin.0 > 0 {
        tty.write_all(b"\r\n")?;
        tty.flush()?;
    }
    let backend = CrosstermBackend::new(tty);
    let mut term = match Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(height),
        },
    ) {
        Ok(t) => t,
        Err(e) => {
            let _ = terminal::disable_raw_mode();
            return Err(e);
        }
    };

    let (tx, mut rx) = mpsc::unbounded_channel::<Msg>();
    let stop = Arc::new(AtomicBool::new(false));
    let input_thread = {
        let tx = tx.clone();
        let stop = stop.clone();
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                match event::poll(Duration::from_millis(50)) {
                    Ok(true) => match event::read() {
                        Ok(ev) => {
                            if tx.send(Msg::Input(ev)).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    },
                    Ok(false) => {}
                    Err(_) => break,
                }
            }
        })
    };

    let mut app = App {
        focus: Focus::Ask,
        question: String::new(),
        command: String::new(),
        status: "질문 입력 후 Enter".into(),
        pending: None,
        spinner: 0,
    };
    let mut tick = tokio::time::interval(Duration::from_millis(120));
    let outcome: io::Result<Option<String>> = loop {
        if let Err(e) = term.draw(|f| draw(f, &app)) {
            break Err(e);
        }
        if panic_test {
            panic!("spike panic test");
        }
        tokio::select! {
            Some(msg) = rx.recv() => match msg {
                Msg::Input(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    match handle_key(&mut app, key, &tx) {
                        Step::Continue => {}
                        Step::Accept => break Ok(Some(app.command.clone())),
                        Step::Cancel => break Ok(None),
                    }
                }
                Msg::Input(Event::Paste(text)) => insert_text(&mut app, &text),
                Msg::Input(_) => {}
                Msg::Llm(command) => {
                    app.pending = None;
                    app.command = command;
                    app.focus = Focus::Command;
                    app.status = "명령어 편집 가능, Enter 확정, Alt+Enter 줄바꿈".into();
                }
            },
            _ = tick.tick() => {
                if app.pending.is_some() {
                    app.spinner = app.spinner.wrapping_add(1);
                }
            }
        }
    };

    stop.store(true, Ordering::Relaxed);
    let _ = input_thread.join();
    if let Some(task) = app.pending.take() {
        task.abort();
    }
    // Inline 영역을 지우고 커서를 영역 시작 위치로 되돌린다.
    let area = term.get_frame().area();
    let _ = term.clear();
    if origin.0 > 0 {
        let _ = term.set_cursor_position((origin.0, area.y.saturating_sub(1)));
    } else {
        let _ = term.set_cursor_position(area.as_position());
    }
    let _ = term.show_cursor();
    terminal::disable_raw_mode()?;
    outcome
}

enum Step {
    Continue,
    Accept,
    Cancel,
}

fn handle_key(app: &mut App, key: KeyEvent, tx: &mpsc::UnboundedSender<Msg>) -> Step {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    match key.code {
        KeyCode::Char('c') if ctrl => return Step::Cancel,
        KeyCode::Esc => {
            if let Some(task) = app.pending.take() {
                task.abort();
                app.status = "요청 취소됨".into();
            } else {
                return Step::Cancel;
            }
        }
        KeyCode::Tab => {
            app.focus = match app.focus {
                Focus::Ask => Focus::Command,
                Focus::Command => Focus::Ask,
            };
        }
        KeyCode::Enter if alt => insert_text(app, "\n"),
        KeyCode::Char('j') if ctrl => insert_text(app, "\n"),
        KeyCode::Enter => match app.focus {
            Focus::Ask if !app.question.is_empty() && app.pending.is_none() => {
                let tx = tx.clone();
                app.status = "생성 중".into();
                app.pending = Some(tokio::spawn(async move {
                    tokio::time::sleep(MOCK_DELAY).await;
                    let _ = tx.send(Msg::Llm(MOCK_COMMAND.to_string()));
                }));
            }
            Focus::Command if !app.command.is_empty() => return Step::Accept,
            _ => {}
        },
        KeyCode::Backspace => {
            target(app).pop();
        }
        KeyCode::Char(c) if !ctrl => target(app).push(c),
        _ => {}
    }
    Step::Continue
}

fn target(app: &mut App) -> &mut String {
    match app.focus {
        Focus::Ask => &mut app.question,
        Focus::Command => &mut app.command,
    }
}

fn insert_text(app: &mut App, text: &str) {
    target(app).push_str(text);
}

fn draw(f: &mut ratatui::Frame, app: &App) {
    const SPIN: [&str; 4] = ["|", "/", "-", "\\"];
    let cmd_lines = app.command.lines().count().max(1) as u16;
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(cmd_lines.min(3)),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(f.area());
    let focus_style = |focus| {
        if app.focus == focus {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        }
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("질문> ", focus_style(Focus::Ask)),
            Span::raw(app.question.as_str()),
        ])),
        rows[0],
    );
    let mut cmd: Vec<Line> = app.command.lines().map(Line::raw).collect();
    if let Some(first) = cmd.first_mut() {
        first.spans.insert(0, Span::styled("명령> ", focus_style(Focus::Command)));
    } else {
        cmd.push(Line::from(Span::styled("명령> ", focus_style(Focus::Command))));
    }
    f.render_widget(Paragraph::new(cmd), rows[1]);
    let status = if app.pending.is_some() {
        format!("{} {} (Esc 취소)", SPIN[app.spinner % 4], app.status)
    } else {
        app.status.clone()
    };
    f.render_widget(
        Paragraph::new(Span::styled(status, Style::default().fg(Color::Yellow))),
        rows[2],
    );
    f.render_widget(
        Paragraph::new(Span::styled(
            "Tab 포커스 전환  Ctrl+C 종료",
            Style::default().fg(Color::DarkGray),
        )),
        rows[3],
    );
}
