# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
# 요구사항 추적성 검사: MRD -> PRD -> SRS -> TC 연결을 문서 표에서 읽어 검사하고
# docs/TRACEABILITY.md를 다시 생성한다. 문제가 있으면 종료 코드 1.
# 실행: uv run scripts/trace_check.py

# %% 경로와 문서 로드
import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"
texts = {name: (DOCS / f"{name}.md").read_text(encoding="utf-8") for name in ["MRD", "PRD", "SRS", "TC"]}

# %% 표 행 파싱 함수
CELL_SPLIT = re.compile(r"(?<!\\)\|")
ID_TOKEN = re.compile(r"(MRD|PRD|SRS|TC)-([A-Z][A-Z0-9]*)-(\d{3})(?:-(\d{3}))?")


def table_rows(text, prefix):
    """첫 열이 prefix-로 시작하는 표 행을 셀 목록으로 반환한다."""
    rows = []
    for line in text.splitlines():
        if not line.startswith(f"| {prefix}-"):
            continue
        cells = [c.strip() for c in CELL_SPLIT.split(line.strip())[1:-1]]
        rows.append(cells)
    return rows


def expand_ids(cell):
    """셀 안의 ID와 범위 표기(PRD-LLM-001-003)를 개별 ID 목록으로 펼친다."""
    ids = []
    for kind, area, start, end in ID_TOKEN.findall(cell):
        last = int(end) if end else int(start)
        ids.extend(f"{kind}-{area}-{n:03d}" for n in range(int(start), last + 1))
    return ids


# %% 문서별 ID 수집
mrd_ids = [r[0] for r in table_rows(texts["MRD"], "MRD")]
prd_rows = table_rows(texts["PRD"], "PRD")
srs_rows = table_rows(texts["SRS"], "SRS")
tc_rows = table_rows(texts["TC"], "TC")

prd = {r[0]: {"text": r[1], "phase": r[2], "priority": r[3], "mrd": expand_ids(r[4])} for r in prd_rows}
srs = {r[0]: {"prd": expand_ids(r[-2]), "impl": r[-1]} for r in srs_rows if len(r) >= 4}
# TC 표 열: ID, 목적, 관련 ID, 사전 조건, 절차, 기대 결과, 유형, 플랫폼, 결과, 이슈
tc = {r[0]: {"refs": expand_ids(r[2]), "type": r[6], "result": r[8]} for r in tc_rows if len(r) >= 10}

# %% 검사
errors = []
warnings = []

for name, rows in [("MRD", mrd_ids), ("PRD", [r[0] for r in prd_rows]), ("SRS", [r[0] for r in srs_rows]), ("TC", [r[0] for r in tc_rows])]:
    seen = set()
    for i in rows:
        if i in seen:
            errors.append(f"{name} 중복 ID: {i}")
        seen.add(i)

mrd_set = set(mrd_ids)
for pid, p in prd.items():
    if not p["mrd"]:
        errors.append(f"{pid}: MRD 근거 없음")
    for m in p["mrd"]:
        if m not in mrd_set:
            errors.append(f"{pid}: 존재하지 않는 MRD 참조 {m}")

prd_to_srs = defaultdict(list)
for sid, s in srs.items():
    if not s["prd"]:
        errors.append(f"{sid}: 관련 PRD 없음")
    for p in s["prd"]:
        if p not in prd:
            errors.append(f"{sid}: 존재하지 않는 PRD 참조 {p}")
        prd_to_srs[p].append(sid)

prd_to_tc = defaultdict(list)
srs_to_tc = defaultdict(list)
for tid, t in tc.items():
    if not t["refs"]:
        errors.append(f"{tid}: 관련 ID 없음")
    for ref in t["refs"]:
        if ref.startswith("PRD-"):
            if ref not in prd:
                errors.append(f"{tid}: 존재하지 않는 PRD 참조 {ref}")
            prd_to_tc[ref].append(tid)
        elif ref.startswith("SRS-"):
            if ref not in srs:
                errors.append(f"{tid}: 존재하지 않는 SRS 참조 {ref}")
            srs_to_tc[ref].append(tid)

for pid in prd:
    if not prd_to_srs[pid]:
        errors.append(f"{pid}: 연결된 SRS 없음")
    if not prd_to_tc[pid]:
        errors.append(f"{pid}: 연결된 TC 없음")

for sid in srs:
    if not srs_to_tc[sid]:
        warnings.append(f"{sid}: 연결된 TC 없음")

referenced_mrd = {m for p in prd.values() for m in p["mrd"]}
for m in mrd_ids:
    if m.startswith("MRD-SCP-") and m not in referenced_mrd:
        errors.append(f"{m}: PRD에서 참조되지 않은 MRD 범위 항목")

# %% 추적성 표 생성
def tc_result_summary(tc_ids):
    counts = defaultdict(int)
    for t in tc_ids:
        counts[tc[t]["result"]] += 1
    return ", ".join(f"{k} {v}" for k, v in sorted(counts.items())) or "-"


lines = [
    "# 요구사항 추적성 표",
    "",
    "- 생성: `uv run scripts/trace_check.py` (수동 편집 금지, 원본은 MRD, PRD, SRS, TC 문서 표)",
    "- 연결 구조: MRD -> PRD -> SRS -> TC -> 구현 위치 -> 테스트 결과",
    "",
    "## 1. 요약",
    "",
    "| 항목 | 값 |",
    "| --- | --- |",
    f"| MRD 항목 | {len(mrd_ids)} |",
    f"| PRD 요구사항 | {len(prd)} |",
    f"| SRS 설계 요구사항 | {len(srs)} |",
    f"| TC | {len(tc)} |",
    f"| 오류 (누락, 불일치) | {len(errors)} |",
    f"| 경고 (TC 미연결 SRS) | {len(warnings)} |",
    "",
]
if errors or warnings:
    lines += ["## 2. 검사 결과", ""]
    lines += [f"- 오류: {e}" for e in errors]
    lines += [f"- 경고: {w}" for w in warnings]
    lines.append("")
else:
    lines += ["## 2. 검사 결과", "", "- 오류와 경고 없음", ""]

lines += [
    "## 3. PRD 기준 추적성",
    "",
    "| PRD | 단계 | 우선순위 | MRD | SRS | TC | 구현 위치 | TC 결과 |",
    "| --- | --- | --- | --- | --- | --- | --- | --- |",
]
for pid, p in prd.items():
    impl = sorted({srs[s]["impl"] for s in prd_to_srs[pid]})
    lines.append(
        f"| {pid} | {p['phase']} | {p['priority']} | {', '.join(p['mrd'])} | "
        f"{', '.join(prd_to_srs[pid]) or '-'} | {', '.join(prd_to_tc[pid]) or '-'} | "
        f"{'<br>'.join(impl) or '-'} | {tc_result_summary(prd_to_tc[pid])} |"
    )
lines.append("")
(DOCS / "TRACEABILITY.md").write_text("\n".join(lines), encoding="utf-8")

# %% 결과 출력
print(f"MRD {len(mrd_ids)}, PRD {len(prd)}, SRS {len(srs)}, TC {len(tc)}")
for e in errors:
    print(f"오류: {e}")
for w in warnings:
    print(f"경고: {w}")
sys.exit(1 if errors else 0)
