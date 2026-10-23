#!/usr/bin/env bash
# PTY E2E 테스트 (SRS-TEST-006): tmux 창에서 mock-provider 빌드를 실제 터미널로 구동하고 결과를 판정한다.
# 사용: cargo build --features mock-provider && tests/pty/run.sh
# 결과: logs/pty_<시각>/ 아래 화면, 출력 파일, summary.txt. 실패 시 종료 코드 1.
set -u

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN="${HOWTUI_BIN:-$ROOT/target/debug/howtui}"
FIX="$ROOT/tests/fixtures"
OUT="$ROOT/logs/pty_$(date +%Y%m%d-%H%M%S)"
SESSION="howtui-pty-$$"
P="$SESSION:t"
mkdir -p "$OUT"
FAILED=0

[ -x "$BIN" ] || { echo "binary not found: $BIN (cargo build --features mock-provider)"; exit 1; }
command -v tmux >/dev/null || { echo "tmux required"; exit 1; }

tmux -u new-session -d -s "$SESSION" -n t -x 100 -y 30 "env -i HOME=$OUT PATH=/usr/bin:/bin TERM=xterm-256color LANG=${PTY_LANG:-C.UTF-8} PS1='$ ' bash --norc --noprofile" 2>&1 | cat
trap 'tmux kill-session -t "$SESSION" 2>/dev/null | cat' EXIT
sleep 0.5

send() { tmux send-keys -t "$P" "$@" 2>&1 | cat; }
lit() { tmux send-keys -t "$P" -l "$1" 2>&1 | cat; }
# snap 패키지 tmux는 파일로 직접 리디렉션한 stdout에 쓰지 못하므로 파이프를 거친다.
snap() { tmux capture-pane -p -t "$P" | cat > "$OUT/$1.screen"; }
pass() { echo "PASS $1" | tee -a "$OUT/summary.txt"; }
fail() { echo "FAIL $1: $2" | tee -a "$OUT/summary.txt"; FAILED=1; }
wait_file() { for _ in $(seq 1 100); do [ -f "$1" ] && return 0; sleep 0.1; done; return 1; }

ENV="HOWTUI_LANG=en HOWTUI_DATA_DIR=$OUT/data HOWTUI_CONFIG=$OUT/none.toml"

# 명령 실행: 종료 코드와 stty 상태를 기록하고 완료 파일 생성
start() {
  local name=$1 cmd=$2
  send C-c; sleep 0.2
  send "clear" Enter; sleep 0.3
  lit "$cmd; echo \$? > $OUT/$name.rc; stty -a > $OUT/$name.stty; touch $OUT/$name.done"
  send Enter
  sleep 0.8
}

finish() {
  if ! wait_file "$OUT/$1.done"; then
    fail "$1" "timeout"
    snap "$1.timeout"
    # 다음 케이스로 연쇄되지 않도록 정리
    pkill -KILL -x howtui 2>/dev/null
    sleep 0.3; send C-c; send "stty sane" Enter; sleep 0.3
    return 1
  fi
  if ! grep -q -- ' icanon' "$OUT/$1.stty" || ! grep -q -- ' echo' "$OUT/$1.stty"; then
    fail "$1" "terminal mode not restored"; return 1
  fi
}

rc_of() { cat "$OUT/$1.rc" 2>/dev/null; }

EXPECTED="$OUT/expected_basic"
printf '%s\n' "printf '%s\\n' \"안녕 세계 🌏\" \\" "  | grep -n '세계'" > "$EXPECTED"
MOCK="HOWTUI_MOCK_SCENARIO=$FIX/mock_basic.json"

# (1) 파이프와 stderr 분리 (TC-OUT-001, TC-OUT-008)
start pipe "$ENV $MOCK $BIN 2> $OUT/pipe.err | cat > $OUT/pipe.out"
lit "find big files"; send Enter; sleep 1.5; snap pipe_result; send Enter
if finish pipe; then
  if cmp -s "$OUT/pipe.out" "$EXPECTED" && [ ! -s "$OUT/pipe.err" ]; then pass pipe; else fail pipe "stdout or stderr mismatch"; fi
fi

# (2) 명령 치환 (TC-OUT-002)
start capture "x=\$($ENV $MOCK $BIN); printf '%s\n' \"\$x\" > $OUT/capture.out"
lit "q"; send Enter; sleep 1.5; send Enter
if finish capture; then
  if cmp -s "$OUT/capture.out" "$EXPECTED"; then pass capture; else fail capture "bytes differ"; fi
fi

# (3) 취소 Ctrl+C, Esc (TC-OUT-004)
for key in C-c Escape; do
  name="cancel_$key"
  start "$name" "$ENV $MOCK $BIN > $OUT/$name.out"
  lit "q"; send "$key"
  if finish "$name"; then
    if [ "$(rc_of "$name")" = 130 ] && [ ! -s "$OUT/$name.out" ]; then pass "$name"; else fail "$name" "rc $(rc_of "$name")"; fi
  fi
done

