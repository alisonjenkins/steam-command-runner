use cucumber::{given, then, when, World};
use std::path::PathBuf;
use steam_command_runner::config::{GlobalConfig, ConfigError};

#[derive(Debug, Default, World)]
pub struct ConfigWorld {
    config_path: PathBuf,
    config: GlobalConfig,
}

#[given(regex = "the config file (.*)")]
fn config_file(world: &mut ConfigWorld, path: PathBuf) {
    world.config_path = path;
}

#[when("I load the config")]
fn load_config(world: &mut ConfigWorld) -> Result<(), ConfigError> {
    let content = std::fs::read_to_string(&world.config_path)?;
    world.config = toml::from_str(&content)?;
    Ok(())
}

#[then(regex = "the config has pre-command set to \"(.*)\"")]
fn check_pre_command_value(world: &mut ConfigWorld, expected_value: String) {
    assert_eq!(world.config.pre_command, Some(expected_value));
}

fn main() {
    futures::executor::block_on(ConfigWorld::run("tests/features/config/config.feature"));
}
