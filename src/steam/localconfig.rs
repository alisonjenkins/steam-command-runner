use crate::error::AppError;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::debug;

/// Represents the localconfig with just the apps section we need
pub struct LocalConfig {
    /// The raw file content
    content: String,
    /// Parsed launch options by app ID
    launch_options: HashMap<u32, String>,
}

impl LocalConfig {
    /// Parse a localconfig.vdf file
    fn parse(content: &str) -> Self {
        let mut launch_options = HashMap::new();

        // Find the apps section and parse launch options
        // VDF format: "apps" { "12345" { "LaunchOptions" "options here" } }
        let mut current_app_id: Option<u32> = None;
        let mut in_apps_section = false;
        let mut brace_depth = 0;
        let mut apps_brace_depth = 0;

        for line in content.lines() {
            let trimmed = line.trim();

            // Track brace depth
            if trimmed == "{" {
                brace_depth += 1;
            } else if trimmed == "}" {
                brace_depth -= 1;
                if in_apps_section && brace_depth < apps_brace_depth {
                    in_apps_section = false;
                }
                if in_apps_section && current_app_id.is_some() && brace_depth == apps_brace_depth {
                    current_app_id = None;
                }
            }

            // Check for apps section
            if trimmed.starts_with("\"apps\"") {
                in_apps_section = true;
                apps_brace_depth = brace_depth + 1;
                continue;
            }

            if !in_apps_section {
                continue;
            }

            // Parse app ID entries (e.g., "1850570")
            if current_app_id.is_none() {
                if let Some(app_id) = parse_quoted_key(trimmed) {
                    if let Ok(id) = app_id.parse::<u32>() {
                        current_app_id = Some(id);
                    }
                }
                continue;
            }

            // Look for LaunchOptions within an app
            if let Some((key, value)) = parse_key_value(trimmed) {
                if key.eq_ignore_ascii_case("LaunchOptions") {
                    if let Some(app_id) = current_app_id {
                        debug!("Found launch options for app {}: {}", app_id, value);
                        launch_options.insert(app_id, value);
                    }
                }
            }
        }

        LocalConfig {
            content: content.to_string(),
            launch_options,
        }
    }

    /// Get launch options for a specific app
    pub fn get_launch_options(&self, app_id: u32) -> Option<&String> {
        self.launch_options.get(&app_id)
    }

    /// Set launch options for a specific app
    pub fn set_launch_options(&mut self, app_id: u32, options: Option<&str>) {
        match options {
            Some(opts) => {
                self.launch_options.insert(app_id, opts.to_string());
            }
            None => {
                self.launch_options.remove(&app_id);
            }
        }

        // Regenerate content
        self.regenerate_content(app_id, options);
    }

    /// Regenerate the content with the updated launch options
    fn regenerate_content(&mut self, app_id: u32, options: Option<&str>) {
        let app_id_str = app_id.to_string();
        let mut new_lines = Vec::new();
        let mut in_apps_section = false;
        let mut in_target_app = false;
        let mut brace_depth = 0;
        let mut apps_brace_depth = 0;
        let mut app_brace_depth = 0;
        let mut added_launch_options = false;

        let lines: Vec<&str> = self.content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            // Track brace depth
            if trimmed == "{" {
                brace_depth += 1;
            } else if trimmed == "}" {
                // Before closing the app section, add launch options if needed
                if in_target_app
                    && brace_depth == app_brace_depth
                    && !added_launch_options
                    && options.is_some()
                {
                    let indent = get_indent(line);
                    new_lines.push(format!(
                        "{}\t\"LaunchOptions\"\t\t\"{}\"",
                        indent,
                        escape_vdf_string(options.unwrap())
                    ));
                    added_launch_options = true;
                }

                brace_depth -= 1;
                if in_apps_section && brace_depth < apps_brace_depth {
                    in_apps_section = false;
                }
                if in_target_app && brace_depth < app_brace_depth {
                    in_target_app = false;
                }
            }

            // Check for apps section
            if trimmed.starts_with("\"apps\"") {
                in_apps_section = true;
                apps_brace_depth = brace_depth + 1;
            }

            // Check for target app
            if in_apps_section
                && !in_target_app
                && parse_quoted_key(trimmed) == Some(&app_id_str)
            {
                in_target_app = true;
                app_brace_depth = brace_depth + 1;
                added_launch_options = false;
            }

            // Handle LaunchOptions within target app
            if in_target_app {
                if let Some((key, _)) = parse_key_value(trimmed) {
                    if key.eq_ignore_ascii_case("LaunchOptions") {
                        if let Some(opts) = options {
                            // Replace with new value
                            let indent = get_line_indent(line);
                            new_lines.push(format!(
                                "{}\"LaunchOptions\"\t\t\"{}\"",
                                indent,
                                escape_vdf_string(opts)
                            ));
                            added_launch_options = true;
                        }
                        // If options is None, skip this line (remove it)
                        i += 1;
                        continue;
                    }
                }
            }

            new_lines.push(line.to_string());
            i += 1;
        }

