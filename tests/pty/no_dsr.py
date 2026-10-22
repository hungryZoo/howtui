# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
# 커서 위치 질의(ESC[6n)에 응답하지 않는 PTY에서 TUI 미시작과 종료 코드 3 확인 (TC-UI-015, SRS-CMP-003).
# 실행: uv run tests/pty/no_dsr.py target/debug/howtui

# %% 설정
import os
import pty
import select
import sys
import time

binary = sys.argv[1] if len(sys.argv) > 1 else "target/debug/howtui"
env = {"PATH": "/usr/bin:/bin", "HOME": "/tmp", "TERM": "xterm-256color", "HOWTUI_LANG": "en", "HOWTUI_MOCK_SCENARIO": "echo"}

# %% 실행: 출력은 읽기만 하고 질의에 응답하지 않음
start = time.monotonic()
pid, fd = pty.fork()
if pid == 0:
    os.execve(binary, [binary], env)
output = b""
status = None
while True:
    ready, _, _ = select.select([fd], [], [], 0.2)
    if ready:
        try:
            chunk = os.read(fd, 4096)
        except OSError:
            break
        if not chunk:
            break
        output += chunk
    done, st = os.waitpid(pid, os.WNOHANG)
    if done:
        status = st
        break
    if time.monotonic() - start > 10:
        os.kill(pid, 9)
        sys.exit("FAIL no_dsr: did not exit within 10 s")
if status is None:
    _, status = os.waitpid(pid, 0)
elapsed = time.monotonic() - start

# %% 판정
code = os.waitstatus_to_exitcode(status)
asked = b"\x1b[6n" in output
print(f"exit={code} elapsed={elapsed:.2f}s cursor_query_sent={asked}")
if code != 3 or not asked or elapsed > 5:
    sys.exit("FAIL no_dsr")
print("PASS no_dsr")
