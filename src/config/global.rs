use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Global configuration for steam-command-runner
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Pre-command to prepend (e.g., gamemoderun, mangohud)
    #[serde(default)]
    pub pre_command: Option<String>,

    /// Default Proton version (name as shown in Steam, or path)
    #[serde(default)]
    pub default_proton: Option<String>,

    /// Default execution mode
    #[serde(default)]
    pub default_mode: ExecutionMode,

    /// Global environment variables applied to all games
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Hook configuration
    #[serde(default)]
    pub hooks: HooksConfig,

    /// Gamescope-specific settings
    #[serde(default)]
    pub gamescope: GamescopeConfig,
}

/// Execution mode for games
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Run as native Linux game
    Native,
    /// Always use Proton/Wine
    Proton,
    /// Auto-detect based on executable type
    #[default]
    Auto,
}

/// Hook configuration for pre-launch and post-exit commands
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct HooksConfig {
    /// Pre-launch hook (runs before game starts)
    #[serde(default)]
    pub pre_launch: Option<HookConfig>,

    /// Post-exit hook (runs after game exits)
    #[serde(default)]
    pub post_exit: Option<HookConfig>,
}

/// Individual hook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    /// Command to execute
    pub command: String,

    /// Wait for completion before continuing
    #[serde(default = "default_wait")]
    pub wait: bool,

    /// Working directory for the hook
    #[serde(default)]
    pub working_dir: Option<String>,
}

fn default_wait() -> bool {
    true
}

/// Gamescope-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamescopeConfig {
    /// Skip pre_command when in Gamescope session (default: true)
    #[serde(default = "default_skip_pre_command")]
    pub skip_pre_command: bool,

    /// Additional pre_command for Gamescope only
    #[serde(default)]
    pub pre_command: Option<String>,
}

impl Default for GamescopeConfig {
    fn default() -> Self {
        Self {
            skip_pre_command: true,
            pre_command: None,
        }
    }
}

fn default_skip_pre_command() -> bool {
    true
}
