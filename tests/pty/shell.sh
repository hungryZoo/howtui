#!/usr/bin/env bash
# 쉘 통합 PTY E2E 테스트 (SRS-TEST-006, TC-SH-001-012, TC-SH-016): tmux 창에서 `howtui init` 등록 쉘을 구동한다.
# 사용: cargo build --features mock-provider && tests/pty/shell.sh <bash|zsh|powershell>
#   PTY_DOCKER_IMAGE: 지정 시 해당 이미지 컨테이너 안에서 쉘 실행 (호스트에 없는 쉘 확인용)
#   PTY_LANG: 쉘 로캘 (기본 C.UTF-8, macOS는 en_US.UTF-8)
#   PTY_SHELL_BIN: 쉘 실행 파일 경로 재정의 (예: macOS /bin/bash 3.2)
# Bash 4.0 미만(macOS 기본 3.2)은 바인딩 생략 경고와 히스토리 전달만 확인 (TC-SH-015).
# 결과: logs/pty_shell_<쉘>_<시각>/ 아래 화면과 summary.txt. 실패 시 종료 코드 1.
set -u

KIND=${1:?usage: shell.sh <bash|zsh|powershell>}
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN_DIR="$ROOT/target/debug"
OUT="${PTY_OUT:-$ROOT/logs/pty_shell_${KIND}_$(date +%Y%m%d-%H%M%S)}"
SESSION="howtui-sh-$$"
P="$SESSION:t"
mkdir -p "$OUT/home" "$OUT/tmp"
printf '%s\n' '{"steps": [{"delay_ms": 100, "error": "usage_limit"}]}' > "$OUT/error.json"
printf '%s\n' '{"steps": [{"delay_ms": 100, "command": "echo one \\\n  two", "explanation": "x"}]}' > "$OUT/multi.json"
FAILED=0

[ -x "$ROOT/target/debug/howtui" ] || { echo "binary not found (cargo build --features mock-provider)"; exit 1; }
command -v tmux >/dev/null || { echo "tmux required"; exit 1; }

# 쉘 실행 파일: env -i의 PATH에 없을 수 있으므로 절대 경로 사용 (컨테이너는 이름)
shell_bin() {
  if [ -n "${PTY_SHELL_BIN:-}" ]; then echo "$PTY_SHELL_BIN"
  elif [ -n "${PTY_DOCKER_IMAGE:-}" ]; then echo "$1"
  else command -v "$1" || { echo "$1 not found" >&2; exit 1; }; fi
}
LEGACY=0
# 쉘 rc: 기록 파일 없음, 고정 프롬프트, 통합 스크립트 등록. 쉘별 명령 문법.
case "$KIND" in
  bash | zsh)
    if [ "$KIND" = bash ]; then
      printf '%s\n' 'unset HISTFILE' "PS1='PROMPT> '" 'eval "$(howtui init bash)"' > "$OUT/home/.bashrc"
      SH_BIN=$(shell_bin bash)
      DEFAULT_CMD="$SH_BIN --rcfile $OUT/home/.bashrc -i"
      if [ -z "${PTY_DOCKER_IMAGE:-}" ] && [ "$("$SH_BIN" -c 'echo ${BASH_VERSINFO[0]}')" -lt 4 ]; then LEGACY=1; fi
    else
      printf '%s\n' 'unset HISTFILE' "PROMPT='PROMPT> '" 'eval "$(howtui init zsh)"' > "$OUT/home/.zshrc"
      # 배포판 전역 rc 제외 (Ubuntu /etc/zsh/zshrc의 compinit 확인 질문)
      DEFAULT_CMD="$(shell_bin zsh) --no-globalrcs -i"
    fi
    PATH_BREAK='OLDPATH=$PATH; PATH=/usr/bin:/bin'
    PATH_RESTORE='PATH=$OLDPATH'
    CAPTURE='x=$(howtui captured one)'
    SHOW_CAPTURE='echo "got:$x"'
    SHOW_RC='echo "rc:$?"'
    REINIT="eval \"\$(howtui init $KIND --key alt+h --no-wrap)\""
    SET_ERROR="export HOWTUI_MOCK_SCENARIO=$OUT/error.json"
    SET_ECHO='export HOWTUI_MOCK_SCENARIO=echo'
    SET_MULTI="export HOWTUI_MOCK_SCENARIO=$OUT/multi.json"
    FILL='for i in $(seq 1 40); do echo fill$i; done'
    ;;
  powershell)
    cat > "$OUT/home/profile.ps1" <<'PS1'
