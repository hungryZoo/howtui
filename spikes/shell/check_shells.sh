#!/usr/bin/env bash
# SPIKE-03/04 쉘 입력 버퍼 삽입 검증. Bash(호스트)와 Zsh(docker)에서 기존 입력 보존, 확정, 취소를 확인한다.
# 사용: spikes/shell/check_shells.sh  (tmux 창 howtui:spike 필요, inline-spike 디버그 빌드 필요)
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BIN_DIR="$ROOT/spikes/inline-spike/target/debug"
OUT="$ROOT/logs/spike03_shells_$(date +%Y%m%d-%H%M%S)"
P=howtui:spike
mkdir -p "$OUT"
send() { tmux send-keys -t "$P" "$@"; }
lit() { tmux send-keys -t "$P" -l "$1"; }
# snap 패키지 tmux는 파일로 직접 리디렉션한 stdout에 쓰지 못하므로 파이프를 거친다.
snap() { tmux capture-pane -p -t "$P" | cat > "$OUT/$1.screen"; }

scenario() { # prefix
  local p=$1
  send "clear" Enter; sleep 0.4
  # (a) 프롬프트 줄에 기존 입력이 있는 상태에서 호출 후 확정
  lit "ls -la 기존입력"; send C-g; sleep 0.8; snap "${p}_a_tui"
  lit "q"; send Enter; sleep 2.2; send Enter; sleep 0.8; snap "${p}_a_buffer"
  # 버퍼 폐기 (실행하지 않음): Ctrl+C는 bash와 zsh 모두 현재 입력 줄 전체를 버린다.
  send C-c; sleep 0.3; send "clear" Enter; sleep 0.4
  # (b) 호출 후 취소, 기존 입력 보존
  lit "echo 보존확인"; send C-g; sleep 0.8; send C-c; sleep 0.8; snap "${p}_b_cancel"
  send C-c; sleep 0.3
  # (c) 빈 프롬프트에서 호출 후 확정
  send "clear" Enter; sleep 0.4
  send C-g; sleep 0.8; lit "q"; send Enter; sleep 2.2; send Enter; sleep 0.8; snap "${p}_c_empty"
  send C-c; sleep 0.3
}

send C-c; sleep 0.2
lit "env -i HOME=\$HOME TERM=xterm-256color LANG=C.UTF-8 PATH=/usr/bin:/bin HOWTUI_SPIKE_BIN=$BIN_DIR/inline-spike HOWTUI_SPIKE_LOG=$OUT/bash_widget.log bash --rcfile $ROOT/spikes/shell/bind_spike.bash -i"
send Enter; sleep 0.8
scenario bash
send "exit" Enter; sleep 0.5

lit "docker run --rm -it --user $(id -u):$(id -g) -e HOME=/tmp -e ZDOTDIR=/shell -e HOWTUI_SPIKE_BIN=/spike/inline-spike -e HOWTUI_SPIKE_LOG=/out/zsh_widget.log -v $BIN_DIR:/spike:ro -v $ROOT/spikes/shell/zdot:/shell:ro -v $OUT:/out howtui-spike-zsh zsh -i"
send Enter; sleep 2
scenario zsh
send "exit" Enter; sleep 1
echo "$OUT"
