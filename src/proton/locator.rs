use crate::error::AppError;
use std::path::PathBuf;
use tracing::{debug, info};

/// Locate a Proton installation
///
/// Search order:
/// 1. If a specific version is requested, search for it by name
/// 2. Search in Steam's compatibilitytools.d (custom Proton)
/// 3. Search in Steam's common directory (official Proton)
/// 4. Use STEAM_COMPAT_TOOL_PATH if set
pub fn locate_proton(requested_version: Option<&str>) -> Result<PathBuf, AppError> {
    let search_paths = get_search_paths();
    debug!("Searching for Proton in: {:?}", search_paths);

    // If a specific version is requested, try to find it
    if let Some(version) = requested_version {
        info!("Looking for Proton version: {}", version);

        for base_path in &search_paths {
            // Try exact match first
            let exact_path = base_path.join(version);
            if is_valid_proton(&exact_path) {
                return Ok(exact_path);
            }

            // Try case-insensitive search
            if let Ok(entries) = std::fs::read_dir(base_path) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.to_lowercase().contains(&version.to_lowercase()) {
                        let path = entry.path();
                        if is_valid_proton(&path) {
                            return Ok(path);
                        }
                    }
                }
            }
        }

        return Err(AppError::ProtonNotFound(version.to_string()));
    }

    // No specific version requested - try to find any Proton
    // Check STEAM_COMPAT_TOOL_PATH first
    if let Ok(tool_path) = std::env::var("STEAM_COMPAT_TOOL_PATH") {
        let path = PathBuf::from(tool_path);
        if is_valid_proton(&path) {
            return Ok(path);
        }
    }

    // Search for any Proton in the search paths
    for base_path in &search_paths {
        if let Ok(entries) = std::fs::read_dir(base_path) {
            let mut proton_versions: Vec<_> = entries
                .flatten()
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().to_lowercase();
                    name.contains("proton") && is_valid_proton(&e.path())
                })
                .collect();

            // Sort by name descending to prefer newer versions
            proton_versions.sort_by(|a, b| {
                b.file_name().cmp(&a.file_name())
            });

            if let Some(entry) = proton_versions.first() {
                return Ok(entry.path());
            }
        }
    }

    Err(AppError::ProtonNotFound("any".to_string()))
}

/// Get list of paths to search for Proton
fn get_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // ~/.steam/root/compatibilitytools.d (custom Proton like GE)
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".steam/root/compatibilitytools.d"));
        paths.push(home.join(".local/share/Steam/compatibilitytools.d"));
    }

    // Steam library paths - check common locations
    let steam_paths = get_steam_library_paths();
    for steam_path in steam_paths {
        paths.push(steam_path.join("steamapps/common"));
    }

    paths
}

/// Get Steam library paths from libraryfolders.vdf
fn get_steam_library_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Default Steam locations
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".steam/steam"));
        paths.push(home.join(".local/share/Steam"));
    }

    // Try to read libraryfolders.vdf for additional library paths
    for base in &paths.clone() {
        let vdf_path = base.join("steamapps/libraryfolders.vdf");
        if vdf_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&vdf_path) {
                // Simple parsing for "path" entries
                for line in content.lines() {
                    if line.contains("\"path\"") {
                        if let Some(start) = line.rfind('"') {
                            let before = &line[..start];
                            if let Some(path_start) = before.rfind('"') {
                                let path = &before[path_start + 1..];
                                let lib_path = PathBuf::from(path);
                                if lib_path.exists() && !paths.contains(&lib_path) {
                                    paths.push(lib_path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    paths
}

/// Check if a path contains a valid Proton installation
fn is_valid_proton(path: &PathBuf) -> bool {
    path.is_dir() && path.join("proton").exists()
}

/// List available Proton versions
pub fn list_proton_versions() -> Vec<(String, PathBuf)> {
    let mut versions = Vec::new();
    let search_paths = get_search_paths();

    for base_path in search_paths {
        if let Ok(entries) = std::fs::read_dir(&base_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if is_valid_proton(&path) {
                    let name = entry.file_name().to_string_lossy().to_string();
                    versions.push((name, path));
                }
            }
        }
    }

    versions.sort_by(|a, b| a.0.cmp(&b.0));
    versions
}