        // If we didn't find the app at all, we need to add it
        if !in_target_app && options.is_some() {
            // Find the apps section and add the new entry
            self.content = add_app_entry(&new_lines.join("\n"), app_id, options.unwrap());
        } else {
            self.content = new_lines.join("\n");
        }
    }

    /// Get the raw content
    pub fn content(&self) -> &str {
        &self.content
    }
}

/// Add a new app entry with launch options
fn add_app_entry(content: &str, app_id: u32, options: &str) -> String {
    let mut new_lines = Vec::new();
    let mut in_apps_section = false;
    let mut brace_depth = 0;
    let mut apps_brace_depth = 0;
    let mut added = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "{" {
            brace_depth += 1;
            new_lines.push(line.to_string());

            // Add entry right after opening brace of apps section
            if in_apps_section && brace_depth == apps_brace_depth && !added {
                new_lines.push(format!("\t\t\t\t\t\"{}\"", app_id));
                new_lines.push("\t\t\t\t\t{".to_string());
                new_lines.push(format!(
                    "\t\t\t\t\t\t\"LaunchOptions\"\t\t\"{}\"",
                    escape_vdf_string(options)
                ));
                new_lines.push("\t\t\t\t\t}".to_string());
                added = true;
            }
            continue;
        } else if trimmed == "}" {
            brace_depth -= 1;
            if in_apps_section && brace_depth < apps_brace_depth {
                in_apps_section = false;
            }
        }

        if trimmed.starts_with("\"apps\"") {
            in_apps_section = true;
            apps_brace_depth = brace_depth + 1;
        }

        new_lines.push(line.to_string());
    }

    new_lines.join("\n")
}

/// Parse a quoted key from a line (e.g., '"key"' returns "key")
/// Only matches standalone keys (e.g., section names, app IDs), not key-value pairs
fn parse_quoted_key(line: &str) -> Option<&str> {
    let line = line.trim();
    if !line.starts_with('"') {
        return None;
    }

    let rest = &line[1..];
    let end = rest.find('"')?;

    // Make sure there's nothing significant after (just optional whitespace or opening brace)
    // This excludes key-value pairs like "key" "value"
    let after = rest[end + 1..].trim();
    if after.is_empty() || after == "{" {
        Some(&rest[..end])
    } else {
        None
    }
}

/// Parse a key-value pair (e.g., '"key" "value"' returns ("key", "value"))
fn parse_key_value(line: &str) -> Option<(&str, String)> {
    let line = line.trim();
    if !line.starts_with('"') {
        return None;
    }

    let rest = &line[1..];
    let key_end = rest.find('"')?;
    let key = &rest[..key_end];

    let after_key = rest[key_end + 1..].trim();
    if !after_key.starts_with('"') {
        return None;
    }

    let value_rest = &after_key[1..];
    // Handle escaped quotes in value
    let mut value = String::new();
    let mut chars = value_rest.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                if next == '"' || next == '\\' {
                    value.push(next);
                    chars.next();
                    continue;
                }
            }
            value.push(c);
        } else if c == '"' {
            break;
        } else {
            value.push(c);
        }
    }

    Some((key, value))
}

