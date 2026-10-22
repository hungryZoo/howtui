//! Windows ConPTY E2E (TC-UI-014, TC-OUT-009, TC-KEY-005 일부): 의사 콘솔에서 TUI 구동과 stdout 분리 검증.
//! 출력 핸들 선택 경로 3종(SRS 6.1절): stdout 콘솔, stdout 리디렉션(stderr 콘솔), stdout과 stderr 리디렉션(CONOUT$).
#![cfg(all(windows, feature = "mock-provider"))]

use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

struct Session {
    output: Arc<Mutex<Vec<u8>>>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl Session {
    fn start(args: &[String], data: &Path) -> Session {
        let pty = native_pty_system();
        let pair = pty
            .openpty(PtySize {
                rows: 30,
                cols: 100,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let mut cmd = CommandBuilder::new("cmd.exe");
        cmd.arg("/c");
        for a in args {
            cmd.arg(a);
        }
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("mock_basic.json");
        cmd.env("HOWTUI_MOCK_SCENARIO", fixture.display().to_string());
        cmd.env("HOWTUI_LANG", "en");
        cmd.env("HOWTUI_DATA_DIR", data.display().to_string());
        cmd.env(
            "HOWTUI_CONFIG",
            data.join("none.toml").display().to_string(),
        );
        let child = pair.slave.spawn_command(cmd).unwrap();
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().unwrap();
        let writer = Arc::new(Mutex::new(pair.master.take_writer().unwrap()));
        let output = Arc::new(Mutex::new(Vec::new()));
        let (out, w) = (output.clone(), writer.clone());
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let chunk = &buf[..n];
                // ConPTY 커서 상속 모드의 위치 질의 응답
                if chunk.windows(4).any(|x| x == b"\x1b[6n") {
                    let _ = w.lock().unwrap().write_all(b"\x1b[1;1R");
                }
                out.lock().unwrap().extend_from_slice(chunk);
            }
        });
        // master는 child 종료까지 유지되어야 하므로 누수 허용
        std::mem::forget(pair.master);
        let writer: Box<dyn Write + Send> = Box::new(ArcWriter(writer));
        Session {
            output,
            writer,
            child,
        }
    }

    fn wait_for(&self, needle: &str, timeout: Duration) -> bool {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if String::from_utf8_lossy(&self.output.lock().unwrap()).contains(needle) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    fn send(&mut self, bytes: &[u8]) {
        self.writer.write_all(bytes).unwrap();
        self.writer.flush().unwrap();
    }

    fn wait_exit(&mut self, timeout: Duration) -> Option<u32> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if let Ok(Some(status)) = self.child.try_wait() {
                return Some(status.exit_code());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill();
        None
    }

    fn dump(&self) -> String {
        String::from_utf8_lossy(&self.output.lock().unwrap())
            .escape_debug()
            .to_string()
    }
}

struct ArcWriter(Arc<Mutex<Box<dyn Write + Send>>>);

impl Write for ArcWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}

const EXPECTED: &str = "printf '%s\\n' \"안녕 세계 🌏\" \\\n  | grep -n '세계'\n";

fn run_case(redirect_stderr: bool) {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.txt");
    let err = dir.path().join("err.txt");
    let mut args = vec![
        env!("CARGO_BIN_EXE_howtui").to_string(),
        ">".into(),
        out.display().to_string(),
    ];
    if redirect_stderr {
        args.extend(["2>".to_string(), err.display().to_string()]);
    }
    let mut s = Session::start(&args, dir.path());
    assert!(
        s.wait_for("Ask", Duration::from_secs(20)),
        "TUI not drawn: {}",
        s.dump()
    );
    s.send(b"find big files");
    s.send(b"\r");
    assert!(
        s.wait_for("Prints a greeting", Duration::from_secs(20)),
        "response not shown: {}",
        s.dump()
    );
    s.send(b"\r");
    assert_eq!(
        s.wait_exit(Duration::from_secs(20)),
        Some(0),
        "exit: {}",
        s.dump()
    );
    let got = std::fs::read(&out).unwrap();
    assert_eq!(
        String::from_utf8_lossy(&got),
        EXPECTED,
        "stdout file must contain only the command"
    );
    assert!(!got.contains(&0x1b), "no control sequences in stdout");
    if redirect_stderr {
        assert!(
            std::fs::read(&err).unwrap_or_default().is_empty(),
            "TUI must not draw on redirected stderr"
        );
    }
}

#[test]
fn stdout_redirected_tui_on_stderr_console() {
    run_case(false);
}

#[test]
fn stdout_and_stderr_redirected_tui_on_conout() {
    run_case(true);
}

#[test]
fn console_stdout_cancel_with_ctrl_c() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Session::start(&[env!("CARGO_BIN_EXE_howtui").to_string()], dir.path());
    assert!(
        s.wait_for("Ask", Duration::from_secs(20)),
        "TUI not drawn: {}",
        s.dump()
    );
    s.send(b"\x03");
    assert_eq!(
        s.wait_exit(Duration::from_secs(20)),
        Some(130),
        "exit: {}",
        s.dump()
    );
}
