# 로컬 LLM 평가 (TC-LLM-021, research/local-llm.md 6절 계획). 실행: uv run --no-project python eval/local_llm/run_eval.py
# 사전 조건: 서버 컨테이너 실행 (start_servers.sh), howtui 릴리스 빌드, 평가 이미지 (docker build -t howtui-eval-box:1 eval/local_llm)
# 생성 명령어는 네트워크 없는 판정 컨테이너 안에서만 실행. 위험 요청 과제는 실행하지 않고 응답만 분류.

# %% 설정
import csv
import json
import os
import statistics
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import evallib  # noqa: E402

ROOT = HERE.parent.parent
BINARY = str(ROOT / "target" / "release" / "howtui")
RESULTS = HERE / "results"
REPEATS = int(os.environ.get("EVAL_REPEATS", "3"))
ONLY = os.environ.get("EVAL_CONFIGS", "").split(",") if os.environ.get("EVAL_CONFIGS") else None
CONFIGS = [
    dict(name="llama-cpu-e2b", provider="llama_server", base="http://127.0.0.1:8080/v1", container="howtui-llama-cpu",
         model="Gemma 4 E2B QAT Q4_0", server="llama-server", device="CPU (8 threads)"),
    dict(name="llama-cpu-e4b", provider="llama_server", base="http://127.0.0.1:18081/v1", container="howtui-llama-cpu-e4b",
         model="Gemma 4 E4B QAT Q4_0", server="llama-server", device="CPU (8 threads)"),
    dict(name="ollama-cpu-e2b", provider="ollama", base="http://127.0.0.1:11434/v1", container="howtui-ollama",
         model="gemma4:e2b-it-qat", ollama_model="gemma4:e2b-it-qat", server="Ollama", device="CPU"),
    dict(name="ollama-cpu-e4b", provider="ollama", base="http://127.0.0.1:11434/v1", container="howtui-ollama",
         model="gemma4:e4b-it-qat", ollama_model="gemma4:e4b-it-qat", server="Ollama", device="CPU"),
    # 클라우드 기준선: 사용자 ChatGPT 로그인, 설정 화면 선택 모델 (EVAL_CONFIGS=chatgpt, EVAL_TAG=chatgpt로 별도 실행)
    dict(name="chatgpt", provider="chatgpt", base=None, container=None, model="ChatGPT (selected model)",
         server="ChatGPT plan", device="cloud", user_login=True),
]
# 소형 모델 비교 (EVAL_OLLAMA_MODELS="qwen3.5:4b,phi4-mini" 형식, 모두 Ollama CPU 컨테이너)
for tag in filter(None, os.environ.get("EVAL_OLLAMA_MODELS", "").split(",")):
    CONFIGS.append(dict(name="ollama-" + tag.replace(":", "-"), provider="ollama", base="http://127.0.0.1:11434/v1",
                        container="howtui-ollama", model=tag, ollama_model=tag, server="Ollama", device="CPU"))
if ONLY:
    CONFIGS = [c for c in CONFIGS if c["name"] in ONLY]
# 위험 요청 세트만 재실행 (규칙 변경 후 분류 재측정, CR-012)
DANGER_ONLY = os.environ.get("EVAL_DANGER_ONLY") == "1"
SUFFIX = "_danger" if DANGER_ONLY else ""
if os.environ.get("EVAL_TAG"):
    SUFFIX += "_" + os.environ["EVAL_TAG"]
RESULTS.mkdir(exist_ok=True)

# %% 과제 로드
TASKS = json.loads((HERE / "tasks.json").read_text())
EXEC_TASKS = [t for t in TASKS if t["mode"] != "danger"]
DANGER_TASKS = [t for t in TASKS if t["mode"] == "danger"]
if DANGER_ONLY:
    TASKS = DANGER_TASKS
print(len(EXEC_TASKS), "executed tasks,", len(DANGER_TASKS), "danger tasks")

# %% 판정 컨테이너와 기준 명령 결과 (기준 명령이 판정을 통과하는지 자체 확인)
evallib.start_box()
REF = {t["id"]: evallib.run_in_box(t["shell"], t["ref"], t.get("post", "")) for t in EXEC_TASKS}
REF_FAIL = [tid for tid, r in REF.items() if not evallib.judge(next(t for t in EXEC_TASKS if t["id"] == tid), r, r) or r[1] != 0]
print("reference failures:", REF_FAIL)
assert not REF_FAIL, REF_FAIL

# %% 생성과 판정 (설정 x 반복 x 과제)
def config_file(cfg, data_dir):
    path = Path(data_dir) / "config.toml"
    section = cfg["provider"]
    if not cfg.get("base"):
        path.write_text("")
        return str(path)
    lines = [f"[provider.{section}]", f'base_url = "{cfg["base"]}"']
    if cfg.get("ollama_model"):
        lines.append(f'model = "{cfg["ollama_model"]}"')
    path.write_text("\n".join(lines) + "\n")
    return str(path)

# 진행 중 결과는 행마다 파일에 기록, 중단 후 재실행 시 완료 행 재사용
FIELDS = ["config", "rep", "task", "shell", "mode", "seconds", "howtui_rc", "command", "high_risk", "caution",
          "model_note", "cold_seconds", "passed", "syntax_ok", "exec_rc"]
PARTIAL = RESULTS / f"runs{SUFFIX}.partial.csv"
MEM_PATH = RESULTS / f"memory{SUFFIX}.json"

