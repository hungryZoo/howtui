"""로컬 LLM 평가 공통 함수 (TC-LLM-021): 격리 컨테이너 실행, 출력 정규화, 판정, 메모리 표본."""

import os
import re
import subprocess
import threading
import time

BOX = "howtui-eval-judge"
IMAGE = "howtui-eval-box:1"
JUDGE_SHELL = {"bash": "bash", "zsh": "zsh", "powershell": "pwsh"}


def start_box():
    """네트워크 없는 판정 컨테이너 시작 (이미 있으면 재사용)."""
    running = subprocess.run(["docker", "ps", "-q", "-f", f"name=^{BOX}$"], capture_output=True, text=True).stdout.strip()
    if running:
        return
    subprocess.run(["docker", "rm", "-f", BOX], capture_output=True)
    subprocess.run(
        ["docker", "run", "-d", "--name", BOX, "--network", "none", "--memory", "1g", "--pids-limit", "256",
         IMAGE, "sleep", "infinity"],
        check=True, capture_output=True,
    )


def stop_box():
    subprocess.run(["docker", "rm", "-f", BOX], capture_output=True)


ANSI = re.compile(r"\x1b\[[0-9;?]*[ -/]*[@-~]")


def run_in_box(shell, command, post=""):
    """고정 파일 복사본에서 명령 실행. (stdout, rc, post 출력, 구문 검사 통과 여부, stderr)."""
    p = subprocess.run(
        ["docker", "exec", "-i", BOX, "bash", "/opt/eval/judge.sh", JUDGE_SHELL[shell], post or "true"],
        input=command, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=120,
    )
    # 색 강조 제거 (pwsh 7.4 Select-String은 리디렉션 출력에도 강조 시퀀스 포함)
    out = ANSI.sub("", p.stdout)
    body, _, rest = out.partition("\n@@RC ")
    rc_text, _, rest = rest.partition("\n@@POST\n")
    post_out, _, rest = rest.partition("@@SYNTAX\n")
    syntax, _, stderr = rest.partition("@@STDERR\n")
    try:
        rc = int(rc_text.strip())
    except ValueError:
        rc = -1
    return body, rc, post_out, syntax.strip().endswith("ok"), stderr


def lines(text):
    out = []
    for line in text.splitlines():
        s = line.strip()
        if s.startswith("./"):
            s = s[2:]
        if s:
            out.append(s)
    return out


def number(text):
    m = re.search(r"-?\d+", text)
    return int(m.group()) if m else None


def tokens_in_order(text, expect):
    pos = 0
    for tok in expect:
        i = text.find(tok, pos)
        if i < 0:
            return False
        pos = i + len(tok)
    return True


def judge(task, cand, ref):
    """cand, ref: run_in_box 결과. 출력 비교 방식은 task['mode']."""
    c_out, c_rc, c_post, _, _ = cand
    r_out, _, r_post, _, _ = ref
    mode = task["mode"]
    if task.get("post") and c_post != r_post:
        return False
    if mode == "post_only":
        return True
    if mode == "number":
        return number(c_out) is not None and number(c_out) == number(r_out)
    if mode == "lines_set":
        return set(lines(c_out)) == set(lines(r_out))
    if mode == "lines_seq":
        return lines(c_out) == lines(r_out)
    if mode == "basenames":
        base = lambda t: {os.path.basename(x.rstrip("/")) for x in lines(t)}
        return base(c_out) == base(r_out)
    if mode == "contains_all":
        want = lines(r_out)
        got = lines(c_out)
        return len(got) == len(want) and all(any(w in g for g in got) for w in want)
    if mode == "tokens":
        return all(t in c_out for t in task["expect"]) and not any(t in c_out for t in task.get("forbid", []))
    if mode == "tokens_order":
        return tokens_in_order(c_out, task["expect"])
    raise ValueError(mode)


class MemSampler:
    """서버 컨테이너 메모리 최댓값 표본 (docker stats, 2초 간격)."""

    def __init__(self, container):
        self.container = container
        self.peak_mib = 0.0
        self._stop = threading.Event()
        self._t = threading.Thread(target=self._run, daemon=True)

    def _run(self):
        if not self.container:
            return
        while not self._stop.is_set():
            p = subprocess.run(["docker", "stats", "--no-stream", "--format", "{{.MemUsage}}", self.container],
                               capture_output=True, text=True)
            used = p.stdout.split("/")[0].strip()
            m = re.match(r"([\d.]+)\s*([KMG]i?B)", used)
            if m:
                v = float(m.group(1)) * {"KiB": 1 / 1024, "KB": 1 / 1024, "MiB": 1, "MB": 1, "GiB": 1024, "GB": 1024}[m.group(2)]
                self.peak_mib = max(self.peak_mib, v)
            time.sleep(2)

    def __enter__(self):
        self._t.start()
        return self

    def __exit__(self, *exc):
        self._stop.set()
        self._t.join(timeout=5)


def howtui(binary, provider, shell, question, config_path, data_dir):
    """비대화형 1회 생성. (명령어, stderr, 종료 코드, 경과 초)."""
    if data_dir is None:
        # 사용자 로그인 사용 (ChatGPT 기준선): 보안 저장소 접근에 필요한 세션 환경 유지, 데이터 디렉터리 기본값
        env = {k: v for k, v in os.environ.items() if not k.startswith("HOWTUI_")}
        env.update({"HOWTUI_CONFIG": config_path, "HOWTUI_LANG": "en", "HOWTUI_NO_UPDATE_CHECK": "1"})
    else:
        env = {
            "PATH": "/usr/bin:/bin",
            "HOME": data_dir,
            "HOWTUI_DATA_DIR": data_dir,
            "HOWTUI_CONFIG": config_path,
            "HOWTUI_LANG": "en",
            "HOWTUI_NO_UPDATE_CHECK": "1",
        }
    t0 = time.monotonic()
    p = subprocess.run([binary, "--no-tui", "--provider", provider, "--shell", shell, question],
                       capture_output=True, text=True, encoding="utf-8", errors="replace", env=env, timeout=400)
    return p.stdout.rstrip("\n"), p.stderr, p.returncode, time.monotonic() - t0
