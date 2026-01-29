use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("No command specified. Did you forget to include the command to run?")]
    NoCommand,

    #[error("Configuration error: {0}")]
    Config(#[from] crate::config::ConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to execute command: {0}")]
    ExecutionFailed(String),

    #[error("Could not parse pre-command: {0}")]
    PreCommandParse(String),

    #[error("Could not parse gamescope args: {0}")]
    GamescopeArgsParse(String),

    #[error("Steam installation not found. Checked: {0:?}")]
    SteamNotFound(Vec<std::path::PathBuf>),

    #[error("Compatibility tool error: {0}")]
    CompatTool(String),

    #[error("Unknown verb '{0}'. Expected: waitforexitandrun, run, getcompatpath, getnativepath")]
    UnknownVerb(String),

    #[error("Steam API error: {0}")]
    SteamApi(String),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Proton version '{0}' not found")]
    ProtonNotFound(String),

    #[error("Editor '{0}' failed")]
    EditorFailed(String),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("Hook execution failed: {0}")]
    HookFailed(String),

    #[error("Steam user not found: {0}")]
    SteamUserNotFound(String),

    #[error("Failed to parse localconfig.vdf: {0}")]
    LocalConfigParseFailed(String),

    #[error("VDF serialization error: {0}")]
    VdfSerialize(String),

    #[error("Real gamescope binary not found in PATH")]
    GamescopeNotFound,

    #[error("Failed to parse gamescope shim arguments: {0}")]
    GamescopeShimParseFailed(String),
}