def parse_row(r):
    r = dict(r)
    for k in ("high_risk", "caution", "model_note"):
        r[k] = r[k] == "True"
    for k in ("passed", "syntax_ok"):
        r[k] = "" if r[k] == "" else r[k] == "True"
    r["rep"], r["howtui_rc"] = int(r["rep"]), int(r["howtui_rc"])
    r["seconds"], r["cold_seconds"] = float(r["seconds"]), float(r["cold_seconds"])
    return r

ROWS = [parse_row(r) for r in csv.DictReader(open(PARTIAL))] if PARTIAL.exists() else []
DONE = {(r["config"], r["rep"], r["task"]) for r in ROWS}
MEMORY = json.loads(MEM_PATH.read_text()) if MEM_PATH.exists() else {}
new_file = not PARTIAL.exists()
out = open(PARTIAL, "a", newline="")
writer = csv.DictWriter(out, fieldnames=FIELDS)
if new_file:
    writer.writeheader()
for cfg in CONFIGS:
    if all((cfg["name"], rep, t["id"]) in DONE for rep in range(REPEATS) for t in TASKS):
        continue
    conf_dir = tempfile.mkdtemp(prefix="howtui-eval-")
    conf = config_file(cfg, conf_dir)
    data_dir = None if cfg.get("user_login") else conf_dir
    # 첫 요청 전 적재 (콜드 시간은 별도 기록)
    _, _, _, cold = evallib.howtui(BINARY, cfg["provider"], "bash", "print hello", conf, data_dir)
    with evallib.MemSampler(cfg["container"]) as mem:
        for rep in range(REPEATS):
            for t in TASKS:
                if (cfg["name"], rep, t["id"]) in DONE:
                    continue
                cmd, err, rc, sec = evallib.howtui(BINARY, cfg["provider"], t["shell"], t["q"], conf, data_dir)
                row = dict(config=cfg["name"], rep=rep, task=t["id"], shell=t["shell"], mode=t["mode"], seconds=round(sec, 3),
                           howtui_rc=rc, command=cmd, high_risk="High risk" in err, caution="Caution:" in err,
                           model_note="Notes:" in err, cold_seconds=round(cold, 3))
                if t["mode"] == "danger":
                    row.update(passed="", syntax_ok="", exec_rc="")
                elif rc != 0 or not cmd:
                    row.update(passed=False, syntax_ok=False, exec_rc="")
                else:
                    cand = evallib.run_in_box(t["shell"], cmd, t.get("post", ""))
                    row.update(passed=evallib.judge(t, cand, REF[t["id"]]), syntax_ok=cand[3], exec_rc=cand[1])
                ROWS.append(row)
                writer.writerow(row)
                out.flush()
            print(cfg["name"], "rep", rep, "done", time.strftime("%H:%M:%S"), flush=True)
    if not cfg["container"]:
        MEMORY[cfg["name"]] = 0
    else:
        MEMORY[cfg["name"]] = max(MEMORY.get(cfg["name"], 0), round(mem.peak_mib, 1))
    MEM_PATH.write_text(json.dumps(MEMORY))
out.close()

with open(RESULTS / f"runs{SUFFIX}.csv", "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=FIELDS)
    w.writeheader()
    w.writerows(ROWS)
PARTIAL.unlink()
evallib.stop_box()

# %% 집계 (summary.json)
def rate(xs):
    return round(100 * sum(xs) / len(xs), 1) if xs else None

SUMMARY = {"repeats": REPEATS, "tasks": {"executed": len(EXEC_TASKS), "danger": len(DANGER_TASKS)}, "configs": []}
for cfg in CONFIGS:
    rows = [r for r in ROWS if r["config"] == cfg["name"]]
    ex = [r for r in rows if r["mode"] != "danger"]
    dg = [r for r in rows if r["mode"] == "danger"]
    per_shell = {}
    for sh in ("bash", "zsh", "powershell"):
        s = [r for r in ex if r["shell"] == sh]
        per_shell[sh] = {"accuracy": rate([r["passed"] for r in s]), "syntax_error": rate([not r["syntax_ok"] for r in s if r["howtui_rc"] == 0 and r["command"]]),
                         "no_command": rate([r["howtui_rc"] != 0 or not r["command"] for r in s]), "n": len(s)}
    secs = [r["seconds"] for r in rows]
    SUMMARY["configs"].append({
        **{k: cfg[k] for k in ("name", "model", "server", "device")},
        "per_shell": per_shell,
        "accuracy_all": rate([r["passed"] for r in ex]),
        "seconds_median": round(statistics.median(secs), 2) if secs else None,
        "seconds_p90": round(sorted(secs)[int(0.9 * (len(secs) - 1))], 2) if secs else None,
        "cold_seconds": rows[0]["cold_seconds"],
        "peak_memory_mib": MEMORY[cfg["name"]],
        "danger": {"no_command": rate([r["howtui_rc"] != 0 or not r["command"] for r in dg]),
                   "howtui_high_risk": rate([r["high_risk"] for r in dg]),
                   "model_note": rate([r["model_note"] for r in dg]), "n": len(dg)},
        "normal_high_risk": rate([r["high_risk"] for r in ex]),
        "per_task_pass": {t["id"]: rate([r["passed"] for r in ex if r["task"] == t["id"]]) for t in EXEC_TASKS},
    })
(RESULTS / f"summary{SUFFIX}.json").write_text(json.dumps(SUMMARY, ensure_ascii=False, indent=1))
print(json.dumps({c["name"]: (c["accuracy_all"], c["seconds_median"]) for c in SUMMARY["configs"]}))
