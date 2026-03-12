# typed: false
# frozen_string_literal: true

class TmuxQuickselect < Formula
  desc "Fast, interactive directory selector for tmux"
  homepage "https://github.com/cvrt-jh/tmux-quickselect"
  url "https://github.com/cvrt-jh/tmux-quickselect.git",
      tag:      "v1.0.5",
      revision: "abc123def456"
  license "MIT"
  head "https://github.com/cvrt-jh/tmux-quickselect.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", "--locked", "--root", prefix, "--path", "."
    (libexec/"plugins").install Dir["plugins/*.rs"]
    (share/"tmux-quickselect").install "config.toml"
  end

  def caveats
    <<~EOS
      To use tmux-quickselect, add the following to your shell config:

      For Nushell (~/.config/nushell/config.nu):
        use #{libexec}/qs.nu

      For Bash/Zsh (~/.bashrc, ~/.zshrc):
        source #{libexec}/qs.sh

      Add to your tmux config (~/.tmux.conf):
        bind-key O display-popup -E -w 70% -h 60% "qs --tmux"

      Configuration file location:
        #{share}/tmux-quickselect/config.toml
    EOS
  end

  test do
    assert_predicate bin/"qs", :exist?
  end
end