Set-PSReadLineOption -HistorySaveStyle SaveNothing
try { Set-PSReadLineOption -PredictionSource None } catch { }
function global:prompt { 'PROMPT> ' }
howtui init powershell | Out-String | Invoke-Expression
PS1
    DEFAULT_CMD="$(shell_bin pwsh) -NoLogo -NoProfile -NoExit -File $OUT/home/profile.ps1"
    PATH_BREAK='$OLDPATH = $env:PATH; $env:PATH = "/usr/bin:/bin"'
    PATH_RESTORE='$env:PATH = $OLDPATH'
    CAPTURE='$x = howtui captured one'
    SHOW_CAPTURE='"got:$x"'
    SHOW_RC='"rc:$LASTEXITCODE"'
    REINIT='howtui init powershell --key alt+h --no-wrap | Out-String | Invoke-Expression'
    SET_ERROR="\$env:HOWTUI_MOCK_SCENARIO = '$OUT/error.json'"
    SET_ECHO="\$env:HOWTUI_MOCK_SCENARIO = 'echo'"
    SET_MULTI="\$env:HOWTUI_MOCK_SCENARIO = '$OUT/multi.json'"
    FILL='1..40 | ForEach-Object { "fill$_" }'
    ;;
  *) echo "unknown shell: $KIND"; exit 2 ;;
esac
if [ -n "${PTY_DOCKER_IMAGE:-}" ]; then
  BIN_DIR=/opt/howtui
fi
ENVS="HOME=$OUT/home ZDOTDIR=$OUT/home PATH=$BIN_DIR:/usr/bin:/bin TERM=xterm-256color LANG=${PTY_LANG:-C.UTF-8} TMPDIR=$OUT/tmp BASH_SILENCE_DEPRECATION_WARNING=1 \
HOWTUI_LANG=en HOWTUI_MOCK_SCENARIO=echo HOWTUI_DATA_DIR=$OUT/data HOWTUI_CONFIG=$OUT/none.toml"
if [ -n "${PTY_DOCKER_IMAGE:-}" ]; then
  # 결과 디렉터리는 같은 경로로 연결, 바이너리는 읽기 전용
  # 이름 지정: tmux 세션 종료(SIGHUP) 후에도 남는 쉘(pwsh) 컨테이너를 종료 시 삭제
  CONTAINER="$SESSION"
  dflags="--rm -it --name $CONTAINER --user $(id -u):$(id -g) -v $ROOT/target/debug:$BIN_DIR:ro -v $OUT:$OUT -w $OUT"
  for kv in $ENVS; do dflags="$dflags -e $kv"; done
  LAUNCH="docker run $dflags $PTY_DOCKER_IMAGE $DEFAULT_CMD"
else
  LAUNCH="env -i $ENVS $DEFAULT_CMD"
fi

tmux -u new-session -d -s "$SESSION" -n t -x 100 -y 30 "$LAUNCH" 2>&1 | cat
trap 'tmux kill-session -t "$SESSION" 2>/dev/null | cat; [ -n "${CONTAINER:-}" ] && docker rm -f "$CONTAINER" >/dev/null 2>&1' EXIT
sleep 1

send() { tmux send-keys -t "$P" "$@" 2>&1 | cat; }
lit() { tmux send-keys -t "$P" -l "$1" 2>&1 | cat; }
snap() { tmux capture-pane -p -t "$P" | cat > "$OUT/$1.screen"; }
last_line() { tmux capture-pane -p -t "$P" | cat | sed '/^[[:space:]]*$/d' | tail -1 | sed 's/[[:space:]]*$//'; }
pass() { echo "PASS $KIND/$1" | tee -a "$OUT/summary.txt"; }
fail() { echo "FAIL $KIND/$1: $2" | tee -a "$OUT/summary.txt"; FAILED=1; }
reset_line() { send C-u; sleep 0.1; send C-l; sleep 0.3; }
# 프롬프트 대기
wait_prompt() { for _ in $(seq 1 50); do case "$(last_line)" in "PROMPT>"*) return 0 ;; esac; sleep 0.1; done; return 1; }
# 명령 실행 후 빈 프롬프트 대기 (고정 대기는 느린 CI 러너에서 경합)
run_line() { lit "$1"; send Enter; for _ in $(seq 1 50); do [ "$(last_line)" = "PROMPT>" ] && return 0; sleep 0.1; done; return 1; }
# 화면에 지정 줄이 나타날 때까지 대기
wait_line() { for _ in $(seq 1 50); do tmux capture-pane -p -t "$P" | cat | sed 's/[[:space:]]*$//' | grep -qxF "$1" && return 0; sleep 0.1; done; return 1; }

