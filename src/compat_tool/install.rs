use super::vdf::{generate_compatibilitytool_vdf, generate_toolmanifest_vdf};
use crate::error::AppError;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use tracing::{debug, info};

/// Get the path to Steam's compatibilitytools.d directory
fn get_compat_tools_dir(steam_path: Option<PathBuf>) -> Result<PathBuf, AppError> {
    if let Some(path) = steam_path {
        return Ok(path.join("compatibilitytools.d"));
    }

    // Try common Steam locations
    let candidates: Vec<PathBuf> = [
        dirs::home_dir().map(|h| h.join(".steam/root/compatibilitytools.d")),
        dirs::home_dir().map(|h| h.join(".local/share/Steam/compatibilitytools.d")),
        dirs::data_dir().map(|d| d.join("Steam/compatibilitytools.d")),
    ]
    .into_iter()
    .flatten()
    .collect();

    for candidate in &candidates {
        // Check if parent Steam directory exists
        if let Some(parent) = candidate.parent() {
            if parent.exists() {
                debug!("Found Steam at: {}", parent.display());
                return Ok(candidate.clone());
            }
        }
    }

    Err(AppError::SteamNotFound(candidates))
}

/// Get the path to the current executable
fn get_self_path() -> Result<PathBuf, AppError> {
    std::env::current_exe().map_err(AppError::Io)
}

/// Install as a Steam compatibility tool
pub fn install_compat_tool(
    name: &str,
    steam_path: Option<PathBuf>,
    require_proton: Option<String>,
) -> Result<(), AppError> {
    let compat_dir = get_compat_tools_dir(steam_path)?;
    let tool_dir = compat_dir.join(name);

    info!("Installing to: {}", tool_dir.display());

    // Create directories
    fs::create_dir_all(&tool_dir)?;

    // Get current executable path
    let self_path = get_self_path()?;
    let target_exe = tool_dir.join("steam-command-runner");

    // Create symlink to the executable
    if target_exe.exists() {
        fs::remove_file(&target_exe)?;
    }

    debug!(
        "Creating symlink: {} -> {}",
        target_exe.display(),
        self_path.display()
    );
    symlink(&self_path, &target_exe)?;

    // Generate and write compatibilitytool.vdf
    let display_name = format!("Steam Command Runner ({})", name);
    let compat_vdf = generate_compatibilitytool_vdf(name, &display_name);
    let compat_vdf_path = tool_dir.join("compatibilitytool.vdf");
    debug!("Writing: {}", compat_vdf_path.display());
    fs::write(&compat_vdf_path, compat_vdf)?;

    // Get Proton App ID if specified
    let proton_appid = require_proton.as_ref().and_then(|p| proton_name_to_appid(p));

    // Generate and write toolmanifest.vdf
    let manifest_vdf = generate_toolmanifest_vdf(proton_appid.as_deref());
    let manifest_path = tool_dir.join("toolmanifest.vdf");
    debug!("Writing: {}", manifest_path.display());
    fs::write(&manifest_path, manifest_vdf)?;

    info!("Installation complete");
    Ok(())
}

/// Uninstall the Steam compatibility tool
pub fn uninstall_compat_tool(steam_path: Option<PathBuf>) -> Result<(), AppError> {
    let compat_dir = get_compat_tools_dir(steam_path)?;
    let tool_dir = compat_dir.join("steam-command-runner");

    if !tool_dir.exists() {
        info!("Tool not installed at: {}", tool_dir.display());
        return Ok(());
    }

    info!("Removing: {}", tool_dir.display());
    fs::remove_dir_all(&tool_dir)?;

    Ok(())
}

/// Convert Proton name to Steam App ID
fn proton_name_to_appid(name: &str) -> Option<String> {
    let name_lower = name.to_lowercase();

    // Common Proton versions and their App IDs
    let mappings: &[(&str, Option<&str>)] = &[
        ("proton 9", Some("2348590")),
        ("proton 8", Some("2348590")),
        ("proton 7", Some("1887720")),
        ("proton experimental", Some("1493710")),
        ("proton hotfix", Some("2180100")),
        ("proton-ge", None), // GE doesn't have an App ID
    ];

    for (pattern, appid) in mappings {
        if name_lower.contains(pattern) {
            return appid.map(|s| s.to_string());
        }
    }

    // If it looks like an App ID already, return it
    if name.chars().all(|c| c.is_ascii_digit()) && !name.is_empty() {
        return Some(name.to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proton_name_to_appid() {
        assert_eq!(
            proton_name_to_appid("Proton 9.0"),
            Some("2348590".to_string())
        );
        assert_eq!(
            proton_name_to_appid("Proton Experimental"),
            Some("1493710".to_string())
        );
        assert_eq!(
            proton_name_to_appid("1493710"),
            Some("1493710".to_string())
        );
        assert_eq!(proton_name_to_appid("Proton-GE"), None);
    }
}
