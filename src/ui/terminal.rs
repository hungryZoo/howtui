//! 터미널 세션 수명 주기 (SRS 6.1절, 6.2절). ratatui의 init과 restore는 stdout 고정이라 사용하지 않는다 (SRS-MOD-008).

use std::io::{self, BufWriter, Write};
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::cursor::{MoveTo, Show};
use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};
use ratatui::backend::CrosstermBackend;
use ratatui::{Frame, Terminal, TerminalOptions, Viewport};

use crate::platform::tty::{self, TermOut};

type Backend = CrosstermBackend<BufWriter<TermOut>>;

static SESSION_ACTIVE: AtomicBool = AtomicBool::new(false);
static KEEP_PASTE: AtomicBool = AtomicBool::new(false);
static MOUSE_ON: AtomicBool = AtomicBool::new(false);
static FULLSCREEN: AtomicBool = AtomicBool::new(false);
/// 마우스 보고 해제 (SGR, urxvt, 이동, 버튼 이벤트 모드)
const MOUSE_OFF_SEQ: &[u8] = b"\x1b[?1006l\x1b[?1015l\x1b[?1003l\x1b[?1002l\x1b[?1000l";

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("no usable terminal: {0}")]
    NoTerminal(String),
    #[error(transparent)]
    Io(#[from] io::Error),
}

pub struct TerminalSession {
    terminal: Terminal<Backend>,
    #[cfg(unix)]
    stdout_guard: Option<tty::StdoutGuard>,
    origin_col: u16,
    added_newline: bool,
    keep_paste: bool,
    mouse: bool,
    fullscreen: bool,
    restored: bool,
}

impl TerminalSession {
    /// `keep_paste`: 종료 시 bracketed paste를 켠 상태로 둔다 (쉘 위젯 모드).
    /// `mouse`: 클릭 입력 수신 (CR-005). 켜져 있는 동안 터미널 텍스트 선택은 Shift+드래그.
    /// `fullscreen`: Alternate Screen과 전체 화면 뷰포트 (PRD-FS-001). 종료 시 기존 화면 복구.
    pub fn start(
        height: u16,
        keep_paste: bool,
        mouse: bool,
        fullscreen: bool,
    ) -> Result<Self, SessionError> {
        let out = tty::open_term_out().map_err(|e| SessionError::NoTerminal(e.to_string()))?;
        #[cfg(unix)]
        let stdout_guard = Some(tty::StdoutGuard::redirect_to(&out)?);
        if let Err(e) = terminal::enable_raw_mode() {
            return Err(SessionError::NoTerminal(e.to_string()));
        }
        SESSION_ACTIVE.store(true, Ordering::SeqCst);
        KEEP_PASTE.store(keep_paste, Ordering::SeqCst);
        let setup = (|| {
            let mut writer = BufWriter::with_capacity(16 * 1024, out);
            // 커서 질의 무응답 터미널은 TUI 미지원 (SRS-CMP-003). Fullscreen은 시작 위치 불필요
            let origin = if fullscreen {
                (0, 0)
            } else {
                crossterm::cursor::position()
                    .map_err(|e| SessionError::NoTerminal(e.to_string()))?
            };
            if origin.0 > 0 {
                writer.write_all(b"\r\n")?;
            }
            if fullscreen {
                execute!(writer, EnterAlternateScreen)?;
                FULLSCREEN.store(true, Ordering::SeqCst);
            }
            execute!(writer, EnableBracketedPaste)?;
            if mouse {
                execute!(writer, EnableMouseCapture)?;
                MOUSE_ON.store(true, Ordering::SeqCst);
            }
            let backend = CrosstermBackend::new(writer);
            let terminal = Terminal::with_options(
                backend,
                TerminalOptions {
                    viewport: if fullscreen {
                        Viewport::Fullscreen
                    } else {
                        Viewport::Inline(height)
                    },
                },
            )?;
            Ok::<_, SessionError>((terminal, origin))
        })();
        let (terminal, origin) = match setup {
            Ok(v) => v,
            Err(e) => {
                emergency_restore();
                return Err(e);
            }
        };
        let added_newline = origin.0 > 0;
        Ok(Self {
            terminal,
            #[cfg(unix)]
            stdout_guard,
            origin_col: origin.0,
            added_newline,
            keep_paste,
            mouse,
            fullscreen,
            restored: false,
        })
    }

