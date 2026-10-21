#!/usr/bin/env bash
# SPIKE-01 검증 스크립트. tmux 창에서 실험 바이너리를 실제 PTY로 구동하고 결과를 logs/ 아래에 저장한다.
# 사용: spikes/inline-spike/check.sh  (저장소 루트 기준, tmux 세션 howtui:spike 필요)
set -u

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN="$ROOT/spikes/inline-spike/target/debug/inline-spike"
OUT="$ROOT/logs/spike01_$(date +%Y%m%d-%H%M%S)"
PANE=howtui:spike
mkdir -p "$OUT"

send() { tmux send-keys -t "$PANE" "$@"; }
type_text() { tmux send-keys -t "$PANE" -l "$1"; }
# snap 패키지 tmux는 파일로 직접 리디렉션한 stdout에 쓰지 못하므로 파이프를 거친다.
snap() { tmux capture-pane -p -t "$PANE" | cat > "$OUT/$1.screen"; }
pause() { sleep "$1"; }

# 명령 실행 후 <case>.done 파일 생성까지 대기
run_case() {
  local name=$1 cmd=$2
  send C-c; pause 0.2
  send "clear" Enter; pause 0.3
  type_text "$cmd; echo \"rc=\$?\" > $OUT/$name.rc; stty -a | tr ' ;' '\\n\\n' | grep -E '^-?(icanon|echo|isig)\$' | tr '\\n' ' ' > $OUT/$name.stty; touch $OUT/$name.done"
  send Enter
}

wait_done() {
  local name=$1 i
  for i in $(seq 1 100); do
    [ -f "$OUT/$name.done" ] && return 0
    pause 0.1
  done
  echo "TIMEOUT $name" >> "$OUT/summary.txt"
}

tmux resize-window -t "$PANE" -x 100 -y 30

# (1) --naive: stderr 백엔드만 사용, stdout 파이프
run_case naive_pipe "$BIN --naive 2>$OUT/naive_pipe.err | od -c > $OUT/naive_pipe.out"
pause 4; snap naive_pipe; send C-c; wait_done naive_pipe

# (2) fd 1 전환 + stdout 파이프
run_case pipe "$BIN 2>$OUT/pipe.err | od -c > $OUT/pipe.out"
pause 0.8; type_text "list files"; send Enter; pause 0.4; snap pipe_loading
pause 2; snap pipe_result; send Enter; wait_done pipe; snap pipe_after

# (3) 명령 치환 캡처
run_case capture "x=\$($BIN 2>$OUT/capture.err); printf '%s' \"\$x\" > $OUT/capture.raw; printf '%s' \"\$x\" | od -c > $OUT/capture.out; (exit 0)"
pause 0.8; type_text "q"; send Enter; pause 2.2; send Enter; wait_done capture

# (4) 파일 리디렉션
run_case file "$BIN > $OUT/file.out 2>$OUT/file.err"
pause 0.8; type_text "q"; send Enter; pause 2.2; send Enter; wait_done file

# (5) Ctrl+C 취소
run_case cancel "$BIN > $OUT/cancel.out 2>$OUT/cancel.err"
pause 0.8; type_text "q"; send C-c; wait_done cancel

# (6) 요청 중 Esc 취소, 결과 미반영 확인
run_case esc "$BIN > $OUT/esc.out 2>$OUT/esc.err"
pause 0.8; type_text "q"; send Enter; pause 0.4; send Escape; pause 0.3; snap esc_cancelled
pause 2; snap esc_after_delay; send C-c; wait_done esc

# (7) 여러 줄 편집 (Alt+Enter, Ctrl+J) 및 Unicode 질문 입력
run_case multiline "$BIN > $OUT/multiline.out 2>$OUT/multiline.err"
pause 0.8; type_text "한글 질문 😀"; snap multiline_question; send Enter; pause 2.2
send M-Enter; type_text "echo 끝"; send C-j; type_text "# 주석"; pause 0.3; snap multiline_edit
send Enter; wait_done multiline

# (8) 실행 중 리사이즈
run_case resize "$BIN > $OUT/resize.out 2>$OUT/resize.err"
pause 0.8; type_text "q"; send Enter; pause 2.2
tmux resize-window -t "$PANE" -x 60 -y 30; pause 0.6; snap resize_narrow
tmux resize-window -t "$PANE" -x 100 -y 12; pause 0.6; snap resize_short
tmux resize-window -t "$PANE" -x 100 -y 30; pause 0.6; snap resize_restored
send Enter; wait_done resize

# (9) panic 시 터미널 복구
run_case panic "$BIN --panic > $OUT/panic.out 2>$OUT/panic.err"
wait_done panic

# (10) 제어 터미널 없음
run_case notty "setsid -w $BIN < /dev/null > $OUT/notty.out 2> $OUT/notty.err"
wait_done notty

# (11) stdin 파이프 + /dev/tty 사용 가능
run_case stdin_pipe "echo from-pipe | $BIN > $OUT/stdin_pipe.out 2>$OUT/stdin_pipe.err"
pause 0.8; type_text "q"; send Enter; pause 2.2; send Enter; wait_done stdin_pipe

# (12) 터미널 높이가 Inline 높이보다 작은 경우
tmux resize-window -t "$PANE" -x 40 -y 5
run_case tiny "$BIN > $OUT/tiny.out 2>$OUT/tiny.err"
pause 0.8; type_text "q"; send Enter; pause 2.2; snap tiny; send Enter; wait_done tiny
tmux resize-window -t "$PANE" -x 100 -y 30

# 요약
{
  for f in "$OUT"/*.rc; do
    n=$(basename "$f" .rc)
    printf '%-12s %s stty=[%s] stdout_bytes=%s esc_in_stdout=%s\n' "$n" "$(cat "$f")" \
      "$(cat "$OUT/$n.stty" 2>/dev/null)" \
      "$(wc -c < "$OUT/$n.out" 2>/dev/null || echo -)" \
      "$(grep -c '033' "$OUT/$n.out" 2>/dev/null || echo -)"
  done
} >> "$OUT/summary.txt"
echo "$OUT"
