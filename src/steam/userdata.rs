use crate::error::AppError;
use std::fs;
use std::path::PathBuf;
use tracing::debug;

/// Get the Steam root directory
pub fn get_steam_root() -> Option<PathBuf> {
    let candidates = [
        dirs::home_dir().map(|h| h.join(".steam/steam")),
        dirs::home_dir().map(|h| h.join(".local/share/Steam")),
        dirs::data_dir().map(|d| d.join("Steam")),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            debug!("Found Steam root at: {}", candidate.display());
            return Some(candidate);
        }
    }

    None
}

/// Find all Steam user IDs in userdata directory
pub fn find_user_ids() -> Result<Vec<u64>, AppError> {
    let steam_root = get_steam_root().ok_or_else(|| {
        AppError::SteamUserNotFound("Could not find Steam installation".to_string())
    })?;

    let userdata_dir = steam_root.join("userdata");
    if !userdata_dir.exists() {
        return Err(AppError::SteamUserNotFound(format!(
            "Userdata directory not found: {}",
            userdata_dir.display()
        )));
    }

    let mut user_ids = Vec::new();

    for entry in fs::read_dir(&userdata_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = path.file_name() {
                if let Some(name_str) = name.to_str() {
                    if let Ok(user_id) = name_str.parse::<u64>() {
                        // Skip special directories like "0" or "anonymous"
                        if user_id > 0 {
                            debug!("Found user ID: {}", user_id);
                            user_ids.push(user_id);
                        }
                    }
                }
            }
        }
    }

    if user_ids.is_empty() {
        return Err(AppError::SteamUserNotFound(
            "No Steam users found in userdata directory".to_string(),
        ));
    }

    Ok(user_ids)
}

/// Get the path to a user's localconfig.vdf
pub fn get_localconfig_path(user_id: u64) -> Result<PathBuf, AppError> {
    let steam_root = get_steam_root().ok_or_else(|| {
        AppError::SteamUserNotFound("Could not find Steam installation".to_string())
    })?;

    let config_path = steam_root
        .join("userdata")
        .join(user_id.to_string())
        .join("config")
        .join("localconfig.vdf");

    if !config_path.exists() {
        return Err(AppError::SteamUserNotFound(format!(
            "localconfig.vdf not found for user {}: {}",
            user_id,
            config_path.display()
        )));
    }

    Ok(config_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_steam_root() {
        // This test just checks that the function doesn't panic
        let _result = get_steam_root();
    }
}
