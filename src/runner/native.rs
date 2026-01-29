use crate::config::MergedConfig;
use crate::error::AppError;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use tracing::{debug, info};

/// Get the Steam overlay library paths for LD_PRELOAD
fn get_steam_overlay_paths() -> Option<String> {
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
        Some(paths)
    } else {
        None
    }
}

/// Build LD_PRELOAD value with Steam overlay added
fn build_ld_preload_with_overlay() -> Option<String> {
    let overlay_paths = get_steam_overlay_paths()?;

    // Check existing LD_PRELOAD
    if let Ok(existing) = std::env::var("LD_PRELOAD") {
        if existing.contains("gameoverlayrenderer.so") {
            // Already has overlay, return as-is
            Some(existing)
        } else {
            // Prepend overlay paths
            Some(format!("{}:{}", overlay_paths, existing))
        }
    } else {
        // No existing LD_PRELOAD, just use overlay paths
        Some(overlay_paths)
    }
}

/// Runner for native Linux games
pub struct NativeRunner<'a> {
    config: &'a MergedConfig,
}

impl<'a> NativeRunner<'a> {
    pub fn new(config: &'a MergedConfig) -> Self {
        Self { config }
    }

    pub fn run(&self, mut command: Vec<String>) -> Result<ExitCode, AppError> {
        // Track if we're adding gamescope
        let mut using_gamescope = false;

        // Add gamescope wrapper if enabled and not already in a gamescope session
        if self.config.gamescope_enabled {
            if self.config.is_gamescope_session {
                debug!("Already in gamescope session, skipping gamescope wrapper");
            } else if let Some(ref gs_args) = self.config.gamescope_args {
                let gs_args_parsed = shlex::split(gs_args)
                    .ok_or_else(|| AppError::GamescopeArgsParse(gs_args.to_string()))?;

                debug!("Wrapping with gamescope: {:?}", gs_args_parsed);

                // Build gamescope command: gamescope [args] -- env LD_PRELOAD=... [command]
                let mut gs_command = vec!["gamescope".to_string()];
                gs_command.extend(gs_args_parsed);
                gs_command.push("--".to_string());

                // Enable Steam overlay Vulkan layer and gamescope WSI for Steam Input
                debug!("Adding env command to enable Steam overlay Vulkan layer for gamescope");
                gs_command.push("env".to_string());
                gs_command.push("ENABLE_VK_LAYER_VALVE_steam_overlay_1=1".to_string());
                gs_command.push("ENABLE_GAMESCOPE_WSI=1".to_string());

                // Also pass LD_PRELOAD for legacy overlay support
                if let Some(ld_preload) = build_ld_preload_with_overlay() {
                    debug!("Also adding LD_PRELOAD: {}", ld_preload);
                    gs_command.push(format!("LD_PRELOAD={}", ld_preload));
                }

                gs_command.extend(command);
                command = gs_command;
                using_gamescope = true;
            }
        }

        // Add pre-command if configured
        if let Some(pre_cmd) = self.config.effective_pre_command() {
            let pre_args = shlex::split(pre_cmd)
                .ok_or_else(|| AppError::PreCommandParse(pre_cmd.to_string()))?;

            debug!("Prepending pre-command: {:?}", pre_args);

            // Insert pre_command args at the beginning
            for (i, arg) in pre_args.into_iter().enumerate() {
                command.insert(i, arg);
            }
        }

        // Add launch args
        if !self.config.launch_args.is_empty() {
            debug!("Adding launch args: {:?}", self.config.launch_args);
            command.extend(self.config.launch_args.clone());
        }

        // Extract command and args
        let (cmd, args) = command.split_first()
            .ok_or(AppError::NoCommand)?;

        info!("Executing: {} {:?}", cmd, args);

        // Build command with environment variables
        let mut process = Command::new(cmd);
        process.args(args);

        // Set environment variables
        for (key, value) in &self.config.env {
            debug!("Setting env: {}={}", key, value);
            process.env(key, value);
        }

        // Set Steam overlay environment variables on the process itself
        // This is critical: gamescope needs to inherit these so the overlay is loaded
        // into gamescope, not just the game.
        if using_gamescope {
            // Set LD_PRELOAD on the process so gamescope loads the overlay
            if let Some(ld_preload) = build_ld_preload_with_overlay() {
                debug!("Setting LD_PRELOAD on gamescope process: {}", ld_preload);
                process.env("LD_PRELOAD", &ld_preload);
            }

            // Set Vulkan layer and WSI vars on the process too
            debug!("Setting ENABLE_VK_LAYER_VALVE_steam_overlay_1=1 on process");
            process.env("ENABLE_VK_LAYER_VALVE_steam_overlay_1", "1");
            debug!("Setting ENABLE_GAMESCOPE_WSI=1 on process");
            process.env("ENABLE_GAMESCOPE_WSI", "1");

            // Set STEAM_GAMESCOPE_* variables that Steam sets when it detects gamescope
            // These may be needed for the overlay to enable gamescope-specific input handling
            debug!("Setting STEAM_GAMESCOPE_* feature flags");
            process.env("STEAM_GAMESCOPE_NIS_SUPPORTED", "1");
            process.env("STEAM_GAMESCOPE_HDR_SUPPORTED", "1");
            process.env("STEAM_GAMESCOPE_VRR_SUPPORTED", "1");
            process.env("STEAM_GAMESCOPE_TEARING_SUPPORTED", "1");
            process.env("STEAM_GAMESCOPE_HAS_TEARING_SUPPORT", "1");
        } else if self.config.is_gamescope_session {
            // We're inside gamescope (either native session or launched by our wrapper)
            // We still need to set LD_PRELOAD so gameoverlayrenderer.so connects to LIBEI_SOCKET
            if let Some(ld_preload) = build_ld_preload_with_overlay() {
                debug!("In gamescope session, setting LD_PRELOAD: {}", ld_preload);
                process.env("LD_PRELOAD", &ld_preload);
            }

            // Also set the Vulkan layer and WSI vars
            debug!("Setting ENABLE_VK_LAYER_VALVE_steam_overlay_1=1 for gamescope session");
            process.env("ENABLE_VK_LAYER_VALVE_steam_overlay_1", "1");
            process.env("ENABLE_GAMESCOPE_WSI", "1");
        }

        // Use exec to replace this process entirely
        // This is important for Steam Input to work properly
        info!("Exec'ing into game (replacing this process)");
        let err = process.exec();

        // If exec returns, it failed
        Err(AppError::ExecutionFailed(format!("exec failed: {}", err)))
    }
}
