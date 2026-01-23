use super::verbs::Verb;
use crate::error::AppError;
use std::path::PathBuf;
use tracing::debug;

/// Context from Steam environment when running as a compatibility tool
#[derive(Debug)]
pub struct CompatToolContext {
    /// The verb passed by Steam
    pub verb: Verb,

    /// Path to the game executable
    pub game_path: PathBuf,

    /// Arguments to pass to the game
    pub game_args: Vec<String>,

    /// Steam App ID (if available)
    pub steam_app_id: Option<u32>,

    /// Path to compatibility data directory
    pub compat_data_path: Option<PathBuf>,

    /// Path to Steam client installation
    pub client_install_path: Option<PathBuf>,

    /// Steam Game ID (may differ from App ID for shortcuts)
    pub steam_game_id: Option<String>,
}

impl CompatToolContext {
    /// Create context from environment variables and command-line arguments
    pub fn from_env_and_args(verb: &str, args: Vec<String>) -> Result<Self, AppError> {
        let verb = Verb::from_str(verb)?;

        // Extract game path and args from the remaining arguments
        let (game_path, game_args) = if args.is_empty() {
            (PathBuf::new(), Vec::new())
        } else {
            let path = PathBuf::from(&args[0]);
            let args = args.into_iter().skip(1).collect();
            (path, args)
        };

        // Read Steam environment variables
        let steam_app_id = std::env::var("SteamAppId")
            .ok()
            .and_then(|s| s.parse().ok());

        let compat_data_path = std::env::var("STEAM_COMPAT_DATA_PATH")
            .ok()
            .map(PathBuf::from);

        let client_install_path = std::env::var("STEAM_COMPAT_CLIENT_INSTALL_PATH")
            .ok()
            .map(PathBuf::from);

        let steam_game_id = std::env::var("SteamGameId").ok();

        debug!("Steam environment:");
        debug!("  SteamAppId: {:?}", steam_app_id);
        debug!("  SteamGameId: {:?}", steam_game_id);
        debug!("  STEAM_COMPAT_DATA_PATH: {:?}", compat_data_path);
        debug!("  STEAM_COMPAT_CLIENT_INSTALL_PATH: {:?}", client_install_path);

        Ok(Self {
            verb,
            game_path,
            game_args,
            steam_app_id,
            compat_data_path,
            client_install_path,
            steam_game_id,
        })
    }
}
