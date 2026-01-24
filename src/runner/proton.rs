use crate::config::MergedConfig;
use crate::error::AppError;
use crate::proton::locate_proton;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use tracing::{debug, info};

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
        // Build the Proton command
        let mut full_command = Vec::new();

        // Add gamescope wrapper if configured and not already in a gamescope session
        if let Some(ref gs_args) = self.config.gamescope_args {
            if !self.config.is_gamescope_session {
                let gs_args_parsed = shlex::split(gs_args)
                    .ok_or_else(|| AppError::GamescopeArgsParse(gs_args.to_string()))?;

                debug!("Wrapping with gamescope: {:?}", gs_args_parsed);

                full_command.push("gamescope".to_string());
                full_command.extend(gs_args_parsed);
                full_command.push("--".to_string());
            } else {
                debug!("Already in gamescope session, skipping gamescope wrapper");
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

        // Execute and wait
        let status = process.status()?;

        let exit_code = status.code().unwrap_or(1) as u8;
        info!("Proton process exited with code: {}", exit_code);

        Ok(ExitCode::from(exit_code))
    }
}
