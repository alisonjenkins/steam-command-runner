use crate::config::MergedConfig;
use std::env;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Check if the current binary was invoked as "gamescope"
pub fn is_invoked_as_gamescope() -> bool {
    std::env::args()
        .next()
        .map(|arg0| {
            Path::new(&arg0)
                .file_name()
                .map(|name| name == "gamescope")
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

/// Parse gamescope arguments, splitting at "--" into (gamescope_args, command)
fn parse_gamescope_args(args: Vec<String>) -> (Vec<String>, Vec<String>) {
    let mut gamescope_args = Vec::new();
    let mut command = Vec::new();
    let mut found_separator = false;

    for arg in args.into_iter().skip(1) {
        // Skip argv[0]
        if !found_separator && arg == "--" {
            found_separator = true;
            continue;
        }

        if found_separator {
            command.push(arg);
        } else {
            gamescope_args.push(arg);
        }
    }

    (gamescope_args, command)
}

/// Get the Steam App ID from environment
fn get_app_id() -> Option<u32> {
    env::var("SteamAppId")
        .ok()
        .and_then(|s| s.parse().ok())
}

/// Get gamescope args from config
fn get_config_gamescope_args() -> Vec<String> {
    let app_id = get_app_id();

    // Try to load config - if it fails, just return empty args
    let config = match MergedConfig::load(app_id, None) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    // Check if gamescope is enabled
    if !config.gamescope_enabled {
        return Vec::new();
    }

    // Parse and return gamescope args
    match config.gamescope_args {
        Some(args_str) => shlex::split(&args_str).unwrap_or_default(),
        None => Vec::new(),
    }
}

/// Find the real gamescope binary, excluding ourselves
fn find_real_gamescope() -> Option<PathBuf> {
    // Get our own inode to exclude from search
    let self_path = std::env::current_exe().ok()?;
    let self_inode = fs::metadata(&self_path).ok()?.ino();

    // Search PATH for gamescope
    let path_env = std::env::var("PATH").ok()?;

    for dir in path_env.split(':') {
        let candidate = Path::new(dir).join("gamescope");

        if !candidate.exists() {
            continue;
        }

        // Check if it's a different file (by inode) to skip our symlink
        if let Ok(metadata) = fs::metadata(&candidate) {
            // Follow symlinks to get the real file
            if let Ok(canonical) = fs::canonicalize(&candidate) {
                if let Ok(canonical_meta) = fs::metadata(&canonical) {
                    if canonical_meta.ino() != self_inode {
                        return Some(candidate);
                    }
                }
            } else if metadata.ino() != self_inode {
                return Some(candidate);
            }
        }
    }

    None
}

/// Handle execution when invoked as the gamescope shim
pub fn handle_gamescope_shim() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let (cli_gamescope_args, command) = parse_gamescope_args(args);

    // Get gamescope args from config and merge with CLI args
    // Config args come first, CLI args can override
    let config_args = get_config_gamescope_args();
    let mut all_gamescope_args = config_args;
    all_gamescope_args.extend(cli_gamescope_args);

    // Find the real gamescope binary
    let real_gamescope = match find_real_gamescope() {
        Some(path) => path,
        None => {
            eprintln!("Error: Real gamescope binary not found in PATH");
            eprintln!("Make sure gamescope is installed and the steam-command-runner symlink");
            eprintln!("is not shadowing the real gamescope binary.");
            return ExitCode::FAILURE;
        }
    };

    // Use exec to replace the current process
    // This preserves all environment variables set by Steam (including LIBEI_SOCKET, LD_PRELOAD)
    use std::os::unix::process::CommandExt;

    let mut cmd = std::process::Command::new(&real_gamescope);
    cmd.args(&all_gamescope_args);

    if !command.is_empty() {
        cmd.arg("--");
        cmd.args(&command);
    }

    // exec() replaces the current process - this never returns on success
    let err = cmd.exec();
    eprintln!("Error: Failed to exec gamescope: {}", err);
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gamescope_args_with_command() {
        let args = vec![
            "gamescope".to_string(),
            "-w".to_string(),
            "1920".to_string(),
            "-h".to_string(),
            "1080".to_string(),
            "--".to_string(),
            "/path/to/game".to_string(),
            "arg1".to_string(),
        ];

        let (gs_args, cmd) = parse_gamescope_args(args);

        assert_eq!(gs_args, vec!["-w", "1920", "-h", "1080"]);
        assert_eq!(cmd, vec!["/path/to/game", "arg1"]);
    }

    #[test]
    fn test_parse_gamescope_args_no_command() {
        let args = vec![
            "gamescope".to_string(),
            "-f".to_string(),
            "--fullscreen".to_string(),
        ];

        let (gs_args, cmd) = parse_gamescope_args(args);

        assert_eq!(gs_args, vec!["-f", "--fullscreen"]);
        assert!(cmd.is_empty());
    }

    #[test]
    fn test_parse_gamescope_args_empty() {
        let args = vec!["gamescope".to_string()];

        let (gs_args, cmd) = parse_gamescope_args(args);

        assert!(gs_args.is_empty());
        assert!(cmd.is_empty());
    }
}
