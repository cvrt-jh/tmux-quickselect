# tmux-quickselect

[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![tmux](https://img.shields.io/badge/tmux-optional-blue.svg)](https://github.com/tmux/tmux)

A fast, interactive directory selector for tmux.

## Features

- **Fuzzy search** - Find directories instantly by typing
- **Git status indicators** - See dirty (`●`) and clean (`○`) repos at a glance
- **Usage tracking** - Sorted by last used with relative timestamps
- **Drill-down navigation** - Browse nested folders interactively with `→`
- **tmux integration** - Launch in popup, open in new window
- **TOML config** - Simple, readable configuration
- **Shell-independent** - Works with Nushell, Bash, and Zsh

## Installation

### Homebrew (recommended)

```bash
brew install cvrt-jh/tap/tmux-quickselect
```

### From source

```bash
cargo install --git https://github.com/cvrt-jh/tmux-quickselect
```

## Shell Integration

Source the appropriate wrapper for your shell so that `qs` can change your working directory.

**Nushell** — add to `~/.config/nushell/config.nu`:

```nu
source "$(brew --prefix)/share/tmux-quickselect/shell/qs.nu"
```

**Bash** — add to `~/.bashrc`:

```bash
source "$(brew --prefix)/share/tmux-quickselect/shell/qs.bash"
```

**Zsh** — add to `~/.zshrc`:

```zsh
source "$(brew --prefix)/share/tmux-quickselect/shell/qs.zsh"
```

For manual installs, replace the `$(brew --prefix)/share/tmux-quickselect/` prefix with the path to the `shell/` directory in your clone.

## tmux Keybinding

Add to your `tmux.conf` to open the selector with `prefix + O`:

```bash
bind-key O display-popup -E -w 70% -h 60% "qs --tmux"
```

## Configuration

Copy `config.toml` to `~/.config/tmux-quickselect/config.toml` and customize:

```toml
# Sort order: "recent", "alphabetical", "label", or ["label", "recent"]
sort = "recent"

# Command to run in new tmux window (empty = just open shell)
# command = "nvim"

# Show hidden directories (starting with .)
show_hidden = false

# Cache directory for usage history
cache_dir = "~/.cache/tmux-quickselect"

# UI settings
[ui]
title = "Quick Select"
icon = "📂"
width = 25   # column width for directory names

# Directories to scan (subdirectories become selectable)
[[directories]]
path = "~/Git"
label = "git"
color = "cyan"

# [[directories]]
# path = "~/projects"
# label = "proj"
# color = "green"
```

### Options

| Option | Description | Default |
|--------|-------------|---------|
| `sort` | Sort order: `recent`, `alphabetical`, `label`, or array | `recent` |
| `command` | Command to run after selection (empty = open shell) | `""` |
| `show_hidden` | Show directories starting with `.` | `false` |
| `cache_dir` | History storage location | `~/.cache/tmux-quickselect` |
| `ui.title` | Header title | `Quick Select` |
| `ui.icon` | Header icon | `📂` |
| `ui.width` | Column width for names | `25` |
| `directories[].path` | Directory to scan | required |
| `directories[].label` | Short label shown in list | required |
| `directories[].color` | `cyan`, `magenta`, `green`, `yellow`, `blue`, `red` | `cyan` |

## Usage

```bash
qs          # Open selector, cd into selected directory
qs --tmux   # Open selected directory in a new tmux window
```

Press `prefix + O` in tmux to open the selector in a popup.

## Keybindings

| Key | Action |
|-----|--------|
| `↑` / `↓` or `k` / `j` | Navigate up / down |
| `Enter` | Select directory or drill into folder |
| Type any characters | Filter list (fuzzy match) |
| `Esc` | Back to parent / clear filter / quit |
| `q` | Quit without selecting |
| `e` | Open config file in `$EDITOR` |
| `h` | Toggle hidden directories |

## Git Status Indicators

Repositories show a status indicator next to their name:

| Indicator | Meaning |
|-----------|---------|
| `●` (red) | Dirty — uncommitted changes |
| `○` (green) | Clean — working tree is clean |

## Drill-Down Navigation

Directories with subdirectories show a `→` indicator. Press Enter to browse inside:

```
  git  client-a                    →  2h ago
  git  client-b                    →  -
  git  standalone-project             1d ago
```

Press `Esc` or `Backspace` to return to the parent level.

## License

MIT
