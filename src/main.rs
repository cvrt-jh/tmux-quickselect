use clap::Parser;

#[derive(Parser)]
#[command(name = "qs", version, about = "Fast, interactive directory selector for tmux")]
struct Cli {
    /// Open selection in new tmux window
    #[arg(short, long)]
    tmux: bool,

    /// Start browsing from a specific path
    #[arg(short, long)]
    path: Option<String>,

    /// Show debug info
    #[arg(short, long)]
    debug: bool,
}

fn main() {
    let cli = Cli::parse();
    if cli.debug {
        eprintln!("qs v{} — tmux={}, path={:?}", env!("CARGO_PKG_VERSION"), cli.tmux, cli.path);
    }
}
