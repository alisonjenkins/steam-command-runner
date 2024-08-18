mod error;

pub use error::ConfigLoadError;
use serde::Deserialize;
use std::io::Read;
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub pre_command: Option<String>,
}

fn get_config_path() -> Option<PathBuf> {
    if let Ok(value) = std::env::var("XDG_CONFIG_HOME") {
        let path: PathBuf = value.into();
        let path = path.join(".config/steamdeck-command-runner/config.toml");
        if path.exists() {
            return Some(path);
        }
    };

    if let Ok(value) = std::env::var("HOME") {
        let path: PathBuf = value.into();
        let path = path.join(".config/steamdeck-command-runner/config.toml");
        if path.exists() {
            return Some(path);
        }
    };

    None
}

impl Config {
    pub fn load(&mut self, path: Option<PathBuf>) -> Result<(), ConfigLoadError> {
        let config_file_path = if let Some(path) = path {
            path
        } else if let Some(path) = get_config_path() {
            path
        } else {
            return Err(ConfigLoadError::CouldNotFind);
        };

        let mut config_file = std::fs::File::open(&config_file_path).map_err(|source| {
            ConfigLoadError::CouldNotOpen {
                path: config_file_path.as_path().display().to_string(),
                source,
            }
        })?;

        let mut buffer = String::new();
        config_file.read_to_string(&mut buffer).map_err(|source| {
            ConfigLoadError::CouldNotRead {
                path: config_file_path.as_path().display().to_string(),
                source,
            }
        })?;

        *self = toml::from_str(&buffer)?;

        Ok(())
    }
}