/// Get the indentation of a line
fn get_line_indent(line: &str) -> &str {
    let trimmed_len = line.trim_start().len();
    &line[..line.len() - trimmed_len]
}

/// Get a reasonable indent based on a closing brace line
fn get_indent(line: &str) -> &str {
    get_line_indent(line)
}

/// Escape a string for VDF format
fn escape_vdf_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Read and parse localconfig.vdf
pub fn read_localconfig<P: AsRef<Path>>(path: P) -> Result<LocalConfig, AppError> {
    let content = fs::read_to_string(path.as_ref())?;
    debug!("Read localconfig.vdf ({} bytes)", content.len());
    Ok(LocalConfig::parse(&content))
}

/// Write localconfig.vdf back to disk
pub fn write_localconfig<P: AsRef<Path>>(path: P, config: &LocalConfig) -> Result<(), AppError> {
    debug!("Writing localconfig.vdf ({} bytes)", config.content.len());
    fs::write(path.as_ref(), &config.content)?;
    Ok(())
}

/// Set launch options (convenience function)
pub fn set_launch_options(config: &mut LocalConfig, app_id: u32, options: Option<&str>) {
    config.set_launch_options(app_id, options);
}

/// Get launch options (convenience function)
pub fn get_launch_options(config: &LocalConfig, app_id: u32) -> Option<String> {
    config.get_launch_options(app_id).cloned()
}

/// Check if launch options look like they were set by steam-command-runner
/// We detect our format: "gamescope -- %command%" or variants with our shim
pub fn is_our_launch_options(options: &str) -> bool {
    // Check for our simple format
    let trimmed = options.trim();
    if trimmed == "gamescope -- %command%" {
        return true;
    }
    // Also match if it starts with gamescope and ends with %command%
    // and contains steam-command-runner (older format)
    if trimmed.starts_with("gamescope") && trimmed.contains("steam-command-runner") {
        return true;
    }
    false
}

/// Generate the default launch options string
///
/// The gamescope shim reads config to get gamescope args automatically,
/// so we don't need $(steam-command-runner gamescope args).
///
/// If using steam-command-runner as the compatibility tool, %command%
/// already has pre_command, env vars, etc. applied, so we don't need
/// steam-command-runner run either.
pub fn generate_default_launch_options() -> String {
    "gamescope -- %command%".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_default_launch_options() {
        let options = generate_default_launch_options();
        assert_eq!(options, "gamescope -- %command%");
    }

    #[test]
    fn test_is_our_launch_options() {
        // New simple format
        assert!(is_our_launch_options("gamescope -- %command%"));
        // Old format with steam-command-runner
        assert!(is_our_launch_options(
            "gamescope $(steam-command-runner gamescope args) -- steam-command-runner run -- %command%"
        ));
        // Not ours
        assert!(!is_our_launch_options("mangohud %command%"));
        assert!(!is_our_launch_options("gamemoderun %command%"));
    }

    #[test]
    fn test_parse_quoted_key() {
        assert_eq!(parse_quoted_key("\"1850570\""), Some("1850570"));
        assert_eq!(parse_quoted_key("\"apps\""), Some("apps"));
        assert_eq!(parse_quoted_key("\"key\" \"value\""), None);
        assert_eq!(parse_quoted_key("not quoted"), None);
    }

    #[test]
    fn test_parse_key_value() {
        let result = parse_key_value("\"LaunchOptions\"\t\t\"test options\"");
        assert_eq!(result, Some(("LaunchOptions", "test options".to_string())));

        let result = parse_key_value("\"key\"\t\"value with \\\"quotes\\\"\"");
        assert_eq!(result, Some(("key", "value with \"quotes\"".to_string())));
    }

    #[test]
    fn test_escape_vdf_string() {
        assert_eq!(escape_vdf_string("test"), "test");
        assert_eq!(escape_vdf_string("test\"quote"), "test\\\"quote");
        assert_eq!(escape_vdf_string("test\\slash"), "test\\\\slash");
    }
}