if ! wait_prompt; then
  snap start; fail start "no prompt"; exit 1
fi
snap start
# 컨테이너의 사용자 그룹 이름 경고는 시험 환경 출력이므로 제외
nonempty=$(sed '/^[[:space:]]*$/d; /^groups: cannot find name/d' "$OUT/start.screen" | wc -l | tr -d ' ')
if [ "$LEGACY" = 1 ]; then
  # (L) Bash 3.2: 바인딩 생략 경고 1줄, 쉘 정상, 직접 실행 결과는 히스토리(↑)로 전달 (TC-SH-015)
  if grep -q 'needs Bash 4.0 or newer' "$OUT/start.screen" && [ "$nonempty" = 2 ]; then pass legacy_warning; else fail legacy_warning "warning line"; fi
  reset_line
  lit "howtui show disk usage"; send Enter; sleep 1.2; send Enter; sleep 0.8
  empty=$(last_line); send Up; sleep 0.3; line=$(last_line)
  if [ "$empty" = "PROMPT>" ] && [ "$line" = "PROMPT> echo 'show disk usage'" ]; then pass legacy_history; else fail legacy_history "[$empty] [$line]"; fi
  echo "result: $( [ $FAILED = 0 ] && echo ok || echo failed ) ($OUT)"
  exit $FAILED
fi
# 등록 시 경고와 오류 출력 없음: 화면에 프롬프트 1줄만
if [ "$nonempty" != 1 ]; then fail start "init script printed output"; fi

# (1) 단축키: 입력 줄을 명령어 초기값으로 열고 확정 시 입력란 교체, 미실행 (TC-SH-001, TC-SH-002, TC-SH-012)
reset_line
lit "ls -l"; send C-g; sleep 1; snap widget_open
lit "list files"; send Enter; sleep 0.8; snap widget_generated; send Enter; sleep 0.5; snap widget_done
line=$(last_line)
if grep -q 'Command > ls -l' "$OUT/widget_open.screen" && [ "$line" = "PROMPT> echo 'list files'" ] \
  && ! grep -qx 'list files' "$OUT/widget_done.screen"; then
  pass widget_replace
else
  fail widget_replace "last line [$line]"
fi

# (1b) 여러 줄 명령어 삽입 (TC-SH-014)
reset_line
run_line "$SET_MULTI"
send C-g; sleep 1; lit "two lines"; send Enter; sleep 0.6; send Enter; sleep 0.5; snap widget_multiline
send C-c; sleep 0.3
run_line "$SET_ECHO"
if awk 'found && /two$/ { ok = 1 } { found = ($0 ~ /^PROMPT> echo one \\[[:space:]]*$/) } END { exit !ok }' "$OUT/widget_multiline.screen"; then
  pass widget_multiline
else
  fail widget_multiline "two-line command not in the prompt"
fi

# (1c) 화면 맨 아래에서 실행: 화면 스크롤 후에도 입력 줄 1개만 남고 이전 출력 유지 (TC-SH-001)
reset_line
run_line "$FILL"
lit "echo bottom"; send C-g; sleep 1; lit "at bottom"; send Enter; sleep 0.6; send Enter; sleep 0.5; snap widget_bottom
prev=$(sed '/^[[:space:]]*$/d' "$OUT/widget_bottom.screen" | tail -2 | head -1)
if [ "$(last_line)" = "PROMPT> echo 'at bottom'" ] && [ "$prev" = "fill40" ] && [ "$(grep -c '^PROMPT>' "$OUT/widget_bottom.screen")" = 1 ]; then
  pass widget_bottom
else
  fail widget_bottom "screen after scrolling"
fi
send C-u

# (2) 취소 시 입력란 보존 (TC-SH-005)
reset_line
lit "echo keep"; send C-g; sleep 1; send Escape; sleep 0.5
line=$(last_line)
if [ "$line" = "PROMPT> echo keep" ]; then pass widget_cancel; else fail widget_cancel "last line [$line]"; fi

