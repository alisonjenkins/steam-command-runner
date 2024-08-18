use steamdeck_command_runner::{SteamCommandRunner, SteamCommandRunnerError};

fn main() -> Result<(), SteamCommandRunnerError> {
    let scr = SteamCommandRunner::from_config(None)?;
    let mut args = scr.get_args();

    if scr.is_gamescope_session() {
        return scr.run(&mut args);
    }

    scr.add_pre_command(&mut args)?;
    scr.run(&mut args)
}
