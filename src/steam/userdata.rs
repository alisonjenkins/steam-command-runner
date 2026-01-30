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

/// Get the path to loginusers.vdf
pub fn get_login_users_path() -> Result<PathBuf, AppError> {
    let steam_root = get_steam_root().ok_or_else(|| {
        AppError::SteamUserNotFound("Could not find Steam installation".to_string())
    })?;

    let config_path = steam_root.join("config").join("loginusers.vdf");

    if !config_path.exists() {
        return Err(AppError::SteamUserNotFound(format!(
            "loginusers.vdf not found: {}",
            config_path.display()
        )));
    }

    Ok(config_path)
}

/// Get a map of Account ID (32-bit) to Persona Name
pub fn get_user_names() -> Result<std::collections::HashMap<u64, String>, AppError> {
    let path = get_login_users_path()?;
    let content = fs::read_to_string(&path)?;
    
    let mut names = std::collections::HashMap::new();
    let mut current_steam_id64 = String::new();
    
    for line in content.lines() {
        let trimmed = line.trim();
        
        // Very basic VDF parsing sufficient for this file structure
        // We look for quoted keys that look like steam IDs, and "PersonaName" keys
        
        if trimmed.starts_with('"') {
            let parts: Vec<&str> = trimmed.split('"').filter(|s| !s.trim().is_empty()).collect();
            
            if parts.len() == 1 {
                // Potential SteamID key (section start)
                let key = parts[0];
                if key.len() > 10 && key.chars().all(|c| c.is_numeric()) {
                    current_steam_id64 = key.to_string();
                }
            } else if parts.len() >= 2 {
                // Key-Value pair
                let key = parts[0];
                let value = parts[1];
                
                if key == "PersonaName" && !current_steam_id64.is_empty() {
                    if let Ok(steam_id64) = current_steam_id64.parse::<u64>() {
                        // Convert to 32-bit Account ID
                        // SteamID64 = AccountID * 2 + 76561197960265728 + Y
                        // But usually simpler conversion is just modifying the high bits or subtracting base
                        // The standard base is 76561197960265728
                        if steam_id64 > 76561197960265728 {
                            let account_id = steam_id64 - 76561197960265728;
                            debug!("Found user: {} -> {}", account_id, value);
                            names.insert(account_id, value.to_string());
                        }
                    }
                }
            }
        }
    }
    
    Ok(names)
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
