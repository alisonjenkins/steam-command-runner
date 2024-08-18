use cucumber::{given, then, when, World};
use steamdeck_command_runner::{error::FindSteamUsersError, UpdateSteam};

#[derive(Debug, Default, World)]
pub struct UpdateSteamWorld {
    // config_path: PathBuf,
    // config: Config,
}

#[given("a computer with Steam installed")]
fn setup(world: &mut UpdateSteamWorld) {
    // world.config_path = path;
}

#[when("I scan for config files")]
fn scan(world: &mut UpdateSteamWorld) -> Result<(), FindSteamUsersError> {
    // Config::load(&mut world.config, Some(world.config_path.clone()))
}

#[then(regex = r#"we should find ([0-9]*) or more localconfig.vdf files"#)]
fn confirm(world: &mut UpdateSteamWorld, num_configs: String) {
    // assert_gt!(world.config.pre_command, Some(expected_value));
}

fn main() {
    futures::executor::block_on(ConfigWorld::run("tests/features/config/config.feature"));
}
