use super::global::{ExecutionMode, HooksConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Per-game configuration, overrides global settings
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    /// Display name for logging/debugging
    #[serde(default)]
    pub name: Option<String>,

    /// Override execution mode for this game
    #[serde(default)]
    pub mode: Option<ExecutionMode>,

    /// Specific Proton version (overrides global default)
    #[serde(default)]
    pub proton: Option<String>,

    /// Override/extend pre_command
    /// Use "inherit" to include global pre_command, or specify full command
    #[serde(default)]
    pub pre_command: Option<String>,

    /// Per-game environment variables (merged with global, game takes precedence)
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Game-specific launch arguments
    #[serde(default)]
    pub launch_args: Vec<String>,

    /// Game-specific hooks (override global hooks)
    #[serde(default)]
    pub hooks: HooksConfig,
}
