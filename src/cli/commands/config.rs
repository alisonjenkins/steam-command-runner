use crate::cli::ConfigAction;
use crate::config::{get_config_path, get_game_config_path, GlobalConfig};
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

    // Write default config
    let default_config = GlobalConfig::default();
    let content = toml::to_string_pretty(&default_config)?;
    fs::write(&path, content)?;

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
                "# Per-game configuration for Steam App ID {}\n\
                # name = \"Game Name\"\n\
                # mode = \"proton\"  # native | proton | auto\n\
                # proton = \"Proton 9.0\"\n\
                # pre_command = \"gamemoderun\"\n\
                \n\
                [env]\n\
                # MANGOHUD = \"1\"\n",
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
