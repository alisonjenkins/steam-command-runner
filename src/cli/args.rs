use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "steam-command-runner")]
#[command(version, about = "Command wrapper for Linux gaming with gamescope integration")]
#[command(after_help = "Examples:\n  \
    steam-command-runner run /path/to/game\n  \
    steam-command-runner launch-options set-all --dry-run\n  \
    steam-command-runner search \"Half-Life\"")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Config file path override
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run a command with configured wrappers
    Run {
        /// Steam App ID (optional, for per-game config)
        #[arg(short, long)]
        app_id: Option<u32>,

        /// Command and arguments to run
        #[arg(trailing_var_arg = true, required = true)]
        command: Vec<String>,
    },

    /// Install the gamescope shim symlink
    Install {
        /// Custom path for the symlink (default: ~/.local/bin/gamescope)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },

    /// Uninstall the gamescope shim symlink
    Uninstall {
        /// Path to the symlink (default: ~/.local/bin/gamescope)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },

    /// Search for a game's Steam App ID
    Search {
        /// Game name to search for
        query: String,

        /// Maximum number of results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Proton version management
    Proton {
        #[command(subcommand)]
        action: ProtonAction,
    },

    /// Gamescope argument management
    Gamescope {
        #[command(subcommand)]
        action: GamescopeAction,
    },

    /// Manage Steam launch options for games
    LaunchOptions {
        #[command(subcommand)]
        action: LaunchOptionsAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show {
        /// App ID to show merged config for
        #[arg(short, long)]
        app_id: Option<u32>,
    },

    /// Initialize configuration with defaults
    Init,

    /// Edit configuration in default editor
    Edit {
        /// App ID to edit (omit for global config)
        #[arg(short, long, conflicts_with = "name")]
        app_id: Option<u32>,

        /// Game name to search for (resolves to App ID)
        #[arg(short, long, conflicts_with = "app_id")]
        name: Option<String>,
    },

    /// Show configuration file path
    Path {
        /// App ID to show path for (omit for global config)
        #[arg(short, long)]
        app_id: Option<u32>,
    },
}

#[derive(Subcommand)]
pub enum ProtonAction {
    /// List available Proton versions
    List {
        /// Show full paths instead of just names
        #[arg(short, long)]
        paths: bool,
    },
}

#[derive(Subcommand)]
pub enum GamescopeAction {
    /// Output gamescope arguments for use in Steam launch options
    ///
    /// Use in Steam launch options like:
    /// gamescope $(steam-command-runner gamescope args) -- %command%
    Args {
        /// App ID to get gamescope args for (uses SteamAppId env var if not specified)
        #[arg(short, long)]
        app_id: Option<u32>,
    },

    /// Check if gamescope is enabled for a game
    ///
    /// Outputs "true" or "false"
    Enabled {
        /// App ID to check (uses SteamAppId env var if not specified)
        #[arg(short, long)]
        app_id: Option<u32>,
    },
}

#[derive(Subcommand)]
pub enum LaunchOptionsAction {
    /// Set launch options for all installed games
    SetAll {
        /// Create a backup of localconfig.vdf before modifying
        #[arg(short, long, default_value = "true")]
        backup: bool,

        /// Show what would be changed without actually modifying
        #[arg(short, long)]
        dry_run: bool,

        /// Steam user ID (auto-detected if not specified)
        #[arg(short, long)]
        user_id: Option<u64>,
    },

    /// Set launch options for a specific game
    Set {
        /// Steam App ID
        #[arg(short, long)]
        app_id: u32,

        /// Launch options to set (uses default if not specified)
        #[arg(short, long)]
        options: Option<String>,

        /// Steam user ID (auto-detected if not specified)
        #[arg(short, long)]
        user_id: Option<u64>,
    },

    /// Clear launch options for all games
    ClearAll {
        /// Create a backup of localconfig.vdf before modifying
        #[arg(short, long, default_value = "true")]
        backup: bool,

        /// Only clear launch options set by steam-command-runner
        #[arg(short, long, default_value = "true")]
        only_ours: bool,

        /// Steam user ID (auto-detected if not specified)
        #[arg(short, long)]
        user_id: Option<u64>,
    },

    /// Show launch options for a specific game
    Show {
        /// Steam App ID
        #[arg(short, long)]
        app_id: u32,

        /// Steam user ID (auto-detected if not specified)
        #[arg(short, long)]
        user_id: Option<u64>,
    },

    /// List all games with their current launch options
    List {
        /// Steam user ID (auto-detected if not specified)
        #[arg(short, long)]
        user_id: Option<u64>,
    },
}
