use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "steam-command-runner")]
#[command(version, about = "Steam compatibility tool and command wrapper for Linux gaming")]
#[command(after_help = "Use as a Steam compatibility tool or standalone command wrapper.\n\n\
    Examples:\n  \
    steam-command-runner run /path/to/game\n  \
    steam-command-runner install\n  \
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

    /// Arguments passed directly (for legacy/compatibility tool mode)
    #[arg(trailing_var_arg = true, hide = true)]
    pub args: Vec<String>,
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

    /// Install as a Steam compatibility tool
    Install {
        /// Custom tool name (default: steam-command-runner)
        #[arg(short, long)]
        name: Option<String>,

        /// Steam installation path override
        #[arg(short, long)]
        steam_path: Option<PathBuf>,

        /// Require a specific Proton version as the underlying layer
        #[arg(short, long)]
        require_proton: Option<String>,
    },

    /// Uninstall the Steam compatibility tool
    Uninstall {
        /// Steam installation path override
        #[arg(short, long)]
        steam_path: Option<PathBuf>,
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

    /// Compatibility tool entry point (called by Steam)
    #[command(hide = true)]
    Compat {
        /// The verb passed by Steam (waitforexitandrun, run, etc.)
        verb: String,

        /// Remaining arguments from Steam
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
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
        #[arg(short, long)]
        app_id: Option<u32>,
    },

    /// Show configuration file path
    Path {
        /// App ID to show path for (omit for global config)
        #[arg(short, long)]
        app_id: Option<u32>,
    },
}
