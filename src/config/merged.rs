use super::error::ConfigError;
use super::game::GameConfig;
use super::global::{ExecutionMode, GlobalConfig, HookConfig};
use super::{get_config_path, get_game_config_path};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::debug;

/// Merged configuration for a specific game launch
#[derive(Debug, Clone)]
pub struct MergedConfig {
    /// Steam App ID if known
    pub app_id: Option<u32>,

    /// Game name if configured
    pub name: Option<String>,

    /// Execution mode (native, proton, auto)
    pub mode: ExecutionMode,

    /// Proton version to use
    pub proton: Option<String>,

    /// Pre-command to prepend
    pub pre_command: Option<String>,

    /// Environment variables to set
    pub env: HashMap<String, String>,

    /// Additional launch arguments
    pub launch_args: Vec<String>,

    /// Pre-launch hook
    pub pre_launch_hook: Option<HookConfig>,

    /// Post-exit hook
    pub post_exit_hook: Option<HookConfig>,

    /// Whether we're in a Gamescope session
    pub is_gamescope_session: bool,

    /// Gamescope-specific pre_command
    pub gamescope_pre_command: Option<String>,

    /// Skip pre_command in Gamescope
    pub skip_pre_command_in_gamescope: bool,
}

impl MergedConfig {
    /// Load and merge configuration for a game
    pub fn load(app_id: Option<u32>, config_path: Option<PathBuf>) -> Result<Self, ConfigError> {
        let is_gamescope = is_gamescope_session();
        debug!("Gamescope session: {}", is_gamescope);

        // Load global config
        let global_path = config_path.unwrap_or_else(get_config_path);
        let global = if global_path.exists() {
            debug!("Loading global config from: {}", global_path.display());
            let content = fs::read_to_string(&global_path)?;
            toml::from_str(&content)?
        } else {
            debug!("No global config found, using defaults");
            GlobalConfig::default()
        };

        // Load game-specific config if app_id is provided
        let game = if let Some(id) = app_id {
            let game_path = get_game_config_path(id);
            if game_path.exists() {
                debug!("Loading game config from: {}", game_path.display());
                let content = fs::read_to_string(&game_path)?;
                Some(toml::from_str(&content)?)
            } else {
                debug!("No game config found for app_id: {}", id);
                None
            }
        } else {
            None
        };

        Ok(Self::merge(global, game, is_gamescope, app_id))
    }

    /// Merge global and game configurations
    fn merge(
        global: GlobalConfig,
        game: Option<GameConfig>,
        is_gamescope: bool,
        app_id: Option<u32>,
    ) -> Self {
        let game = game.unwrap_or_default();

        // Merge environment variables (game overrides global)
        let mut env = global.env.clone();
        env.extend(game.env);

        // Handle pre_command with "inherit" keyword
        let pre_command = match &game.pre_command {
            Some(cmd) if cmd.contains("inherit") => {
                // Replace "inherit" with global pre_command
                let global_pre = global.pre_command.as_deref().unwrap_or("");
                Some(cmd.replace("inherit", global_pre).trim().to_string())
            }
            Some(cmd) => Some(cmd.clone()),
            None => global.pre_command.clone(),
        };

        // Merge hooks (game overrides global)
        let pre_launch_hook = game
            .hooks
            .pre_launch
            .or(global.hooks.pre_launch);
        let post_exit_hook = game
            .hooks
            .post_exit
            .or(global.hooks.post_exit);

        Self {
            app_id,
            name: game.name,
            mode: game.mode.unwrap_or(global.default_mode),
            proton: game.proton.or(global.default_proton),
            pre_command,
            env,
            launch_args: game.launch_args,
            pre_launch_hook,
            post_exit_hook,
            is_gamescope_session: is_gamescope,
            gamescope_pre_command: global.gamescope.pre_command,
            skip_pre_command_in_gamescope: global.gamescope.skip_pre_command,
        }
    }

    /// Get the effective pre_command considering Gamescope session
    pub fn effective_pre_command(&self) -> Option<&str> {
        if self.is_gamescope_session {
            if self.skip_pre_command_in_gamescope {
                self.gamescope_pre_command.as_deref()
            } else {
                self.pre_command.as_deref()
            }
        } else {
            self.pre_command.as_deref()
        }
    }
}

/// Check if running in a Gamescope session
fn is_gamescope_session() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|v| v.to_lowercase() == "gamescope")
        .unwrap_or(false)
}
