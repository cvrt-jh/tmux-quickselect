# qs.nu
# tmux-quickselect: Interactive directory launcher for tmux with Nushell
# https://github.com/cvrt-jh/tmux-quickselect

# ============ Configuration ============

def get-config [] {
    let config_paths = [
        "~/.config/tmux-quickselect/config.nuon"
    ] | each { path expand }
    
    let config_file = ($config_paths | where { path exists } | get -o 0)
    
    if ($config_file == null) {
        # Default configuration
        {
            directories: [
                { path: "~/Git", label: "git", color: "cyan" }
            ]
            command: ""
            cache_dir: "~/.cache/tmux-quickselect"
            ui: { title: "Quick Select", icon: "📂", width: 25 }
        }
    } else {
        open $config_file
    }
}

# ============ Helper Functions ============

def format-ago [timestamp: string] {
    let diff = (date now) - ($timestamp | into datetime)
    if $diff < 1min {
        "just now"
    } else if $diff < 1hr {
        $"($diff / 1min | math floor)m ago"
    } else if $diff < 24hr {
        $"($diff / 1hr | math floor)h ago"
    } else if $diff < 7day {
        $"($diff / 1day | math floor)d ago"
    } else {
        $"($diff / 1wk | math floor)w ago"
    }
}

def get-ansi-color [color: string] {
    match $color {
        "cyan" => (ansi cyan)
        "magenta" => (ansi magenta)
        "green" => (ansi green)
        "yellow" => (ansi yellow)
        "blue" => (ansi blue)
        "red" => (ansi red)
        "white" => (ansi white)
        _ => (ansi cyan)
    }
}

# ============ Main Command ============

# Interactive directory selector for tmux
# Usage: qs        - select and cd into directory
#        qs --tmux - open in new tmux window (for popup use)
#        qs --debug - show debug info and wait
export def --env qs [--tmux (-t), --debug (-d)] {
    if $debug {
        print $"(ansi yellow)DEBUG: qs started(ansi reset)"
        print $"  PWD: ($env.PWD)"
        print $"  TERM: ($env.TERM? | default 'not set')"
        print $"  TMUX: ($env.TMUX? | default 'not set')"
        print ""
    }
    let config = (get-config)
    let cache_file = ($"($config.cache_dir)/history.nuon" | path expand)
    
    # Ensure cache directory exists
    mkdir ($cache_file | path dirname)
    
    # Load history or create empty record
    let history = if ($cache_file | path exists) {
        open $cache_file
    } else {
        {}
    }

    # Scan all configured directories
    let all_projects = ($config.directories | each {|dir|
        let expanded_path = ($dir.path | path expand)
        if ($expanded_path | path exists) {
            ls $expanded_path | where type == dir | each {|it| 
                { 
                    name: ($it.name | path basename)
                    path: ($it.name | path expand)
                    label: $dir.label
                    color: $dir.color
                }
            }
        } else {
            []
        }
    } | flatten)

    # Add last_used timestamp and sort
    let projects = ($all_projects | each {|proj|
        let last_used = ($history | get -o $proj.path | default null)
        $proj | insert last_used $last_used
    } | sort-by last_used --reverse)

    # Count projects per group
    let group_counts = ($config.directories | each {|dir|
        let count = ($projects | where label == $dir.label | length)
        { label: $dir.label, count: $count, color: $dir.color }
    })

    # Homebrew-style header
    let line = "══════════════════════════════════════════════════════════"
    print $"(ansi green)($line)(ansi reset)"
    print $"(ansi green_bold)  ($config.ui.icon) ($config.ui.title)(ansi reset)"
    print $"(ansi green)($line)(ansi reset)"
    print ""
    
    # Show group counts
    let counts_str = ($group_counts | each {|g|
        $"(get-ansi-color $g.color)($g.label):(ansi reset) ($g.count)"
    } | str join "    ")
    print $"  ($counts_str)"
    print ""

    # Build display list
    let display_list = ($projects | each {|proj|
        let prefix = $"(get-ansi-color $proj.color)($proj.label)(ansi reset)"
        let time_str = if $proj.last_used != null {
            $"(ansi dark_gray)(format-ago $proj.last_used)(ansi reset)"
        } else {
            $"(ansi dark_gray)-(ansi reset)"
        }
        let padded_name = ($proj.name | fill -w $config.ui.width)
        { 
            display: $"($prefix)  ($padded_name) ($time_str)"
            path: $proj.path 
            name: $proj.name
        }
    })

    # Show interactive selection menu
    let selection = ($display_list | input list --display display --fuzzy $"(ansi yellow)Select:(ansi reset)")

    if ($selection | is-not-empty) {
        # Update history
        let new_history = ($history | upsert $selection.path (date now | format date "%+"))
        $new_history | save -f $cache_file

        if $tmux {
            # Open in new tmux window with directory name
            if ($config.command | is-empty) {
                tmux new-window -n $selection.name -c $selection.path
            } else {
                tmux new-window -n $selection.name -c $selection.path $"nu -e '($config.command)'"
            }
        } else {
            print ""
            print $"(ansi green)($line)(ansi reset)"
            print $"(ansi green)  ✓(ansi reset) Selected (ansi white_bold)($selection.name)(ansi reset)"
            print $"(ansi dark_gray)  → ($selection.path)(ansi reset)"
            print $"(ansi green)($line)(ansi reset)"
            cd $selection.path
            
            # Run the configured command if set
            if ($config.command | is-not-empty) {
                nu -c $config.command
            }
        }
    }
}
