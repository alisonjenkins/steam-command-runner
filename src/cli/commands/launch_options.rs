use crate::cli::LaunchOptionsAction;
use crate::error::AppError;
use crate::steam::{
    find_installed_games, find_user_ids, generate_default_launch_options, get_launch_options,
    get_localconfig_path, is_our_launch_options, read_localconfig, set_launch_options,
    write_localconfig,
};
use std::fs;
use tracing::{debug, info};

/// Handle the launch-options command and its subcommands
pub fn handle_launch_options(action: LaunchOptionsAction) -> Result<(), AppError> {
    match action {
        LaunchOptionsAction::SetAll {
            backup,
            dry_run,
            user_id,
        } => set_all(backup, dry_run, user_id),

        LaunchOptionsAction::Set {
            app_id,
            options,
            user_id,
        } => set_single(app_id, options, user_id),

        LaunchOptionsAction::ClearAll {
            backup,
            only_ours,
            user_id,
        } => clear_all(backup, only_ours, user_id),

        LaunchOptionsAction::Show { app_id, user_id } => show_single(app_id, user_id),

        LaunchOptionsAction::List { user_id } => list_all(user_id),
    }
}

/// Get the user ID to use, either from arg or auto-detect
fn resolve_user_id(user_id: Option<u64>) -> Result<u64, AppError> {
    match user_id {
        Some(id) => Ok(id),
        None => {
            let user_ids = find_user_ids()?;
            if user_ids.len() == 1 {
                Ok(user_ids[0])
            } else {
                // Try to get user names for better display
                let user_names = crate::steam::userdata::get_user_names().unwrap_or_default();
                
                println!("Multiple Steam users found:");
                for id in &user_ids {
                    if let Some(name) = user_names.get(id) {
                        println!("  {} ({})", id, name);
                    } else {
                        println!("  {}", id);
                    }
                }
                Err(AppError::SteamUserNotFound(
                    "Multiple users found. Please specify --user-id".to_string(),
                ))
            }
        }
    }
}

/// Create a backup of localconfig.vdf
fn create_backup(path: &std::path::Path) -> Result<(), AppError> {
    let backup_path = path.with_extension("vdf.backup");
    debug!("Creating backup: {}", backup_path.display());
    fs::copy(path, &backup_path)?;
    info!("Created backup: {}", backup_path.display());
    Ok(())
}

/// Set launch options for all installed games
fn set_all(backup: bool, dry_run: bool, user_id: Option<u64>) -> Result<(), AppError> {
    let user_id = resolve_user_id(user_id)?;
    let config_path = get_localconfig_path(user_id)?;
    let games = find_installed_games()?;

    if games.is_empty() {
        println!("No installed games found.");
        return Ok(());
    }

    let default_options = generate_default_launch_options();

    if dry_run {
        println!("Dry run - would set launch options for {} games:", games.len());
        println!("Launch options: {}", default_options);
        println!();
        for game in &games {
            println!("  {} ({})", game.name, game.app_id);
        }
        return Ok(());
    }

    if backup {
        create_backup(&config_path)?;
    }

    let mut config = read_localconfig(&config_path)?;

    let mut count = 0;
    for game in &games {
        set_launch_options(&mut config, game.app_id, Some(&default_options));
        count += 1;
    }

    write_localconfig(&config_path, &config)?;

    println!(
        "Set launch options for {} games in {}",
        count,
        config_path.display()
    );
    println!("Launch options: {}", default_options);
    println!();
    println!("Note: Restart Steam for changes to take effect.");

    Ok(())
}

/// Set launch options for a single game
fn set_single(app_id: u32, options: Option<String>, user_id: Option<u64>) -> Result<(), AppError> {
    let user_id = resolve_user_id(user_id)?;
    let config_path = get_localconfig_path(user_id)?;

    let launch_options = options.unwrap_or_else(generate_default_launch_options);

    let mut config = read_localconfig(&config_path)?;
    set_launch_options(&mut config, app_id, Some(&launch_options));
    write_localconfig(&config_path, &config)?;

    println!("Set launch options for app {}:", app_id);
    println!("  {}", launch_options);
    println!();
    println!("Note: Restart Steam for changes to take effect.");

    Ok(())
}

/// Clear launch options for all games
fn clear_all(backup: bool, only_ours: bool, user_id: Option<u64>) -> Result<(), AppError> {
    let user_id = resolve_user_id(user_id)?;
    let config_path = get_localconfig_path(user_id)?;
    let games = find_installed_games()?;

    if backup {
        create_backup(&config_path)?;
    }

    let mut config = read_localconfig(&config_path)?;

    let mut cleared = 0;
    let mut skipped = 0;

    for game in &games {
        if let Some(current_options) = get_launch_options(&config, game.app_id) {
            if only_ours && !is_our_launch_options(&current_options) {
                debug!(
                    "Skipping {} ({}) - not set by us",
                    game.name, game.app_id
                );
                skipped += 1;
                continue;
            }

            set_launch_options(&mut config, game.app_id, None);
            debug!("Cleared launch options for {} ({})", game.name, game.app_id);
            cleared += 1;
        }
    }

    write_localconfig(&config_path, &config)?;

    println!("Cleared launch options for {} games.", cleared);
    if skipped > 0 {
        println!("Skipped {} games (not set by steam-command-runner).", skipped);
    }
    println!();
    println!("Note: Restart Steam for changes to take effect.");

    Ok(())
}

/// Show launch options for a single game
fn show_single(app_id: u32, user_id: Option<u64>) -> Result<(), AppError> {
    let user_id = resolve_user_id(user_id)?;
    let config_path = get_localconfig_path(user_id)?;

    let config = read_localconfig(&config_path)?;

    match get_launch_options(&config, app_id) {
        Some(options) => {
            println!("Launch options for app {}:", app_id);
            println!("  {}", options);
            if is_our_launch_options(&options) {
                println!("  (set by steam-command-runner)");
            }
        }
        None => {
            println!("No launch options set for app {}.", app_id);
        }
    }

    Ok(())
}

/// List all games with their launch options
fn list_all(user_id: Option<u64>) -> Result<(), AppError> {
    let user_id = resolve_user_id(user_id)?;
    let config_path = get_localconfig_path(user_id)?;
    let games = find_installed_games()?;

    let config = read_localconfig(&config_path)?;

    let mut with_options = Vec::new();
    let mut without_options = Vec::new();

    for game in &games {
        if let Some(options) = get_launch_options(&config, game.app_id) {
            let ours = is_our_launch_options(&options);
            with_options.push((game, options, ours));
        } else {
            without_options.push(game);
        }
    }

    if !with_options.is_empty() {
        println!("Games with launch options:");
        for (game, options, ours) in &with_options {
            let marker = if *ours { " [ours]" } else { "" };
            println!("  {} ({}){}", game.name, game.app_id, marker);
            println!("    {}", options);
        }
        println!();
    }

    println!(
        "Games without launch options: {} (use 'launch-options set-all' to set)",
        without_options.len()
    );

    Ok(())
}
