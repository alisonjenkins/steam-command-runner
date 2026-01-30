use crate::config::MergedConfig;
use std::env;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::io::Write;

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
/// Load the full merged configuration
fn load_config() -> Option<MergedConfig> {
    let app_id = get_app_id();
    MergedConfig::load(app_id, None).ok()
}

/// Handle execution when invoked as the gamescope shim
pub fn handle_gamescope_shim() -> ExitCode {
    // Load config first to check logging preference
    let config = load_config();
    let debug_enabled = config.as_ref().map(|c| c.shim_debug).unwrap_or(false);

    log_to_file("Shim started", debug_enabled);
    let args: Vec<String> = std::env::args().collect();
    log_to_file(&format!("Args: {:?}", args), debug_enabled);
    let (cli_gamescope_args, command) = parse_gamescope_args(args);

    // Get gamescope args from config
    let config_gamescope_args = if let Some(c) = &config {
        if c.gamescope_enabled {
             match &c.gamescope_args {
                Some(args_str) => shlex::split(args_str).unwrap_or_default(),
                None => Vec::new(),
            }
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let mut all_gamescope_args = config_gamescope_args;
    all_gamescope_args.extend(cli_gamescope_args);

    // Find the real gamescope binary
    let real_gamescope = match find_real_gamescope() {
        Some(path) => {
            log_to_file(&format!("Found real gamescope at: {:?}", path), debug_enabled);
            path
        },
        None => {
            log_to_file("Error: Real gamescope binary not found in PATH", debug_enabled);
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
    log_to_file(&format!("Executing: {:?} args: {:?}", real_gamescope, all_gamescope_args), debug_enabled);

    // Apply environment variables from config
    if let Some(c) = &config {
        for (key, value) in &c.env {
            log_to_file(&format!("Setting env: {}={}", key, value), debug_enabled);
            cmd.env(key, value);
        }
    }

    // We CANNOT successfully set LD_PRELOAD on the gamescope process itself
    // because gamescope has capabilities (cap_sys_nice) which causes the OS to strip insecure env vars.
    // Instead, we must inject it into the INNER command using 'env'.

    // Set Gamescope Overlay variables (These are likely safe from stripping or gamescope might use them)
    log_to_file("Setting ENABLE_VK_LAYER_VALVE_steam_overlay_1=1", debug_enabled);
    cmd.env("ENABLE_VK_LAYER_VALVE_steam_overlay_1", "1");
    
    log_to_file("Setting ENABLE_GAMESCOPE_WSI=1", debug_enabled);
    cmd.env("ENABLE_GAMESCOPE_WSI", "1");

    // Copy STEAM_GAMESCOPE_* env vars
    cmd.env("STEAM_GAMESCOPE_NIS_SUPPORTED", "1");
    cmd.env("STEAM_GAMESCOPE_HDR_SUPPORTED", "1");
    cmd.env("STEAM_GAMESCOPE_VRR_SUPPORTED", "1");
    cmd.env("STEAM_GAMESCOPE_TEARING_SUPPORTED", "1");
    cmd.env("STEAM_GAMESCOPE_HAS_TEARING_SUPPORT", "1");

    if !command.is_empty() {
        cmd.arg("--");
        
        // Inject Steam Overlay via env wrapper in inner command
        if let Some(ld_preload) = build_ld_preload_with_overlay(debug_enabled) {
            log_to_file(&format!("Injecting LD_PRELOAD via inner 'env' wrapper: {}", ld_preload), debug_enabled);
            cmd.arg("env");
            cmd.arg(format!("LD_PRELOAD={}", ld_preload));
        }

        // Inject pre_command (e.g., mangohud) into inner command
        // This ensures it runs AFTER gamescope has started, avoiding capability stripping
        if let Some(c) = &config {
            if let Some(pre_cmd) = c.effective_pre_command() {
                log_to_file(&format!("Injecting pre_command: {}", pre_cmd), debug_enabled);
                if let Some(pre_args) = shlex::split(pre_cmd) {
                    cmd.args(pre_args);
                }
            }
        }

        cmd.args(&command);
    }

    // exec() replaces the current process - this never returns on success
    let err = cmd.exec();
    log_to_file(&format!("Error: Failed to exec gamescope: {}", err), debug_enabled);
    eprintln!("Error: Failed to exec gamescope: {}", err);
    ExitCode::FAILURE
}

/// Get the Steam overlay library paths for LD_PRELOAD
fn get_steam_overlay_paths(debug: bool) -> Option<String> {
    // Try to find Steam installation path
    let home = std::env::var("HOME").ok()?;
    let steam_path = PathBuf::from(&home).join(".local/share/Steam");

    let overlay_64 = steam_path.join("ubuntu12_64/gameoverlayrenderer.so");
    let overlay_32 = steam_path.join("ubuntu12_32/gameoverlayrenderer.so");

    if overlay_64.exists() {
        let mut paths = overlay_64.to_string_lossy().to_string();
        if overlay_32.exists() {
            paths.push(':');
            paths.push_str(&overlay_32.to_string_lossy());
        }
        log_to_file(&format!("Found Steam overlay paths: {}", paths), debug);
        Some(paths)
    } else {
        log_to_file("Steam overlay 64-bit library not found!", debug);
        None
    }
}

/// Build LD_PRELOAD value with Steam overlay added
fn build_ld_preload_with_overlay(debug: bool) -> Option<String> {
    let overlay_paths = get_steam_overlay_paths(debug)?;

    // Check existing LD_PRELOAD
    let existing_preload = std::env::var("LD_PRELOAD").ok();
    
    if let Some(existing) = existing_preload {
        if existing.contains("gameoverlayrenderer.so") {
            log_to_file("LD_PRELOAD already contains gameoverlayrenderer.so, mimicking it", debug);
            Some(existing)
        } else {
            let new_preload = format!("{}:{}", overlay_paths, existing);
            log_to_file(&format!("Prepending overlay to existing LD_PRELOAD"), debug);
            Some(new_preload)
        }
    } else {
        log_to_file("Setting new LD_PRELOAD with overlay", debug);
        Some(overlay_paths)
    }
}

fn log_to_file(message: &str, enabled: bool) {
    if !enabled {
        return;
    }
    if let Ok(home) = std::env::var("HOME") {
        let log_path = PathBuf::from(&home).join(".steam-command-runner-shim.log");
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            let _ = writeln!(file, "{}", message);
        }
    }
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
