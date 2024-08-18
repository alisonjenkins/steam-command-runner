pub mod error;
use crate::Config;
use error::SteamCommandRunnerError;
use std::path::PathBuf;
use std::process::Command;

#[derive(Default)]
pub struct SteamCommandRunner {
    pub config: Option<Config>,
}

impl SteamCommandRunner {
    /// Load config
    pub fn from_config(
        path: Option<PathBuf>,
    ) -> Result<SteamCommandRunner, SteamCommandRunnerError> {
        let mut config = Config::default();
        config.load(path)?;
        Ok(SteamCommandRunner {
            config: Some(config),
        })
    }

    /// Checks if we are in a gamescope session or not
    pub fn is_gamescope_session(&self) -> bool {
        match std::env::var("XDG_CURRENT_DESKTOP") {
            Ok(value) => value == "gamescope",
            Err(_) => false,
        }
    }

    /// Gets the args passed to steam-command-runner
    pub fn get_args(&self) -> Vec<String> {
        let mut args: Vec<String> = std::env::args().collect();
        args.remove(0);
        args
    }

    /// Adds the pre command and it's args to the command in the args vec
    pub fn add_pre_command(&self, args: &mut Vec<String>) -> Result<(), SteamCommandRunnerError> {
        let config = match &self.config {
            Some(config) => config,
            None => return Ok(()),
        };

        let pre_command: String = match &config.pre_command {
            Some(pre_command) => pre_command.to_string(),
            None => return Ok(()),
        };

        let mut command =
            shlex::split(&pre_command).ok_or(SteamCommandRunnerError::CouldNotSplitPreCommand)?;

        command.append(args);

        *args = command;

        Ok(())
    }

    /// Runs the game with the generated command
    pub fn run(&self, args: &mut Vec<String>) -> Result<(), SteamCommandRunnerError> {
        let command = args
            .first()
            .ok_or(SteamCommandRunnerError::CouldNotGetCommand)?
            .to_string();
        args.remove(0);

        let status = Command::new(command).args(args).status()?;
        println!("Command returned status: {status}");
        Ok(())
    }
}
