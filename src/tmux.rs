use std::env;
use std::process::Command;

/// Returns true if the process is running inside a tmux session.
pub fn is_inside_tmux() -> bool {
    env::var("TMUX").map(|v| !v.is_empty()).unwrap_or(false)
}

/// Builds the argument list for `tmux new-window`.
pub fn build_new_window_args(name: &str, path: &str, command: Option<&str>) -> Vec<String> {
    let mut args = vec![
        "new-window".to_string(),
        "-n".to_string(),
        name.to_string(),
        "-c".to_string(),
        path.to_string(),
    ];
    if let Some(cmd) = command {
        args.push(cmd.to_string());
    }
    args
}

/// Executes `tmux new-window` to open a new window with the given name and path.
pub fn open_in_tmux(name: &str, path: &str, command: Option<&str>) -> std::io::Result<()> {
    let args = build_new_window_args(name, path, command);
    let status = Command::new("tmux").args(&args).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("tmux exited with status: {status}"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_args_no_command() {
        let args = build_new_window_args("my-project", "/home/user/project", None);
        assert_eq!(
            args,
            vec!["new-window", "-n", "my-project", "-c", "/home/user/project"]
        );
    }

    #[test]
    fn test_build_args_with_command() {
        let args = build_new_window_args("my-project", "/home/user/project", Some("nvim"));
        assert_eq!(
            args,
            vec!["new-window", "-n", "my-project", "-c", "/home/user/project", "nvim"]
        );
    }

    #[test]
    fn test_is_inside_tmux_when_unset() {
        // This test runs outside tmux in CI, so TMUX should not be set
        // We can't easily test the positive case without mocking env
        // Just verify the function doesn't panic
        let _ = is_inside_tmux();
    }
}
