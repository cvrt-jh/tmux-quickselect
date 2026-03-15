# typed: false
# frozen_string_literal: true

class TmuxQuickselect < Formula
  desc "Fast, interactive directory selector for tmux"
  homepage "https://github.com/cvrt-jh/tmux-quickselect"
  url "https://github.com/cvrt-jh/tmux-quickselect.git",
      tag:      "v2.0.0",
      revision: "cc17853954e0e35cd45146b2ea59929818c12da1"
  license "MIT"
  head "https://github.com/cvrt-jh/tmux-quickselect.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
    (share/"tmux-quickselect/shell").install Dir["shell/*"]
    (share/"tmux-quickselect").install "config.toml"
  end

  def caveats
    <<~EOS
      Shell integration (pick one):

      Nushell (~/.config/nushell/config.nu):
        source #{share}/tmux-quickselect/shell/qs.nu

      Bash (~/.bashrc):
        source #{share}/tmux-quickselect/shell/qs.bash

      Zsh (~/.zshrc):
        source #{share}/tmux-quickselect/shell/qs.zsh

      tmux keybinding (tmux.conf):
        bind-key O display-popup -E -w 70% -h 60% "qs --tmux"

      Config file:
        cp #{share}/tmux-quickselect/config.toml ~/.config/tmux-quickselect/config.toml
    EOS
  end

  test do
    assert_match "directory selector", shell_output("#{bin}/qs --help")
  end
end
