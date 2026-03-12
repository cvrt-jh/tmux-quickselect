# tmux-quickselect shell integration for Bash
# Source this file in your .bashrc:
#   source /path/to/shell/qs.bash

qs() {
    local result
    result="$(command qs "$@")"
    if [ -n "$result" ]; then
        cd "$result" || return
    fi
}
