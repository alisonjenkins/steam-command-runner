use crate::config::MergedConfig;
use crate::error::AppError;
use std::process::{Command, ExitCode};
use tracing::{debug, info};

/// Runner for native Linux games
pub struct NativeRunner<'a> {
    config: &'a MergedConfig,
}

impl<'a> NativeRunner<'a> {
    pub fn new(config: &'a MergedConfig) -> Self {
        Self { config }
    }

    pub fn run(&self, mut command: Vec<String>) -> Result<ExitCode, AppError> {
        // Add gamescope wrapper if configured and not already in a gamescope session
        if let Some(ref gs_args) = self.config.gamescope_args {
            if !self.config.is_gamescope_session {
                let gs_args_parsed = shlex::split(gs_args)
                    .ok_or_else(|| AppError::GamescopeArgsParse(gs_args.to_string()))?;

                debug!("Wrapping with gamescope: {:?}", gs_args_parsed);

                // Build gamescope command: gamescope [args] -- [command]
                let mut gs_command = vec!["gamescope".to_string()];
                gs_command.extend(gs_args_parsed);
                gs_command.push("--".to_string());
                gs_command.extend(command);
                command = gs_command;
            } else {
                debug!("Already in gamescope session, skipping gamescope wrapper");
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

        // Execute and wait
        let status = process.status()?;

        let exit_code = status.code().unwrap_or(1) as u8;
        info!("Process exited with code: {}", exit_code);

        Ok(ExitCode::from(exit_code))
    }
}
