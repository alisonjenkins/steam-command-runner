use crate::error::AppError;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use tracing::{debug, info};

/// Install the gamescope shim symlink
pub fn handle_install(path: Option<PathBuf>) -> Result<(), AppError> {
    // Default to ~/.local/bin/gamescope
    let target_path = path.unwrap_or_else(|| {
        dirs::home_dir()
            .expect("Could not find home directory")
            .join(".local/bin/gamescope")
    });

    // Get current executable path
    let self_path = std::env::current_exe()?;

    // Create parent directory if needed
    if let Some(parent) = target_path.parent() {
        if !parent.exists() {
            debug!("Creating directory: {}", parent.display());
            fs::create_dir_all(parent)?;
        }
    }

    // Remove existing symlink/file if present
    if target_path.exists() || target_path.is_symlink() {
        debug!("Removing existing: {}", target_path.display());
        fs::remove_file(&target_path)?;
    }

    // Create symlink
    debug!(
        "Creating symlink: {} -> {}",
        target_path.display(),
        self_path.display()
    );
    symlink(&self_path, &target_path)?;

    info!("Installed gamescope shim to: {}", target_path.display());
    println!("Installed gamescope shim to: {}", target_path.display());
    println!();
    println!("Make sure {} is in your PATH before /usr/bin", target_path.parent().unwrap().display());
    println!("You can add this to your shell profile:");
    println!("  export PATH=\"{}:$PATH\"", target_path.parent().unwrap().display());

    Ok(())
}

/// Uninstall the gamescope shim symlink
pub fn handle_uninstall(path: Option<PathBuf>) -> Result<(), AppError> {
    let target_path = path.unwrap_or_else(|| {
        dirs::home_dir()
            .expect("Could not find home directory")
            .join(".local/bin/gamescope")
    });

    if !target_path.exists() && !target_path.is_symlink() {
        println!("Gamescope shim not installed at: {}", target_path.display());
        return Ok(());
    }

    // Verify it's our symlink before removing
    if target_path.is_symlink() {
        let link_target = fs::read_link(&target_path)?;

        // Check if it points to steam-command-runner
        if !link_target.to_string_lossy().contains("steam-command-runner") {
            println!("Warning: {} doesn't appear to be our symlink", target_path.display());
            println!("Link target: {}", link_target.display());
            println!("Expected to contain: steam-command-runner");
            return Ok(());
        }
    }

    fs::remove_file(&target_path)?;
    println!("Removed gamescope shim: {}", target_path.display());

    Ok(())
}
