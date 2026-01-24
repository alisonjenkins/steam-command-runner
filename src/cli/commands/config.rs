use crate::cli::ConfigAction;
use crate::config::{get_config_path, get_game_config_path};
use crate::error::AppError;
use std::fs;
use tracing::info;

/// Handle the config command and its subcommands
pub fn handle_config(action: ConfigAction) -> Result<(), AppError> {
    match action {
        ConfigAction::Show { app_id } => show_config(app_id),
        ConfigAction::Init => init_config(),
        ConfigAction::Edit { app_id } => edit_config(app_id),
        ConfigAction::Path { app_id } => show_path(app_id),
    }
}

fn show_config(app_id: Option<u32>) -> Result<(), AppError> {
    let path = match app_id {
        Some(id) => get_game_config_path(id),
        None => get_config_path(),
    };

    if path.exists() {
        let content = fs::read_to_string(&path)?;
        println!("# {}\n", path.display());
        println!("{}", content);
    } else {
        println!("Config file does not exist: {}", path.display());
        println!("\nRun 'steam-command-runner config init' to create default config.");
    }

    Ok(())
}

fn init_config() -> Result<(), AppError> {
    let path = get_config_path();

    if path.exists() {
        println!("Config file already exists: {}", path.display());
        return Ok(());
    }

    // Create parent directory
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write default config with comments
    let template = r#"# Steam Command Runner - Global Configuration

# Pre-command to prepend to game launches (e.g., gamemoderun, mangohud)
# pre_command = "gamemoderun"

# Default Proton version (name as shown in Steam, or path)
# default_proton = "Proton 9.0"

# Default execution mode: native | proton | auto
default_mode = "auto"

# Global environment variables applied to all games
[env]
# MANGOHUD = "1"
# DXVK_ASYNC = "1"

# Gamescope-specific settings
[gamescope]
# Enable gamescope wrapper (default: true)
# enabled = true
# Skip pre_command when in Gamescope session
skip_pre_command = true
# Additional pre_command for Gamescope only
# pre_command = ""
# Arguments to pass to gamescope (e.g., "-w 1920 -h 1080 -f")
# args = ""

# Pre-launch hook (runs before game starts)
[hooks]
# [hooks.pre_launch]
# command = "/path/to/script.sh"
# wait = true

# [hooks.post_exit]
# command = "/path/to/cleanup.sh"
# wait = false
"#;
    fs::write(&path, template)?;

    info!("Created default config at: {}", path.display());
    println!("Created default config at: {}", path.display());

    Ok(())
}

fn edit_config(app_id: Option<u32>) -> Result<(), AppError> {
    let path = match app_id {
        Some(id) => get_game_config_path(id),
        None => get_config_path(),
    };

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // If file doesn't exist for game config, create a template
    if !path.exists() {
        if let Some(id) = app_id {
            let template = format!(
                r#"# Per-game configuration for Steam App ID {}

# Display name (for logging)
# name = "Game Name"

# Execution mode: native | proton | auto
# mode = "proton"

# Specific Proton version (overrides global)
# proton = "Proton 9.0"

# Pre-command (use "inherit" to include global pre_command)
# pre_command = "inherit mangohud"

# Game-specific gamescope arguments (overrides global)
# gamescope_args = "-w 1920 -h 1080 -f"

# Disable gamescope for this game (e.g., for Steam Input compatibility)
# gamescope_enabled = false

# Game-specific environment variables
[env]
# MANGOHUD = "1"

# Game-specific hooks
# [hooks.pre_launch]
# command = "/path/to/script.sh"
# wait = true
"#,
                id
            );
            fs::write(&path, template)?;
        } else {
            init_config()?;
        }
    }

    // Open in editor
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()?;

    if !status.success() {
        return Err(AppError::EditorFailed(editor));
    }

    Ok(())
}

fn show_path(app_id: Option<u32>) -> Result<(), AppError> {
    let path = match app_id {
        Some(id) => get_game_config_path(id),
        None => get_config_path(),
    };

    println!("{}", path.display());

    Ok(())
}
