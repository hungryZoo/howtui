# SPIKE-04 Zsh ZLE 입력 버퍼 삽입 검증용 rc 파일. (ZDOTDIR로 지정)
unset HISTFILE
__howtui_spike_widget() {
  local out rc
  out=$("$HOWTUI_SPIKE_BIN" --quiet </dev/tty 2>/dev/tty)
  rc=$?
  if (( rc == 0 )) && [[ -n $out ]]; then
    BUFFER=$out
    CURSOR=${#BUFFER}
  fi
  zle reset-prompt
  print -r -- "widget_rc=$rc len=${#BUFFER}" >> "${HOWTUI_SPIKE_LOG:-/dev/null}"
}
zle -N __howtui_spike_widget
bindkey -M emacs '^G' __howtui_spike_widget
bindkey -M viins '^G' __howtui_spike_widget
PS1='%% '
