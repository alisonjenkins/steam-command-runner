use crate::config::MergedConfig;
use crate::error::AppError;
use crate::runner::execute_game;
use std::path::PathBuf;
use tracing::{debug, info};

/// Handle the run command - execute a game with configured wrappers
pub fn handle_run(
    app_id: Option<u32>,
    command: Vec<String>,
    config_path: Option<PathBuf>,
) -> Result<(), AppError> {
    if command.is_empty() {
        return Err(AppError::NoCommand);
    }

    info!("Running command with app_id: {:?}", app_id);
    debug!("Command: {:?}", command);

    // Load and merge configuration
    let config = MergedConfig::load(app_id, config_path)?;
    debug!("Loaded config: {:?}", config);

    // Execute the game
    execute_game(&config, command)?;

    Ok(())
}
