use crate::config::ConfigLoadError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SteamCommandRunnerError {
    #[error("Could not get command. Did you forget to put %command% in the call to steam-command-runner?")]
    CouldNotGetCommand,

    #[error("Could not split pre-command. Did you not specify one?")]
    CouldNotSplitPreCommand,

    #[error("IO Error occurred")]
    IOError(#[from] std::io::Error),

    #[error("Could not load the config: {source}")]
    CouldNotLoadConfig {
        #[from]
        source: ConfigLoadError,
    },
}