    /// 마우스 보고 켜기와 끄기 (로그인 URL 복사 중에는 끔).
    pub fn set_mouse(&mut self, on: bool) -> io::Result<()> {
        if on == self.mouse || self.restored {
            return Ok(());
        }
        let w = self.terminal.backend_mut();
        if on {
            execute!(w, EnableMouseCapture)?;
        } else {
            execute!(w, DisableMouseCapture)?;
        }
        self.mouse = on;
        MOUSE_ON.store(on, Ordering::SeqCst);
        Ok(())
    }

    pub fn draw<F: FnOnce(&mut Frame)>(&mut self, render: F) -> io::Result<()> {
        self.terminal.draw(render).map(|_| ())
    }

    /// Inline 영역 삭제, 커서 원위치, 모드 복구, stdout 복구 (멱등).
    pub fn restore(&mut self) -> io::Result<()> {
        if self.restored {
            return Ok(());
        }
        self.restored = true;
        let area = self.terminal.get_frame().area();
        let w = self.terminal.backend_mut();
        if self.fullscreen {
            // 기존 화면과 커서는 터미널이 복원 (PRD-FS-004)
            let _ = queue!(w, LeaveAlternateScreen);
            FULLSCREEN.store(false, Ordering::SeqCst);
        } else {
            let _ = queue!(w, MoveTo(area.x, area.y), Clear(ClearType::FromCursorDown));
            let _ = if self.added_newline {
                queue!(w, MoveTo(self.origin_col, area.y.saturating_sub(1)))
            } else {
                queue!(w, MoveTo(0, area.y))
            };
        }
        if !self.keep_paste {
            let _ = queue!(w, DisableBracketedPaste);
        }
        if self.mouse {
            let _ = queue!(w, DisableMouseCapture);
            MOUSE_ON.store(false, Ordering::SeqCst);
        }
        let _ = queue!(w, Show);
        let _ = w.flush();
        let raw = terminal::disable_raw_mode();
        SESSION_ACTIVE.store(false, Ordering::SeqCst);
        #[cfg(unix)]
        if let Some(guard) = self.stdout_guard.take() {
            guard.restore()?;
        }
        raw
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

/// panic hook에서 호출하는 최소 복구 (SRS-TERM-006).
pub fn emergency_restore() {
    if !SESSION_ACTIVE.swap(false, Ordering::SeqCst) {
        return;
    }
    let _ = terminal::disable_raw_mode();
    let mut seq = Vec::new();
    if FULLSCREEN.swap(false, Ordering::SeqCst) {
        seq.extend_from_slice(b"\x1b[?1049l");
    }
    if !KEEP_PASTE.load(Ordering::SeqCst) {
        seq.extend_from_slice(b"\x1b[?2004l");
    }
    if MOUSE_ON.swap(false, Ordering::SeqCst) {
        seq.extend_from_slice(MOUSE_OFF_SEQ);
        #[cfg(windows)]
        {
            use crossterm::Command;
            let _ = DisableMouseCapture.execute_winapi();
        }
    }
    seq.extend_from_slice(b"\x1b[?25h\r\n");
    tty::emergency_write(&seq);
    tty::emergency_restore_stdout();
}

pub fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        emergency_restore();
        default(info);
    }));
}

/// doctor용 렌더링 가능 여부 점검: 제어 터미널 열기와 커서 위치 질의 (SRS-DOC-001).
pub fn probe() -> Result<(u16, u16), SessionError> {
    let out = tty::open_term_out().map_err(|e| SessionError::NoTerminal(e.to_string()))?;
    #[cfg(unix)]
    let guard = tty::StdoutGuard::redirect_to(&out)?;
    let result = (|| {
        terminal::enable_raw_mode().map_err(|e| SessionError::NoTerminal(e.to_string()))?;
        let pos = crossterm::cursor::position();
        let _ = terminal::disable_raw_mode();
        pos.map_err(|e| SessionError::NoTerminal(e.to_string()))?;
        terminal::size().map_err(SessionError::Io)
    })();
    #[cfg(unix)]
    guard.restore()?;
    drop(out);
    result
}
