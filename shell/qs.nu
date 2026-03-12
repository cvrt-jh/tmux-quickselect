# tmux-quickselect shell integration for Nushell
# Source this file in your config.nu:
#   source /path/to/shell/qs.nu

def --env qs [...args: string] {
    let result = (^qs ...$args)
    if ($result | is-not-empty) {
        cd $result
    }
}
