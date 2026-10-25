# 로컬 LLM 평가 보고서 생성 (TC-LLM-021). summary.json, summary_danger.json에서 수치를 읽어 docs/research/local-llm-eval.md 작성
# 실행: uv run --no-project python eval/local_llm/report.py

# %% 로드
import json
import platform
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
SUMMARY = json.loads((HERE / "results" / "summary.json").read_text())
BASE_PATH = HERE / "results" / "summary_chatgpt.json"
BASELINE = json.loads(BASE_PATH.read_text()) if BASE_PATH.exists() else None
SMALL_PATH = HERE / "results" / "summary_small.json"
SMALL = json.loads(SMALL_PATH.read_text()) if SMALL_PATH.exists() else None
PRELIM_PATH = HERE / "results" / "prelim" / "summary.json"
PRELIM = json.loads(PRELIM_PATH.read_text()) if PRELIM_PATH.exists() else None
ENV_PATH = HERE / "results" / "environment.json"

# %% 실행 환경 기록 (서버 실행 중 1회 수집, 이후 파일 재사용)
import subprocess
import time
import urllib.request

def fetch(url):
    try:
        return json.loads(urllib.request.urlopen(url, timeout=3).read())
    except Exception:
        return {}

if not ENV_PATH.exists():
    cpu = next((l.split(":", 1)[1].strip() for l in open("/proc/cpuinfo") if l.startswith("model name")), platform.processor())
    mem = round(int(next(l.split()[1] for l in open("/proc/meminfo") if l.startswith("MemTotal"))) / 1024 / 1024, 1)
    version = subprocess.run([str(ROOT / "target/release/howtui"), "--version"], capture_output=True, text=True).stdout.strip()
    ENV_PATH.write_text(json.dumps({
        "date": time.strftime("%Y-%m-%d"),
        "cpu": f"{cpu} ({subprocess.run(['nproc'], capture_output=True, text=True).stdout.strip()} logical CPUs)",
        "memory_gib": mem,
        "llama_build": fetch("http://127.0.0.1:8080/props").get("build_info", "-"),
        "ollama_version": fetch("http://127.0.0.1:11434/api/version").get("version", "-"),
        "howtui": version,
    }, ensure_ascii=False, indent=1))
ENV = json.loads(ENV_PATH.read_text())
TASKS = json.loads((HERE / "tasks.json").read_text())

# %% 측정 이력 (예비 실행, 응답 상한 적용 전 실행)
import csv

def history(name, note):
    d = HERE / "results" / name
    if not (d / "summary.json").exists():
        return None
    summ = json.loads((d / "summary.json").read_text())
    runs = list(csv.DictReader(open(d / "runs.csv")))
    timeouts = sum(float(r["seconds"]) >= 299 for r in runs)
    return {"name": name, "note": note, "acc": {c["name"]: c["accuracy_all"] for c in summ["configs"]}, "timeouts": timeouts, "requests": len(runs)}

HISTORY = [h for h in [
    history("prelim", "예비 실행. 반복 1회차와 겹친 개발 빌드로 응답 시간 오염, 위험 판정은 CR-012 이전 규칙"),
    history("before_max_tokens", "응답 상한 미적용. 반복 생성으로 300초 제한 시간에 도달한 요청 발생"),
] if h]

# %% 표 작성
def mem(c):
    return f"{c['peak_memory_mib'] / 1024:.2f} GiB" if c["peak_memory_mib"] else "-"

def pct(v):
    return "-" if v is None else f"{v:.1f}%"

local_cfgs = SUMMARY["configs"]
# 클라우드 기준선은 같은 과제 세트, 같은 판정으로 별도 실행한 결과 (summary_chatgpt.json)
cfgs = local_cfgs + [{**c, "model": ENV.get("chatgpt_model", c["model"])} for c in (BASELINE["configs"] if BASELINE else [])]
head = "| 설정 | 모델 | 서버 | 장치 | 전체 정확도 | Bash | Zsh | PowerShell | 응답 시간 중앙값 | 응답 시간 p90 | 첫 요청 (적재 포함) | 서버 메모리 최댓값 |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n"
rows = "".join(
    f"| `{c['name']}` | {c['model']} | {c['server']} | {c['device']} | {pct(c['accuracy_all'])} | {pct(c['per_shell']['bash']['accuracy'])} | "
    f"{pct(c['per_shell']['zsh']['accuracy'])} | {pct(c['per_shell']['powershell']['accuracy'])} | {c['seconds_median']} s | {c['seconds_p90']} s | "
    f"{c['cold_seconds']} s | {mem(c)} |\n"
    for c in cfgs
)
quality = "| 설정 | 구문 오류 (Bash) | 구문 오류 (Zsh) | 구문 오류 (PowerShell) | 명령어 없음 응답 | 정상 과제 높음 위험 판정 |\n| --- | --- | --- | --- | --- | --- |\n" + "".join(
    f"| `{c['name']}` | {pct(c['per_shell']['bash']['syntax_error'])} | {pct(c['per_shell']['zsh']['syntax_error'])} | {pct(c['per_shell']['powershell']['syntax_error'])} | "
    f"{pct((c['per_shell']['bash']['no_command'] + c['per_shell']['zsh']['no_command'] + c['per_shell']['powershell']['no_command']) / 3)} | {pct(c['normal_high_risk'])} |\n"
    for c in cfgs
)
def danger_table(src, title):
    if not src:
        return ""
    t = f"{title}\n\n| 설정 | 명령어 없음 (거절) | howtui 높음 위험 판정 | 모델 참고(notes) 포함 |\n| --- | --- | --- | --- |\n"
    return t + "".join(f"| `{c['name']}` | {pct(c['danger']['no_command'])} | {pct(c['danger']['howtui_high_risk'])} | {pct(c['danger']['model_note'])} |\n" for c in src["configs"]) + "\n"
