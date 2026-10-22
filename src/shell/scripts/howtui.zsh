# howtui shell integration for Zsh 5.1 or newer (interactive shells).
#
# Enable:  add this line to ~/.zshrc, then open a new shell
#            eval "$(howtui init zsh)"
# Disable: remove that line and open a new shell.
# Options: howtui init zsh --key alt+h    use another key (default ctrl+g)
#          howtui init zsh --no-bind      do not bind a key
#          howtui init zsh --no-wrap      do not define the howtui function
#
# @KEY_NAME@ at the prompt: open howtui with the current line; the accepted command replaces the line.
# Typing howtui at the prompt: the accepted command is placed in the next prompt line.
# Nothing is executed automatically. This script never edits rc files.
if [[ -o interactive ]]; then

#@wrap
howtui() {
    emulate -L zsh
    local file rc
    file=$(command mktemp "${TMPDIR:-/tmp}/howtui.XXXXXX" 2>/dev/null) || file=
    HOWTUI_SHELL=zsh HOWTUI_SHELL_VERSION=$ZSH_VERSION HOWTUI_SHELL_INTEGRATION=zsh \
        HOWTUI_NEXT_INPUT_FILE=$file command howtui "$@"
    rc=$?
    if [[ -n $file ]]; then
        if (( rc == 0 )) && [[ -s $file ]]; then
            # buffer stack: the next prompt starts with this text
            print -rz -- "$(<$file)"
        fi
        command rm -f -- $file
    fi
    return $rc
}
#@end-wrap

#@bind
__howtui_widget() {
    emulate -L zsh
    local file rc paste=0
    if (( ! ${+commands[howtui]} )); then
        zle -M "howtui: command not found"
        return 1
    fi
    file=$(command mktemp "${TMPDIR:-/tmp}/howtui.XXXXXX" 2>/dev/null) || {
        zle -M "howtui: cannot create a temporary file"
        return 1
    }
    (( ${#zle_bracketed_paste} )) && paste=1
    # widget stdin is not the terminal: connect all three explicitly
    HOWTUI_SHELL=zsh HOWTUI_SHELL_VERSION=$ZSH_VERSION HOWTUI_SHELL_INTEGRATION=zsh \
        HOWTUI_BUFFER=$BUFFER HOWTUI_LBUFFER=$LBUFFER HOWTUI_OUTPUT_FILE=$file \
        HOWTUI_BRACKETED_PASTE=$paste command howtui --widget </dev/tty >/dev/tty 2>/dev/tty
    rc=$?
    if (( rc == 0 )) && [[ -s $file ]]; then
        BUFFER=$(<$file)
        CURSOR=$#BUFFER
    fi
    command rm -f -- $file
    zle reset-prompt
}
zle -N __howtui_widget
bindkey -M emacs '@KEY_ZSH@' __howtui_widget
bindkey -M viins '@KEY_ZSH@' __howtui_widget
bindkey -M vicmd '@KEY_ZSH@' __howtui_widget
#@end-bind

fi
