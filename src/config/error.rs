use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigLoadError {
    #[error("Could not find config file. Neither the HOME or XDG_CONFIG_HOME variables were set")]
    CouldNotFind,

    #[error("Could not open the config file from path: '{path}' with error: {source}")]
    CouldNotOpen {
        path: String,
        source: std::io::Error,
    },

    #[error("Could not read the config file from path: '{path}' with error: {source}")]
    CouldNotRead {
        path: String,
        source: std::io::Error,
    },

    #[error("Failed to parse config file: {source}")]
    FailedToParse {
        #[from]
        source: toml::de::Error,
    },
}
