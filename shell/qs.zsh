# tmux-quickselect shell integration for Zsh
# Source this file in your .zshrc:
#   source /path/to/shell/qs.zsh

qs() {
    local result
    result="$(command qs "$@")"
    if [ -n "$result" ]; then
        cd "$result" || return
    fi
}
