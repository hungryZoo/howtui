# howtui shell integration for Bash (interactive shells; the key binding needs Bash 4.0 or newer).
#
# Enable:  add this line to ~/.bashrc, then open a new shell
#            eval "$(howtui init bash)"
# Disable: remove that line and open a new shell.
# Options: howtui init bash --key alt+h    use another key (default ctrl+g)
#          howtui init bash --no-bind      do not bind a key
#          howtui init bash --no-wrap      do not define the howtui function
#
# @KEY_NAME@ at the prompt: open howtui with the current line; the accepted command replaces the line.
# Typing howtui at the prompt: the accepted command is placed in the next prompt line and in history.
# Nothing is executed automatically. This script never edits rc files.
if [[ $- == *i* ]]; then

#@wrap
howtui() {
    local __howtui_file __howtui_rc
    __howtui_file=$(command mktemp "${TMPDIR:-/tmp}/howtui.XXXXXX" 2>/dev/null) || __howtui_file=
    HOWTUI_SHELL=bash HOWTUI_SHELL_VERSION="$BASH_VERSION" HOWTUI_SHELL_INTEGRATION=bash \
        HOWTUI_NEXT_INPUT_FILE="$__howtui_file" command howtui "$@"
    __howtui_rc=$?
    if [[ -n $__howtui_file ]]; then
        if [[ $__howtui_rc == 0 && -s $__howtui_file ]]; then
            __howtui_next=$(<"$__howtui_file")
            # replaces this howtui line in history
            history -s -- "$__howtui_next"
        fi
        command rm -f -- "$__howtui_file"
    fi
    return "$__howtui_rc"
}

if ((BASH_VERSINFO[0] >= 4)); then
    # Before the next prompt, ask the terminal for a status report (ESC[5n). Its answer (ESC[0n)
    # arrives as input to the new prompt line and runs __howtui_prefill.
    __howtui_prompt() {
        if [[ -n ${__howtui_next+set} ]]; then
            printf '\033[5n' >/dev/tty 2>/dev/null
        fi
    }
    __howtui_prefill() {
        if [[ -n ${__howtui_next+set} ]]; then
            if [[ -z $READLINE_LINE ]]; then
                READLINE_LINE=$__howtui_next
                READLINE_POINT=2147483647
            fi
            unset __howtui_next
        fi
    }
    for __howtui_km in emacs vi-insert vi-command; do
        bind -m "$__howtui_km" -x '"\e[0n": __howtui_prefill'
    done
    unset __howtui_km
    if [[ ";${PROMPT_COMMAND[*]};" != *";__howtui_prompt;"* ]]; then
        PROMPT_COMMAND="__howtui_prompt${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
    fi
fi
#@end-wrap

#@bind
if ((BASH_VERSINFO[0] >= 4)); then
    __howtui_widget() {
        local file rc paste=0 lbuffer=
        file=$(command mktemp "${TMPDIR:-/tmp}/howtui.XXXXXX" 2>/dev/null) || {
            echo "howtui: cannot create a temporary file" >&2
            return 1
        }
        [[ $(bind -v 2>/dev/null) == *"enable-bracketed-paste on"* ]] && paste=1
        # READLINE_POINT counts characters from Bash 5.0
        ((BASH_VERSINFO[0] >= 5)) && lbuffer=${READLINE_LINE:0:READLINE_POINT}
        HOWTUI_SHELL=bash HOWTUI_SHELL_VERSION="$BASH_VERSION" HOWTUI_SHELL_INTEGRATION=bash \
            HOWTUI_BUFFER="$READLINE_LINE" HOWTUI_LBUFFER="$lbuffer" HOWTUI_OUTPUT_FILE="$file" \
            HOWTUI_BRACKETED_PASTE=$paste command howtui --widget </dev/tty >/dev/tty 2>/dev/tty
        rc=$?
        if [[ $rc == 0 && -s $file ]]; then
            READLINE_LINE=$(<"$file")
            READLINE_POINT=2147483647
        fi
        command rm -f -- "$file"
    }
    for __howtui_km in emacs vi-insert vi-command; do
        # Bash 5.1 does not replace an existing binding without removing it first
        bind -m "$__howtui_km" -r '@KEY_BASH@' 2>/dev/null
        bind -m "$__howtui_km" -x '"@KEY_BASH@": __howtui_widget'
    done
    unset __howtui_km
else
    echo "howtui: the @KEY_NAME@ binding needs Bash 4.0 or newer (this is $BASH_VERSION); skipped" >&2
fi
#@end-bind

fi
