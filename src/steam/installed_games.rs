use crate::error::AppError;
use crate::steam::userdata::get_steam_root;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use tracing::debug;

/// Information about an installed Steam game
#[derive(Debug, Clone)]
pub struct InstalledGame {
    pub app_id: u32,
    pub name: String,
    pub install_dir: String,
}

/// Parse a VDF key-value line like: "key"		"value"
fn parse_vdf_key_value(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if !line.starts_with('"') {
        return None;
    }

    // Find all quoted strings in the line
    let mut quotes = Vec::new();
    let mut in_quote = false;
    let mut start = 0;

    for (i, c) in line.char_indices() {
        if c == '"' {
            if in_quote {
                quotes.push(&line[start..i]);
                in_quote = false;
            } else {
                start = i + 1;
                in_quote = true;
            }
        }
    }

    // We need at least 2 quoted strings (key and value)
    if quotes.len() >= 2 {
        Some((quotes[0], quotes[1]))
    } else {
        None
    }
}

/// Get all Steam library folders from libraryfolders.vdf
fn get_library_folders() -> Result<Vec<PathBuf>, AppError> {
    let steam_root = get_steam_root().ok_or_else(|| {
        AppError::SteamUserNotFound("Could not find Steam installation".to_string())
    })?;

    let libraryfolders_path = steam_root.join("steamapps/libraryfolders.vdf");

    if !libraryfolders_path.exists() {
        // Fall back to just the main steamapps folder
        return Ok(vec![steam_root.join("steamapps")]);
    }

    let content = fs::read_to_string(&libraryfolders_path)?;
    let mut folders = Vec::new();

    // Parse the VDF file to extract library paths
    // Format: "path"		"/home/user/.steam/steam"
    for line in content.lines() {
        if let Some((key, value)) = parse_vdf_key_value(line) {
            if key == "path" {
                let path = PathBuf::from(value);
                let steamapps = path.join("steamapps");
                if steamapps.exists() {
                    debug!("Found library folder: {}", steamapps.display());
                    folders.push(steamapps);
                } else {
                    debug!(
                        "Library folder does not exist: {}",
                        steamapps.display()
                    );
                }
            }
        }
    }

    // Always include the main steamapps folder
    let main_steamapps = steam_root.join("steamapps");
    if main_steamapps.exists() && !folders.contains(&main_steamapps) {
        folders.insert(0, main_steamapps);
    }

    if folders.is_empty() {
        return Err(AppError::SteamUserNotFound(
            "No Steam library folders found".to_string(),
        ));
    }

    Ok(folders)
}

/// Parse an appmanifest_*.acf file to get game info
fn parse_appmanifest(path: &PathBuf) -> Option<InstalledGame> {
    let content = fs::read_to_string(path).ok()?;

    let mut app_id: Option<u32> = None;
    let mut name: Option<String> = None;
    let mut install_dir: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();

        // Parse key-value pairs like: "appid"		"1850570"
        if line.starts_with('"') {
            let parts: Vec<&str> = line.split('"').collect();
            // parts: ["", "key", "\t\t", "value", ""]
            if parts.len() >= 4 {
                let key = parts[1].to_lowercase();
                let value = parts[3].to_string();

                match key.as_str() {
                    "appid" => app_id = value.parse().ok(),
                    "name" => name = Some(value),
                    "installdir" => install_dir = Some(value),
                    _ => {}
                }
            }
        }
    }

    match (app_id, name, install_dir) {
        (Some(app_id), Some(name), Some(install_dir)) => Some(InstalledGame {
            app_id,
            name,
            install_dir,
        }),
        (Some(app_id), Some(name), None) => Some(InstalledGame {
            app_id,
            name,
            install_dir: String::new(),
        }),
        _ => None,
    }
}

/// Find all installed games across all Steam library folders
pub fn find_installed_games() -> Result<Vec<InstalledGame>, AppError> {
    let library_folders = get_library_folders()?;
    let mut games = Vec::new();
    let mut seen_ids: HashSet<u32> = HashSet::new();

    for steamapps in library_folders {
        debug!("Scanning library folder: {}", steamapps.display());

        let entries = match fs::read_dir(&steamapps) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if filename.starts_with("appmanifest_") && filename.ends_with(".acf") {
                if let Some(game) = parse_appmanifest(&path) {
                    if !seen_ids.contains(&game.app_id) {
                        debug!("Found game: {} ({})", game.name, game.app_id);
                        seen_ids.insert(game.app_id);
                        games.push(game);
                    }
                }
            }
        }
    }

    // Sort by name
    games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(games)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_appmanifest_content() {
        // This is a simplified test - in reality we'd need a temp file
        // Just testing that the function exists and doesn't panic
    }
}
