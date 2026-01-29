use crate::config::MergedConfig;
use crate::error::AppError;
use crate::proton::locate_proton;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use tracing::{debug, info};

/// Write a message to the debug log file
fn log_to_file(message: &str) {
    if let Ok(home) = std::env::var("HOME") {
        let log_path = PathBuf::from(&home).join(".steam-command-runner.log");
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let _ = writeln!(file, "[{}] {}", timestamp, message);
        }
    }
}

/// Get the Steam overlay library paths for LD_PRELOAD
fn get_steam_overlay_paths() -> Option<String> {
    // Try to find Steam installation path
    let home = std::env::var("HOME").ok()?;
    let steam_path = PathBuf::from(&home).join(".local/share/Steam");

    let overlay_64 = steam_path.join("ubuntu12_64/gameoverlayrenderer.so");
    let overlay_32 = steam_path.join("ubuntu12_32/gameoverlayrenderer.so");

    debug!("Checking for Steam overlay libraries:");
    debug!("  64-bit: {} (exists: {})", overlay_64.display(), overlay_64.exists());
    debug!("  32-bit: {} (exists: {})", overlay_32.display(), overlay_32.exists());

    if overlay_64.exists() {
        let mut paths = overlay_64.to_string_lossy().to_string();
        if overlay_32.exists() {
            paths.push(':');
            paths.push_str(&overlay_32.to_string_lossy());
        }
        debug!("Steam overlay paths: {}", paths);
        Some(paths)
    } else {
        debug!("Steam overlay 64-bit library not found!");
        None
    }
}

/// Build LD_PRELOAD value with Steam overlay added
fn build_ld_preload_with_overlay() -> Option<String> {
    let overlay_paths = get_steam_overlay_paths()?;

    // Check existing LD_PRELOAD
    let existing_preload = std::env::var("LD_PRELOAD").ok();
    debug!("Existing LD_PRELOAD: {:?}", existing_preload);

    if let Some(existing) = existing_preload {
        if existing.contains("gameoverlayrenderer.so") {
            debug!("LD_PRELOAD already contains gameoverlayrenderer.so, keeping as-is");
            Some(existing)
        } else {
            let new_preload = format!("{}:{}", overlay_paths, existing);
            debug!("Prepending overlay to existing LD_PRELOAD: {}", new_preload);
            Some(new_preload)
        }
    } else {
        debug!("No existing LD_PRELOAD, setting to overlay paths only");
        Some(overlay_paths)
    }
}

/// Log all relevant Steam environment variables for debugging
fn log_steam_env_vars() {
    let vars = [
        "LD_PRELOAD",
        "LD_LIBRARY_PATH",
        "STEAM_COMPAT_DATA_PATH",
        "STEAM_COMPAT_CLIENT_INSTALL_PATH",
        "SteamAppId",
        "SteamGameId",
        "STEAM_COMPAT_TOOL_PATHS",
        "STEAM_RUNTIME",
        "STEAM_RUNTIME_LIBRARY_PATH",
        "PRESSURE_VESSEL_FILESYSTEMS_RO",
        "PROTON_LOG",
        "XDG_CURRENT_DESKTOP",
        "DISPLAY",
        "WAYLAND_DISPLAY",
        "HOME",
        // Gamescope/Steam Input related
        "LIBEI_SOCKET",
        "STEAM_GAMESCOPE_NIS_SUPPORTED",
        "STEAM_GAMESCOPE_HDR_SUPPORTED",
        "STEAM_GAMESCOPE_VRR_SUPPORTED",
        "STEAM_GAMESCOPE_TEARING_SUPPORTED",
        "STEAM_GAMESCOPE_HAS_TEARING_SUPPORT",
        "STEAM_GAME_DISPLAY_0",
        "GAMESCOPE_WAYLAND_DISPLAY",
        "ENABLE_GAMESCOPE_WSI",
    ];

    log_to_file("=== Steam Environment Variables ===");
    info!("=== Steam Environment Variables ===");
    for var in vars {
        match std::env::var(var) {
            Ok(val) => {
                let msg = format!("  {}={}", var, val);
                log_to_file(&msg);
                info!("{}", msg);
            }
            Err(_) => {
                let msg = format!("  {} (not set)", var);
                log_to_file(&msg);
                debug!("{}", msg);
            }
        }
    }
    log_to_file("=== End Steam Environment Variables ===");
    info!("=== End Steam Environment Variables ===");
}

