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
    ///
    /// These land on the process we exec, which is the *gamescope* process when
    /// gamescope is in play — gamescope inherits them too. Use `inner_env` for
    /// anything that must not reach the compositor.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Environment variables applied to the inner game command only
    ///
    /// Emitted as `KEY=VALUE` assignments to the `env` wrapper after gamescope's
    /// `--`, so gamescope itself never sees them. MANGOHUD is the canonical case:
    /// MangoHud's implicit Vulkan layer keys off `MANGOHUD=1`, and letting
    /// gamescope inherit it loads the overlay into gamescope's own Vulkan
    /// instance, which segfaults at exit in `CVulkanDevice::~CVulkanDevice`.
    #[serde(default)]
    pub inner_env: HashMap<String, String>,

    /// Hook configuration
    #[serde(default)]
    pub hooks: HooksConfig,

    /// Gamescope-specific settings
    #[serde(default)]
    pub gamescope: GamescopeConfig,

    /// How launches behave while a Remote Play client is streaming
    #[serde(default)]
    pub stream: StreamConfig,

    /// Arguments to append to the game command
    #[serde(default)]
    pub game_args: Option<String>,

    /// Enable debug logging for the shim (default: false)
    #[serde(default)]
    pub shim_debug: bool,
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
    /// Whether to wrap games with gamescope (default: true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Skip pre_command when in Gamescope session (default: true)
    #[serde(default = "default_skip_pre_command")]
    pub skip_pre_command: bool,

    /// Additional pre_command for Gamescope only
    #[serde(default)]
    pub pre_command: Option<String>,

    /// Arguments to pass to gamescope
    #[serde(default)]
    pub args: Option<String>,
}

impl Default for GamescopeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            skip_pre_command: true,
            pre_command: None,
            args: None,
        }
    }
}

fn default_enabled() -> bool {
    true
}

/// Launch behaviour while a Remote Play client is streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Run the game directly instead of inside gamescope (default: true)
    ///
    /// Steam can only stream in game mode, and so capture the mouse, when the
    /// game window is on its own X display. gamescope moves it to a nested one.
    #[serde(default = "default_enabled")]
    pub bypass_gamescope: bool,

    /// Which Steam overlay builds the game preloads while streaming
    #[serde(default)]
    pub overlay: OverlayPolicy,

    /// Render a streamed game at the client's resolution (default: true)
    ///
    /// A directly launched game otherwise renders at the resolution it saved
    /// last, which suits no more than one client.
    #[serde(default = "default_enabled")]
    pub set_resolution: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            bypass_gamescope: true,
            overlay: OverlayPolicy::Auto,
            set_resolution: true,
        }
    }
}

/// Which `gameoverlayrenderer.so` builds stay in `LD_PRELOAD` while streaming
///
/// Steam binds game capture to the first process whose overlay registers the
/// game window. A launcher or anti-cheat helper of the other bitness can win
/// that race and then exit, freezing the stream, so `auto` keeps only the
/// build matching the game binary.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OverlayPolicy {
    /// Keep the build matching the game binary's architecture
    #[default]
    Auto,
    /// Keep both builds, as Steam does
    Both,
    /// Keep only the 64-bit build
    X86_64,
    /// Keep only the 32-bit build
    I386,
}

fn default_skip_pre_command() -> bool {
    true
}