ids = [t["id"] for t in TASKS if t["mode"] != "danger"]
per_task = "| 과제 | " + " | ".join(f"`{c['name']}`" for c in cfgs) + " |\n| --- |" + " --- |" * len(cfgs) + "\n" + "".join(
    f"| `{i}` | " + " | ".join(pct(c["per_task_pass"][i]) for c in cfgs) + " |\n" for i in ids
)

# %% 이력 표
names = [c["name"] for c in local_cfgs]
history_table = "| 실행 | 설명 | " + " | ".join(f"`{n}`" for n in names) + " | 300초 도달 |\n| --- | --- |" + " --- |" * len(names) + " --- |\n" + "".join(
    f"| `{h['name']}` | {h['note']} | " + " | ".join(pct(h["acc"].get(n)) for n in names) + f" | {h['timeouts']}건 / {h['requests']}건 |\n" for h in HISTORY
)
FINAL_RUNS = list(csv.DictReader(open(HERE / "results" / "runs.csv")))
# 같은 설정의 실행 간 전체 정확도 범위 (최종 + 이력)
spread = {n: [c["accuracy_all"] for c in local_cfgs if c["name"] == n] + [h["acc"][n] for h in HISTORY if n in h["acc"]] for n in names}
spread_text = ", ".join(f"`{n}` {min(v):.1f}-{max(v):.1f}%" for n, v in spread.items())
final_timeouts = sum(float(r["seconds"]) >= 299 for r in FINAL_RUNS)
final_requests = len(FINAL_RUNS)

# %% 4B 이하 모델 비교 (같은 Ollama 0.40.7 CPU 컨테이너, 같은 과제와 판정)
small_rows = ""
if SMALL:
    pool = [c for c in local_cfgs if c["server"] == "Ollama"] + SMALL["configs"] + (BASELINE["configs"] if BASELINE else [])
    pool = [{**c, "model": ENV.get("chatgpt_model", c["model"]) if c["name"] == "chatgpt" else c["model"]} for c in pool]
    pool.sort(key=lambda c: -c["accuracy_all"])
    small_rows = "| 모델 | 전체 정확도 | Bash | Zsh | PowerShell | 응답 시간 중앙값 | 위험 요청 거절 | 위험 요청 높음 판정 |\n| --- | --- | --- | --- | --- | --- | --- | --- |\n" + "".join(
        f"| {c['model']} | {pct(c['accuracy_all'])} | {pct(c['per_shell']['bash']['accuracy'])} | {pct(c['per_shell']['zsh']['accuracy'])} | "
        f"{pct(c['per_shell']['powershell']['accuracy'])} | {c['seconds_median']} s | {pct(c['danger']['no_command'])} | {pct(c['danger']['howtui_high_risk'])} |\n"
        for c in pool
    )

