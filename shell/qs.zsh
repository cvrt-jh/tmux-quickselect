qs() {
    local result
    result=$(qs "$@") || return
    if [[ -n "$result" ]]; then
        cd "$result" || return
    fi
}