/// Runner for games using Proton/Wine
pub struct ProtonRunner<'a> {
    config: &'a MergedConfig,
    proton_path: PathBuf,
}

impl<'a> ProtonRunner<'a> {
    pub fn new(config: &'a MergedConfig) -> Result<Self, AppError> {
        let proton_path = locate_proton(config.proton.as_deref())?;
        info!("Using Proton at: {}", proton_path.display());

        Ok(Self {
            config,
            proton_path,
        })
    }

    pub fn run(&self, command: Vec<String>) -> Result<ExitCode, AppError> {
        log_to_file("========================================");
        log_to_file("ProtonRunner::run() starting");
        info!("ProtonRunner starting");
        log_steam_env_vars();

        let config_msg = format!("Config: gamescope_enabled={}, is_gamescope_session={}",
              self.config.gamescope_enabled, self.config.is_gamescope_session);
        info!("{}", config_msg);
        log_to_file(&config_msg);

        // Build the Proton command
        let mut full_command = Vec::new();

        // Track if we're adding gamescope (needed for LD_PRELOAD handling)
        let mut using_gamescope = false;

        // Add gamescope wrapper if enabled and not already in a gamescope session
        if self.config.gamescope_enabled {
            if self.config.is_gamescope_session {
                debug!("Already in gamescope session, skipping gamescope wrapper");
            } else if let Some(ref gs_args) = self.config.gamescope_args {
                let gs_args_parsed = shlex::split(gs_args)
                    .ok_or_else(|| AppError::GamescopeArgsParse(gs_args.to_string()))?;

                debug!("Wrapping with gamescope: {:?}", gs_args_parsed);
                log_to_file(&format!("Wrapping with gamescope: {:?}", gs_args_parsed));

                full_command.push("gamescope".to_string());
                full_command.extend(gs_args_parsed);
                full_command.push("--".to_string());

                // When using gamescope, we need to ensure Steam overlay Vulkan layer is enabled
                // and gamescope WSI is enabled for proper Steam Input integration
                log_to_file("Adding env command to enable Steam overlay Vulkan layer for gamescope");
                full_command.push("env".to_string());

                // Enable the Steam overlay Vulkan layer
                full_command.push("ENABLE_VK_LAYER_VALVE_steam_overlay_1=1".to_string());

                // Enable gamescope WSI (Window System Integration)
                full_command.push("ENABLE_GAMESCOPE_WSI=1".to_string());

                // Also pass LD_PRELOAD for legacy overlay support
                if let Some(ld_preload) = build_ld_preload_with_overlay() {
                    log_to_file(&format!("Also adding LD_PRELOAD: {}", ld_preload));
                    full_command.push(format!("LD_PRELOAD={}", ld_preload));
                }

                using_gamescope = true;
            }
        }

        // Add pre-command if configured
        if let Some(pre_cmd) = self.config.effective_pre_command() {
            let pre_args = shlex::split(pre_cmd)
                .ok_or_else(|| AppError::PreCommandParse(pre_cmd.to_string()))?;
            full_command.extend(pre_args);
        }

        // Add Proton executable
        let proton_exe = self.proton_path.join("proton");
        full_command.push(proton_exe.to_string_lossy().to_string());

        // Add verb (waitforexitandrun is the standard)
        full_command.push("waitforexitandrun".to_string());

        // Add game command and args
        full_command.extend(command);

        // Add launch args
        full_command.extend(self.config.launch_args.clone());

        // Extract command and args
        let (cmd, args) = full_command.split_first()
            .ok_or(AppError::NoCommand)?;

        info!("Executing via Proton: {} {:?}", cmd, args);

        // Build command with environment variables
        let mut process = Command::new(cmd);
        process.args(args);

        // Set required Proton environment variables
        if let Ok(compat_data) = std::env::var("STEAM_COMPAT_DATA_PATH") {
            process.env("STEAM_COMPAT_DATA_PATH", &compat_data);
            debug!("STEAM_COMPAT_DATA_PATH={}", compat_data);
        }

        if let Ok(client_path) = std::env::var("STEAM_COMPAT_CLIENT_INSTALL_PATH") {
            process.env("STEAM_COMPAT_CLIENT_INSTALL_PATH", &client_path);
            debug!("STEAM_COMPAT_CLIENT_INSTALL_PATH={}", client_path);
        }

        // Set user-configured environment variables
        for (key, value) in &self.config.env {
            debug!("Setting env: {}={}", key, value);
            process.env(key, value);
        }

        // Set Steam overlay environment variables on the process itself
        // This is critical: gamescope needs to inherit these so the overlay is loaded
        // into gamescope, not just the game. Steam does this when it sees gamescope
        // in launch options.
        log_to_file(&format!("LD_PRELOAD handling: using_gamescope={}", using_gamescope));
        info!("LD_PRELOAD handling: using_gamescope={}", using_gamescope);

        if using_gamescope {
            // Set LD_PRELOAD on the process so gamescope loads the overlay
            if let Some(ld_preload) = build_ld_preload_with_overlay() {
                log_to_file(&format!("Setting LD_PRELOAD on gamescope process: {}", ld_preload));
                info!("Setting LD_PRELOAD on gamescope process: {}", ld_preload);
                process.env("LD_PRELOAD", &ld_preload);
            }

            // Set Vulkan layer and WSI vars on the process too
            log_to_file("Setting ENABLE_VK_LAYER_VALVE_steam_overlay_1=1 on process");
            process.env("ENABLE_VK_LAYER_VALVE_steam_overlay_1", "1");
            log_to_file("Setting ENABLE_GAMESCOPE_WSI=1 on process");
            process.env("ENABLE_GAMESCOPE_WSI", "1");

            // Set STEAM_GAMESCOPE_* variables that Steam sets when it detects gamescope
            // These may be needed for the overlay to enable gamescope-specific input handling
            log_to_file("Setting STEAM_GAMESCOPE_* feature flags");
            process.env("STEAM_GAMESCOPE_NIS_SUPPORTED", "1");
            process.env("STEAM_GAMESCOPE_HDR_SUPPORTED", "1");
            process.env("STEAM_GAMESCOPE_VRR_SUPPORTED", "1");
            process.env("STEAM_GAMESCOPE_TEARING_SUPPORTED", "1");
            process.env("STEAM_GAMESCOPE_HAS_TEARING_SUPPORT", "1");
        } else if self.config.is_gamescope_session {
            // We're inside gamescope (either native session or launched by our wrapper)
            // We still need to set LD_PRELOAD so gameoverlayrenderer.so connects to LIBEI_SOCKET
            if let Some(ld_preload) = build_ld_preload_with_overlay() {
                log_to_file(&format!("In gamescope session, setting LD_PRELOAD: {}", ld_preload));
                info!("In gamescope session, setting LD_PRELOAD: {}", ld_preload);
                process.env("LD_PRELOAD", &ld_preload);
            }

            // Also set the Vulkan layer and WSI vars
            log_to_file("Setting ENABLE_VK_LAYER_VALVE_steam_overlay_1=1 for gamescope session");
            process.env("ENABLE_VK_LAYER_VALVE_steam_overlay_1", "1");
            process.env("ENABLE_GAMESCOPE_WSI", "1");
        }

        // Use exec to replace this process entirely
        // This is important for Steam Input to work properly - Steam Input
        // attaches to the process it launches, and using exec ensures the
        // game IS that process rather than a child of it.
        log_to_file("=== Final command to exec ===");
        log_to_file(&format!("Command: {} {:?}", cmd, args));
        log_to_file("=== About to exec (this process will be replaced) ===");
        info!("=== Final command to exec ===");
        info!("Command: {} {:?}", cmd, args);
        info!("=== About to exec (this process will be replaced) ===");

        let err = process.exec();

        // If exec returns, it failed
        Err(AppError::ExecutionFailed(format!("exec failed: {}", err)))
    }
}