# (4) stdin 파이프 (TC-OUT-007)
start stdin_pipe "echo ignored | $ENV $MOCK $BIN > $OUT/stdin_pipe.out"
lit "q"; send Enter; sleep 1.5; send Enter
if finish stdin_pipe; then
  if cmp -s "$OUT/stdin_pipe.out" "$EXPECTED"; then pass stdin_pipe; else fail stdin_pipe "output mismatch"; fi
fi

# (5) 기존 출력 유지와 Alternate Screen 미사용 (TC-UI-001)
tmux pipe-pane -t "$P" -o "cat > $OUT/raw.log" 2>&1 | cat
start inline_keep "echo MARKER-LINE; $ENV $MOCK $BIN > /dev/null"
send Escape
if finish inline_keep; then
  tmux pipe-pane -t "$P" 2>&1 | cat
  snap inline_keep
  if grep -q MARKER-LINE "$OUT/inline_keep.screen" && ! grep -q $'\e\[?1049h' "$OUT/raw.log"; then pass inline_keep; else fail inline_keep "marker lost or alternate screen used"; fi
fi

# (6) 질문 인자 즉시 생성과 Ctrl+J, Alt+Enter 줄바꿈 (TC-CLI-001, TC-KEY-002)
start multiline "$ENV $MOCK $BIN list files > $OUT/multiline.out"
sleep 1.5; snap multiline_auto
send Down; send End; send C-j; lit "echo one"; send M-Enter; lit "echo two"; sleep 0.3; send Enter
if finish multiline; then
  if grep -q 'list files' "$OUT/multiline_auto.screen" && grep -q '^echo one$' "$OUT/multiline.out" && grep -q '^echo two$' "$OUT/multiline.out"; then
    pass multiline
  else
    fail multiline "auto submit or newline keys failed"
  fi
fi

# (7) bracketed paste (TC-EDIT-004)
start paste "$ENV $MOCK $BIN > $OUT/paste.out"
lit "q"; send Enter; sleep 1.5
tmux set-buffer -b howtui "pasted-a
pasted-b" 2>&1 | cat
send Down; send End; send C-j
tmux paste-buffer -p -b howtui -t "$P" 2>&1 | cat
sleep 0.3; snap paste_before_accept; send Enter
if finish paste; then
  if grep -q '^pasted-a$' "$OUT/paste.out" && grep -q '^pasted-b$' "$OUT/paste.out"; then pass paste; else fail paste "pasted lines missing"; fi
fi

# (8) 위험 명령어 재확인 (TC-SAFE-003)
cat > "$OUT/danger.json" <<'JSON'
{"steps": [{"delay_ms": 300, "command": "rm -rf ./build", "explanation": "Deletes build output."}]}
JSON
start danger "$ENV HOWTUI_MOCK_SCENARIO=$OUT/danger.json $BIN > $OUT/danger.out"
lit "clean"; send Enter; sleep 1; send Enter; sleep 0.3; snap danger_confirm
if [ -s "$OUT/danger.out" ]; then fail danger "accepted without confirmation"; fi
send Enter
if finish danger; then
  if grep -q 'High-risk' "$OUT/danger_confirm.screen" && [ "$(cat "$OUT/danger.out")" = "rm -rf ./build" ]; then pass danger; else fail danger "confirmation flow"; fi
fi

# (9) 리사이즈 (TC-UI-008)
start resize "$ENV $MOCK $BIN > $OUT/resize.out"
lit "q"; send Enter; sleep 1.5
tmux resize-window -t "$P" -x 60 -y 30 2>&1 | cat; sleep 0.5; snap resize_narrow
tmux resize-window -t "$P" -x 100 -y 12 2>&1 | cat; sleep 0.5; snap resize_short
tmux resize-window -t "$P" -x 100 -y 30 2>&1 | cat; sleep 0.5
send Enter
if finish resize; then
  if cmp -s "$OUT/resize.out" "$EXPECTED" && grep -q 'Command' "$OUT/resize_narrow.screen"; then pass resize; else fail resize "output or redraw"; fi
fi

# (10) 최소 크기 미만 (TC-UI-009)
tmux resize-window -t "$P" -x 25 -y 5 2>&1 | cat; sleep 0.3
start tiny "$ENV $MOCK $BIN > $OUT/tiny.out"
lit "q"; send Enter; sleep 0.5; snap tiny; send Escape
if finish tiny; then
  if grep -q 'too small' "$OUT/tiny.screen" && [ "$(rc_of tiny)" = 130 ] && [ ! -s "$OUT/tiny.out" ]; then pass tiny; else fail tiny "too small handling"; fi
fi
tmux resize-window -t "$P" -x 100 -y 30 2>&1 | cat; sleep 0.3

# (11) 줄 중간 시작과 커서 원위치 (TC-UI-011)
start midline "printf 'prefix'; $ENV $MOCK $BIN > /dev/null; echo END"
send Escape
if finish midline; then
  snap midline
  if grep -q 'prefixEND' "$OUT/midline.screen"; then pass midline; else fail midline "cursor not restored to origin column"; fi
fi

