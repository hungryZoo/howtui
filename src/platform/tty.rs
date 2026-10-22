//! 제어 터미널 열기와 stdout 보호 (SRS-TERM-001-004, ADR-0002).

use std::io::{self, Write};

#[cfg(unix)]
pub use unix::*;
#[cfg(windows)]
pub use windows::*;

#[cfg(unix)]
mod unix {
    use super::*;
    use std::fs::{File, OpenOptions};
    use std::os::fd::{AsRawFd, RawFd};
    use std::sync::atomic::{AtomicI32, Ordering};

    /// TUI 출력 대상. 제어 터미널(/dev/tty).
    pub type TermOut = File;

    static SAVED_STDOUT: AtomicI32 = AtomicI32::new(-1);
    static EMERGENCY_TTY: AtomicI32 = AtomicI32::new(-1);

    pub fn open_term_out() -> io::Result<TermOut> {
        let tty = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
        // panic 시 복구 출력용 사본
        let dup = unsafe { libc::dup(tty.as_raw_fd()) };
        if dup >= 0 {
            let old = EMERGENCY_TTY.swap(dup, Ordering::SeqCst);
            if old >= 0 {
                unsafe { libc::close(old) };
            }
        }
        Ok(tty)
    }

    /// fd 1을 제어 터미널로 임시 전환. crossterm 커서 위치 질의(`ESC[6n`)가 stdout에 쓰이는 문제 대응.
    pub struct StdoutGuard {
        saved: RawFd,
    }

    impl StdoutGuard {
        pub fn redirect_to(tty: &File) -> io::Result<Self> {
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
            SAVED_STDOUT.store(saved, Ordering::SeqCst);
            Ok(Self { saved })
        }

        pub fn restore(self) -> io::Result<()> {
            let result = restore_fd(self.saved);
            std::mem::forget(self);
            result
        }
    }

    impl Drop for StdoutGuard {
        fn drop(&mut self) {
            let _ = restore_fd(self.saved);
        }
    }

    fn restore_fd(saved: RawFd) -> io::Result<()> {
        if SAVED_STDOUT
            .compare_exchange(saved, -1, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Ok(());
        }
        let _ = io::stdout().flush();
        let rc = unsafe { libc::dup2(saved, 1) };
        unsafe { libc::close(saved) };
        if rc < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// panic hook 경로: stdout 복구.
    pub fn emergency_restore_stdout() {
        let saved = SAVED_STDOUT.load(Ordering::SeqCst);
        if saved >= 0 {
            let _ = restore_fd(saved);
        }
    }

    /// panic hook 경로: 제어 터미널에 직접 기록.
    pub fn emergency_write(bytes: &[u8]) {
        let fd = EMERGENCY_TTY.load(Ordering::SeqCst);
        if fd >= 0 {
            unsafe { libc::write(fd, bytes.as_ptr().cast(), bytes.len()) };
        }
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::io::IsTerminal;
    use windows_sys::Win32::Foundation::{
        CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Console::WriteConsoleW;

    /// TUI 출력 대상 선택 순서: stdout 콘솔 → stderr 콘솔 → CONOUT$ (SRS 6.1절).
    pub enum TermOut {
        Stdout(io::Stdout),
        Stderr(io::Stderr),
        Conout(ConoutWriter),
    }

    pub fn open_term_out() -> io::Result<TermOut> {
        if io::stdout().is_terminal() {
            Ok(TermOut::Stdout(io::stdout()))
        } else if io::stderr().is_terminal() {
            Ok(TermOut::Stderr(io::stderr()))
        } else {
            ConoutWriter::open().map(TermOut::Conout)
        }
    }

    impl Write for TermOut {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            match self {
                TermOut::Stdout(w) => w.write(buf),
                TermOut::Stderr(w) => w.write(buf),
                TermOut::Conout(w) => w.write(buf),
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            match self {
                TermOut::Stdout(w) => w.flush(),
                TermOut::Stderr(w) => w.flush(),
                TermOut::Conout(w) => w.flush(),
            }
        }
    }

    /// `CONOUT$` 핸들에 UTF-16으로 기록 (콘솔 코드 페이지 무관).
    pub struct ConoutWriter {
        handle: HANDLE,
        pending: Vec<u8>,
    }

    impl ConoutWriter {
        pub fn open() -> io::Result<Self> {
            let name: Vec<u16> = "CONOUT$\0".encode_utf16().collect();
            let handle = unsafe {
                CreateFileW(
                    name.as_ptr(),
                    GENERIC_READ | GENERIC_WRITE,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    0,
                    std::ptr::null_mut(),
                )
            };
            if handle == INVALID_HANDLE_VALUE || handle.is_null() {
                return Err(io::Error::last_os_error());
            }
            Ok(Self {
                handle,
                pending: Vec::new(),
            })
        }

        fn write_wide(&self, text: &str) -> io::Result<()> {
            let wide: Vec<u16> = text.encode_utf16().collect();
            let mut offset = 0;
            while offset < wide.len() {
                let mut written: u32 = 0;
                let chunk = &wide[offset..];
                let ok = unsafe {
                    WriteConsoleW(
                        self.handle,
                        chunk.as_ptr().cast(),
                        chunk.len() as u32,
                        &mut written,
                        std::ptr::null(),
                    )
                };
                if ok == 0 {
                    return Err(io::Error::last_os_error());
                }
                if written == 0 {
                    break;
                }
                offset += written as usize;
            }
            Ok(())
        }
    }

    impl Write for ConoutWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.pending.extend_from_slice(buf);
            // 완전한 UTF-8 접두부만 기록, 잘린 다중 바이트 문자는 다음 호출로 보류
            let valid = match std::str::from_utf8(&self.pending) {
                Ok(s) => s.len(),
                Err(e) if e.error_len().is_none() => e.valid_up_to(),
                Err(_) => self.pending.len(),
            };
            let text = String::from_utf8_lossy(&self.pending[..valid]).into_owned();
            self.write_wide(&text)?;
            self.pending.drain(..valid);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Drop for ConoutWriter {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.handle) };
        }
    }

    pub fn emergency_restore_stdout() {}

    pub fn emergency_write(bytes: &[u8]) {
        let _ = io::stderr().write_all(bytes);
    }
}
