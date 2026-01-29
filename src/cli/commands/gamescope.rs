use crate::cli::GamescopeAction;
use crate::config::MergedConfig;
use crate::error::AppError;

/// Handle the gamescope command and its subcommands
pub fn handle_gamescope(action: GamescopeAction) -> Result<(), AppError> {
    match action {
        GamescopeAction::Args { app_id } => print_gamescope_args(app_id),
        GamescopeAction::Enabled { app_id } => print_gamescope_enabled(app_id),
    }
}

fn print_gamescope_args(app_id: Option<u32>) -> Result<(), AppError> {
    // Try to get app_id from environment if not provided
    let app_id = app_id.or_else(|| {
        std::env::var("SteamAppId")
            .ok()
            .and_then(|s| s.parse().ok())
    });

    // Load merged config
    let config = MergedConfig::load(app_id, None)?;

    // Check if gamescope is enabled
    if !config.gamescope_enabled {
        // Output nothing - gamescope is disabled for this game
        // This allows: gamescope $(steam-command-runner gamescope args) -- %command%
        // to become: gamescope -- %command% (which works fine)
        return Ok(());
    }

    // Output the gamescope args (just the args, no newline for clean substitution)
    if let Some(args) = config.gamescope_args {
        print!("{}", args);
    }

    Ok(())
}

fn print_gamescope_enabled(app_id: Option<u32>) -> Result<(), AppError> {
    // Try to get app_id from environment if not provided
    let app_id = app_id.or_else(|| {
        std::env::var("SteamAppId")
            .ok()
            .and_then(|s| s.parse().ok())
    });

    // Load merged config
    let config = MergedConfig::load(app_id, None)?;

    // Output true/false
    if config.gamescope_enabled && config.gamescope_args.is_some() {
        println!("true");
    } else {
        println!("false");
    }

    Ok(())
}