# (12) panic 복구 (TC-ERR-003)
start panic "HOWTUI_TEST_PANIC=1 $ENV $MOCK $BIN > $OUT/panic.out 2> $OUT/panic.err"
if finish panic; then
  if [ "$(rc_of panic)" = 101 ] && [ ! -s "$OUT/panic.out" ]; then pass panic; else fail panic "rc $(rc_of panic)"; fi
fi

# (13) 시그널 복구 (TC-ERR-004)
for sig in TERM HUP; do
  name="signal_$sig"
  start "$name" "$ENV $MOCK $BIN > $OUT/$name.out"
  pkill "-$sig" -x howtui
  if finish "$name"; then
    want=$([ "$sig" = TERM ] && echo 143 || echo 129)
    if [ "$(rc_of "$name")" = "$want" ] && [ ! -s "$OUT/$name.out" ]; then pass "$name"; else fail "$name" "rc $(rc_of "$name")"; fi
  fi
done

# (14) 로그인 자동 시작 화면 (TC-UI-012)
start login "HOWTUI_NO_BROWSER=1 $ENV $BIN > $OUT/login.out"
sleep 0.5; snap login; send Escape; sleep 0.3; snap login_cancelled; send Escape
if finish login; then
  if grep -q 'Waiting for sign-in' "$OUT/login.screen" && grep -q '1. Sign in' "$OUT/login.screen" && grep -q 'to OpenAI' "$OUT/login.screen" \
    && grep -q 'auth.openai.com' "$OUT/login.screen" && grep -q 'Sign-in required' "$OUT/login_cancelled.screen" \
    && [ "$(rc_of login)" = 130 ]; then pass login; else fail login "login screen"; fi
fi

# (15) --height 옵션 (TC-CLI-005): 상태줄이 시작 위치에서 4번째 줄
start height "echo TOP; $ENV $MOCK $BIN --height 4 > /dev/null"
snap height; send Escape
if finish height; then
  top=$(grep -n '^TOP$' "$OUT/height.screen" | head -1 | cut -d: -f1)
  status=$(sed -n "$((top + 4))p" "$OUT/height.screen")
  case "$status" in *"Enter submit"*) pass height ;; *) fail height "status line not at row 4: $status" ;; esac
fi

# (16) 한국어 화면 (TC-I18N-004)
start korean "HOWTUI_LANG=ko HOWTUI_DATA_DIR=$OUT/data $MOCK $BIN > /dev/null"
snap korean; send Escape
if finish korean; then
  if grep -q '질문' "$OUT/korean.screen"; then pass korean; else fail korean "Korean labels missing"; fi
fi

# (17) 설정 화면: Ctrl+O 모델 변경, 상태줄 모델 버튼 클릭, 항목 클릭, 마우스 모드 복구 (TC-UI-019, TC-UI-020)
# 클릭은 SGR 마우스 시퀀스(ESC [ < 0 ; 열 ; 행 M/m, 1부터)로 입력
click() { lit $'\e'"[<0;$1;$2M"; lit $'\e'"[<0;$1;$2m"; }
tmux pipe-pane -t "$P" -o "cat > $OUT/settings_raw.log" 2>&1 | cat
start settings "$ENV $MOCK $BIN > $OUT/settings.out"
send C-o; sleep 0.5; snap settings_open
send Down; send Enter; sleep 0.5; snap settings_changed
pos=$(tmux capture-pane -p -t "$P" | cat | grep -n '\[mock-2\]' | tail -1)
crow=${pos%%:*}; line=${pos#*:}; prefix=${line%%\[mock-2\]*}; ccol=$(( ${#prefix} + 2 ))
click "$ccol" "$crow"; sleep 0.5; snap settings_clicked
irow=$(tmux capture-pane -p -t "$P" | cat | grep -n 'Mock mock-model' | tail -1 | cut -d: -f1)
click 12 "$irow"; sleep 0.5; snap settings_item
send Escape; sleep 0.3
# 설정 화면이 남아 있으면 첫 Esc는 닫기
[ -f "$OUT/settings.done" ] || send Escape
tmux pipe-pane -t "$P" 2>&1 | cat
if finish settings; then
  if grep -q 'Settings  Agent: Mock' "$OUT/settings_open.screen" && grep -q '(\*) Mock mock-model' "$OUT/settings_open.screen" \
    && grep -q 'Model: mock-2' "$OUT/settings_changed.screen" && grep -q '\[mock-2\]' "$OUT/settings_changed.screen" \
    && grep -q 'Settings  Agent: Mock' "$OUT/settings_clicked.screen" && grep -q '\[mock-model\]' "$OUT/settings_item.screen" \
    && grep -q $'\e\[?1000h' "$OUT/settings_raw.log" && grep -q $'\e\[?1000l' "$OUT/settings_raw.log" \
    && [ "$(rc_of settings)" = 130 ]; then pass settings; else fail settings "settings view, clicks or mouse mode"; fi
fi

echo "result: $( [ $FAILED = 0 ] && echo ok || echo failed ) ($OUT)"
exit $FAILED
