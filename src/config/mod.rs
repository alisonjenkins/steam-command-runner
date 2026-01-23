mod error;
mod game;
mod global;
mod merged;

pub use error::ConfigError;
pub use game::GameConfig;
pub use global::{ExecutionMode, GlobalConfig, GamescopeConfig, HookConfig, HooksConfig};
pub use merged::MergedConfig;

use std::path::PathBuf;

/// Get the global config file path
pub fn get_config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(".config"));
    config_dir.join("steam-command-runner").join("config.toml")
}

/// Get the game-specific config file path
pub fn get_game_config_path(app_id: u32) -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(".config"));
    config_dir
        .join("steam-command-runner")
        .join("games")
        .join(format!("{}.toml", app_id))
}

/// Get the games config directory
pub fn get_games_config_dir() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(".config"));
    config_dir.join("steam-command-runner").join("games")
}
