use cucumber::{given, then, when, World};

#[derive(Debug, Default, World)]
pub struct UpdateSteamWorld {
    found_configs: usize,
}

#[given("a computer with Steam installed")]
fn setup(_world: &mut UpdateSteamWorld) {
    // Setup for test
}

#[when("I scan for config files")]
fn scan(world: &mut UpdateSteamWorld) {
    // Scan for Steam config files
    let home = dirs::home_dir().unwrap_or_default();
    let steam_path = home.join(".steam/steam/userdata");

    if steam_path.exists() {
        if let Ok(entries) = std::fs::read_dir(&steam_path) {
            world.found_configs = entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .count();
        }
    }
}

#[then(regex = r#"we should find ([0-9]*) or more localconfig.vdf files"#)]
fn confirm(world: &mut UpdateSteamWorld, num_configs: String) {
    let expected: usize = num_configs.parse().unwrap_or(0);
    assert!(world.found_configs >= expected,
        "Expected at least {} configs, found {}", expected, world.found_configs);
}

fn main() {
    futures::executor::block_on(UpdateSteamWorld::run("tests/features/update-steam/update-steam.feature"));
}
