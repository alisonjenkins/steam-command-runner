use crate::compat_tool::install::uninstall_compat_tool;
use crate::error::AppError;
use std::path::PathBuf;
use tracing::info;

/// Handle the uninstall command - remove Steam compatibility tool installation
pub fn handle_uninstall(steam_path: Option<PathBuf>) -> Result<(), AppError> {
    info!("Uninstalling compatibility tool");

    uninstall_compat_tool(steam_path)?;

    println!("Successfully uninstalled steam-command-runner from Steam.");

    Ok(())
}