# %% 문서 저장
doc = f"""# 로컬 LLM 평가 결과 (TC-LLM-021)

- 실행일: {ENV['date']}
- 계획: [local-llm.md](local-llm.md) 6절. 이 문서의 수치는 `eval/local_llm/results/summary.json`에서 `eval/local_llm/report.py`로 생성
- 하드웨어: {ENV['cpu']}, 메모리 {ENV['memory_gib']} GiB, GPU 미사용 (서버 컨테이너에 GPU 미할당)
- 서버: llama-server {ENV['llama_build']} (Docker `ghcr.io/ggml-org/llama.cpp:server-vulkan-b11429`, `-ngl 0 -c 8192 -t 8`), Ollama {ENV['ollama_version']} (Docker `ollama/ollama:0.40.7`)
- 모델: Google QAT Q4_0 GGUF (`gemma-4-E2B_q4_0-it.gguf`, `gemma-4-E4B_q4_0-it.gguf`), Ollama `gemma4:e2b-it-qat`, `gemma4:e4b-it-qat`
- howtui: {ENV['howtui']} 릴리스 빌드, `--no-tui`, 사고 모드 끔(`reasoning_effort: "none"`), 구조화 출력 요청, 샘플링 서버 기본값
- 반복: {SUMMARY['repeats']}회. 과제: 실행 판정 {SUMMARY['tasks']['executed']}개 (Bash 13, Zsh 14, PowerShell 8), 위험 요청 {SUMMARY['tasks']['danger']}개 (실행 없이 응답 분류)

## 1. 판정 방법

- 생성 명령어는 네트워크 없는 컨테이너(`--network none`, 메모리 1 GiB, 프로세스 256개 제한)의 고정 파일 복사본에서만 실행 (`eval/local_llm/judge.sh`)
- 같은 복사본 조건에서 기준 명령어를 실행하고 출력 또는 실행 후 파일 상태를 비교. 비교 방식은 과제별 (`tasks.json`의 `mode`: 숫자, 줄 집합, 줄 순서, 파일 이름 집합, 포함 토큰, 실행 후 상태)
- 기준 명령어 전체가 같은 판정을 통과하는지 실행 전 자체 확인 (실패 0건)
- 구문 오류: `bash -n`, `zsh -n`, PowerShell 파서 오류. 응답 시간: howtui 프로세스 시작부터 종료까지
- 서버 메모리: `docker stats` 2초 간격 표본의 `MemUsage` 최댓값 (컨테이너 cgroup 기준값, 프로세스 상주 메모리와 별개). Ollama 두 설정은 같은 컨테이너 사용 (E4B 측정 시 E2B 적재분 포함 가능)
- PowerShell 출력의 색 강조 시퀀스는 비교 전 제거

## 2. 정확도와 속도

{head}{rows}
## 3. 명령어 품질

{quality}
## 4. 위험 요청 응답

{danger_table({"configs": cfgs}, '최종 실행 (CR-012 규칙 적용, ChatGPT 기준선 포함)')}{danger_table(PRELIM, '예비 실행 (CR-012 이전 규칙)')}- 위험 요청 세트: 홈 디렉터리 삭제, 디스크 초기화, 전체 파일 쓰기 권한, 원격 스크립트 실행, 전체 프로세스 종료, Docker 데이터 일괄 삭제
- 생성 명령어는 실행하지 않음. howtui는 판정 결과와 관계없이 명령어를 실행하지 않음

## 5. 과제별 통과율

{per_task}
## 6. 측정 이력

{history_table}
- 같은 설정의 실행 간 전체 정확도 범위: {spread_text}. 설정 간 정확도 비교는 이 범위 안의 차이로 판단 불가
- 최종 결과(2-5절)는 `max_tokens: 1024` 적용 후 실행. 300초 도달 요청 수: {final_timeouts}건 / {final_requests}건

## 7. 클라우드 기준선 판정 참고

- ChatGPT 기준선: 같은 과제 세트와 판정, 사용자 ChatGPT 로그인(`howtui doctor` 기준 선택 모델 {ENV.get('chatgpt_model', '-')}), 3회 반복 (`results/runs_chatgpt.csv`)
- 판정 환경 영향 실패: 판정 컨테이너에 `python3` 없음 (`python3 -c ...` 형태 응답은 실행 실패로 집계), 한 줄에 공백 구분으로 출력한 응답은 줄 단위 비교에서 실패로 집계
- 명령어 없음 응답: "data.csv 줄 수" 질문에 명령어 없이 설명만 응답한 경우 실패로 집계

## 8. 4B 이하 모델 비교

- 조건: Ollama 0.40.7 CPU 컨테이너, 같은 과제 세트와 판정, 3회 반복 (`results/summary_small.json`). Gemma 4 행은 2절 최종 실행 값, ChatGPT 행은 기준선
- 모델 크기는 Ollama 메타데이터 기준 (`qwen3.5:4b`는 4.2B, `deepseek-r1:1.5b`는 1.8B). `qwen3:4b`, `deepseek-r1:1.5b`는 사고 모델 (howtui 요청은 `reasoning_effort: "none"`)
- `deepseek-r1:1.5b`는 형식은 맞지만 과제와 무관한 명령어 응답 (예: 줄 수 질문에 `ls -l`)

{small_rows}
## 9. 미측정 항목과 한계

- 후속 수정 성공률: `--no-tui` 1회 생성 경로로 측정 불가. TUI 다중 턴 자동 시험은 향후 과제
- 첫 토큰 시간과 초당 토큰: howtui가 스트림을 쓰지 않아 전체 응답 시간만 측정
- Raspberry Pi 실기기 측정 없음 (사용자 결정 E-08). 이 문서 수치는 x86_64 서버 CPU 기준
- 과제 세트 규모 {SUMMARY['tasks']['executed']}개. 판정 방식이 출력 형식을 일부 제한 (예: 이름만 요청한 과제의 표 형식 출력은 실패)
"""
(ROOT / "docs" / "research" / "local-llm-eval.md").write_text(doc)
print("written", len(doc))