# (3) 바이너리 없음: 입력란 보존, 원인 표시 (TC-SH-006)
reset_line
run_line "$PATH_BREAK"
lit "echo still"; wait_line "PROMPT> echo still"; send C-g; sleep 1; snap widget_missing
# Zsh는 입력 줄 아래 메시지 표시: 마지막 프롬프트 줄로 판정
line=$(grep '^PROMPT>' "$OUT/widget_missing.screen" | tail -1 | sed 's/[[:space:]]*$//')
send C-u; run_line "$PATH_RESTORE"
if [ "$line" = "PROMPT> echo still" ]; then
  pass widget_missing
else
  fail widget_missing "last line [$line]"
fi

# (3b) 요청 오류 후 종료: 입력란 보존 (TC-SH-006)
reset_line
run_line "$SET_ERROR"
lit "echo err-keep"; send C-g; sleep 1; lit "q"; send Enter; sleep 0.6; snap widget_error; send Escape; sleep 0.5
line=$(last_line)
send C-u; run_line "$SET_ECHO"
if grep -q 'usage limit' "$OUT/widget_error.screen" && [ "$line" = "PROMPT> echo err-keep" ]; then
  pass widget_error
else
  fail widget_error "last line [$line]"
fi

# (3c) 출력 파일 기록 실패: 원인과 명령어를 터미널에 표시, 입력란 보존 (TC-SH-007)
reset_line
lit "echo fallback"; send C-g; sleep 1
chmod 000 "$OUT"/tmp/* 2>/dev/null
lit "fallback case"; send Enter; sleep 0.8; send Enter; sleep 0.6; snap widget_fallback
chmod 600 "$OUT"/tmp/* 2>/dev/null
if grep -q 'Copy it manually' "$OUT/widget_fallback.screen" && grep -q "^echo 'fallback case'$" "$OUT/widget_fallback.screen" \
  && [ "$(grep '^PROMPT>' "$OUT/widget_fallback.screen" | tail -1 | sed 's/[[:space:]]*$//')" = "PROMPT> echo fallback" ]; then
  pass widget_fallback
else
  fail widget_fallback "fallback message or line"
fi

# (4) 직접 실행한 howtui의 확정 명령어가 다음 입력란으로 이동, 미실행 (TC-SH-016)
reset_line
lit "howtui show disk usage"; send Enter; sleep 1.2; snap next_generated; send Enter; sleep 0.8; snap next_done
line=$(last_line)
if [ "$line" = "PROMPT> echo 'show disk usage'" ] && ! grep -qx 'show disk usage' "$OUT/next_done.screen"; then
  pass next_input
else
  fail next_input "last line [$line]"
fi

# (5) 캡처 시 stdout 전달, 다음 입력란 무변경 (TC-SH-016, TC-OUT-002)
reset_line
lit "$CAPTURE"; send Enter; sleep 1.2; send Enter; sleep 0.8
lit "$SHOW_CAPTURE"; send Enter; sleep 0.4; snap capture
line=$(last_line)
if grep -q "^got:echo 'captured one'$" "$OUT/capture.screen" && [ "$line" = "PROMPT>" ]; then
  pass wrapper_capture
else
  fail wrapper_capture "last line [$line]"
fi

# (6) 취소 시 다음 입력란 무변경, 종료 코드 130 전달
reset_line
lit "howtui"; send Enter; sleep 1; send Escape; sleep 0.6
lit "$SHOW_RC"; send Enter; sleep 0.4; snap wrapper_cancel
if grep -q '^rc:130$' "$OUT/wrapper_cancel.screen" && [ "$(last_line)" = "PROMPT>" ]; then
  pass wrapper_cancel
else
  fail wrapper_cancel "rc or prompt"
fi

# (7) 통합 표식 (TC-SH-010)
reset_line
lit "howtui doctor"; send Enter; sleep 1; snap doctor
if grep -q "shell integration: $KIND" "$OUT/doctor.screen"; then pass doctor; else fail doctor "integration not reported"; fi

# (8) 단축키 변경과 바인딩 생략 (TC-SH-004)
reset_line
lit "$REINIT"; send Enter; sleep 0.8
lit "echo alt"; send M-h; sleep 1; snap alt_key; send Escape; sleep 0.5
line=$(last_line)
if grep -q 'Command > echo alt' "$OUT/alt_key.screen" && [ "$line" = "PROMPT> echo alt" ]; then
  pass key_option
else
  fail key_option "alt+h did not open howtui"
fi

echo "result: $( [ $FAILED = 0 ] && echo ok || echo failed ) ($OUT)"
exit $FAILED
