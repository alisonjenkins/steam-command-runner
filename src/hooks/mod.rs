use crate::config::HookConfig;
use crate::error::AppError;
use std::process::Command;
use tracing::{debug, info, warn};

/// Execute a hook command
pub fn execute(hook: &HookConfig) -> Result<(), AppError> {
    info!("Executing hook: {}", hook.command);

    let args = shlex::split(&hook.command)
        .ok_or_else(|| AppError::HookFailed(format!("Failed to parse hook command: {}", hook.command)))?;

    if args.is_empty() {
        return Err(AppError::HookFailed("Empty hook command".to_string()));
    }

    let (cmd, cmd_args) = args.split_first().unwrap();

    let mut command = Command::new(cmd);
    command.args(cmd_args);

    // Set working directory if specified
    if let Some(ref dir) = hook.working_dir {
        command.current_dir(dir);
    }

    if hook.wait {
        debug!("Waiting for hook to complete");
        let status = command.status()?;
        if !status.success() {
            let code = status.code().unwrap_or(-1);
            warn!("Hook exited with non-zero status: {}", code);
            return Err(AppError::HookFailed(format!(
                "Hook '{}' exited with status {}",
                hook.command, code
            )));
        }
    } else {
        debug!("Running hook in background (no wait)");
        command.spawn()?;
    }

    Ok(())
}
