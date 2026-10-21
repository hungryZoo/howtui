# SPIKE-03 Bash bind -x 입력 버퍼 삽입 검증용 rc 파일.
# 사용: HOWTUI_SPIKE_BIN=<inline-spike 경로> bash --rcfile spikes/shell/bind_spike.bash -i
# 실험 쉘이 사용자 ~/.bash_history에 기록하지 않도록 기록 파일을 끈다.
unset HISTFILE
__howtui_spike_widget() {
  local out rc
  out=$("$HOWTUI_SPIKE_BIN" --quiet </dev/tty 2>/dev/tty)
  rc=$?
  if [ $rc -eq 0 ] && [ -n "$out" ]; then
    READLINE_LINE=$out
    READLINE_POINT=${#READLINE_LINE}
  fi
  echo "widget_rc=$rc line_len=${#READLINE_LINE}" >> "${HOWTUI_SPIKE_LOG:-/dev/null}"
}
bind -x '"\C-g": __howtui_spike_widget'
PS1='$ '
