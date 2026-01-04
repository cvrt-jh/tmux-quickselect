# qs.nu
# tmux-quickselect: Interactive directory launcher for tmux with Nushell
# https://github.com/cvrt-jh/tmux-quickselect

# ============ Configuration ============

const CONFIG_FILE = "~/.config/tmux-quickselect/config.nuon"

def get-config [] {
    let config_file = ($CONFIG_FILE | path expand)
    
    if not ($config_file | path exists) {
        # Default configuration
        {
            directories: [
                { path: "~/Git", label: "git", color: "cyan" }
            ]
            command: ""
            sort: "recent"
            cache_dir: "~/.cache/tmux-quickselect"
            ui: { title: "Quick Select", icon: "📂", width: 25 }
        }
    } else {
        open $config_file
    }
}

def save-config [config: record] {
    let config_file = ($CONFIG_FILE | path expand)
    mkdir ($config_file | path dirname)
    $config | to nuon | save -f $config_file
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

    # Add last_used timestamp
    let projects_with_history = ($all_projects | each {|proj|
        let last_used = ($history | get -o $proj.path | default null)
        $proj | insert last_used $last_used
    })

    # Sort based on config (default: recent)
    # Can be a string ("recent", "alphabetical", "label") or a list ["label", "recent"]
    let sort_config = ($config | get -o sort | default "recent")
    let sort_keys = if ($sort_config | describe | str starts-with "list") {
        $sort_config
    } else {
        [$sort_config]
    }
    
    # Apply sorting - process keys in reverse order for correct precedence
    let projects = ($sort_keys | reverse | reduce --fold $projects_with_history {|key, acc|
        match $key {
            "recent" => {
                # Recent first: items with timestamp sorted by date, then items without
                let with_ts = ($acc | where last_used != null | sort-by last_used --reverse)
                let without_ts = ($acc | where last_used == null)
                $with_ts | append $without_ts
            }
            "alphabetical" | "name" => {
                $acc | sort-by name
            }
            "label" => {
                $acc | sort-by label
            }
            _ => { $acc }
        }
    })

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

    # Build display list for projects
    let project_list = ($projects | each {|proj|
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
            type: "project"
        }
    })

    # Config menu items
    let sort_display = if ($sort_keys | length) == 1 {
        $sort_keys | first
    } else {
        $sort_keys | str join " → "
    }
    let config_items = [
        { display: $"(ansi dark_gray)──────────────────────────────────────(ansi reset)", type: "separator", action: "" }
        { display: $"(ansi yellow)⚙(ansi reset)  Sort: (ansi white_bold)($sort_display)(ansi reset)", type: "config", action: "sort" }
        { display: $"(ansi yellow)⚙(ansi reset)  Command: (ansi white_bold)(if ($config.command | is-empty) { '(none)' } else { $config.command })(ansi reset)", type: "config", action: "command" }
        { display: $"(ansi red)✕(ansi reset)  Clear history", type: "config", action: "clear_history" }
    ]

    let display_list = ($project_list | append $config_items)

    # Show interactive selection menu
    let selection = ($display_list | input list --display display --fuzzy $"(ansi yellow)Select:(ansi reset)")

    if ($selection | is-not-empty) {
        match $selection.type {
            "project" => {
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
            "config" => {
                match $selection.action {
                    "sort" => {
                        print ""
                        print $"(ansi yellow)Configure sort order:(ansi reset)"
                        print $"(ansi dark_gray)Current: ($sort_display)(ansi reset)"
                        print ""
                        print "  1 = recent (last used first)"
                        print "  2 = alphabetical (A-Z)"
                        print "  3 = label (grouped by label)"
                        print ""
                        print $"(ansi dark_gray)Enter numbers in order, e.g. '31' = label then recent(ansi reset)"
                        let input = (input "Sort order: ")
                        
                        if ($input | is-not-empty) {
                            let sort_map = { "1": "recent", "2": "alphabetical", "3": "label" }
                            let new_sort = ($input | split chars | each {|c| $sort_map | get -o $c } | where { $in != null })
                            
                            if ($new_sort | is-not-empty) {
                                let sort_value = if ($new_sort | length) == 1 { $new_sort | first } else { $new_sort }
                                let new_config = ($config | upsert sort $sort_value)
                                save-config $new_config
                                let display = if ($new_sort | length) == 1 { $new_sort | first } else { $new_sort | str join " → " }
                                print $"(ansi green)✓(ansi reset) Sort order set to (ansi white_bold)($display)(ansi reset)"
                            } else {
                                print $"(ansi red)✗(ansi reset) Invalid input"
                            }
                        }
                    }
                    "command" => {
                        print ""
                        let current = if ($config.command | is-empty) { "" } else { $config.command }
                        let current_display = if ($current | is-empty) { "(none)" } else { $current }
                        print $"(ansi yellow)Current command:(ansi reset) ($current_display)"
                        print $"(ansi dark_gray)Enter new command, empty for just cd:(ansi reset)"
                        let new_cmd = (input "Command: ")
                        let new_config = ($config | upsert command $new_cmd)
                        save-config $new_config
                        let new_display = if ($new_cmd | is-empty) { "(none)" } else { $new_cmd }
                        print $"(ansi green)✓(ansi reset) Command set to (ansi white_bold)($new_display)(ansi reset)"
                    }
                    "clear_history" => {
                        {} | save -f $cache_file
                        print $"(ansi green)✓(ansi reset) History cleared"
                    }
                }
            }
            "separator" => {
                # Do nothing for separator
            }
        }
    }
}
