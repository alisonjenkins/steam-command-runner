use crate::compat_tool::{CompatToolContext, Verb};
use crate::config::MergedConfig;
use crate::error::AppError;
use crate::hooks;
use crate::runner::execute_game;
use std::process::ExitCode;
use tracing::{debug, info, warn};

/// Handle the compat command - entry point when called by Steam
pub fn handle_compat(verb: String, args: Vec<String>) -> Result<ExitCode, AppError> {
    let ctx = CompatToolContext::from_env_and_args(&verb, args)?;

    info!(
        "Compatibility tool invoked: verb={:?}, app_id={:?}",
        ctx.verb, ctx.steam_app_id
    );
    debug!("Game path: {:?}", ctx.game_path);
    debug!("Game args: {:?}", ctx.game_args);

    // Handle non-execution verbs
    match ctx.verb {
        Verb::GetCompatPath => {
            if let Some(ref path) = ctx.compat_data_path {
                println!("{}", path.display());
            }
            return Ok(ExitCode::SUCCESS);
        }
        Verb::GetNativePath => {
            println!("{}", ctx.game_path.display());
            return Ok(ExitCode::SUCCESS);
        }
        _ => {}
    }

    // Load configuration based on app ID
    let config = MergedConfig::load(ctx.steam_app_id, None)?;
    debug!("Merged config: {:?}", config);

    // Execute pre-launch hook
    if let Some(ref hook) = config.pre_launch_hook {
        info!("Executing pre-launch hook");
        if let Err(e) = hooks::execute(hook) {
            warn!("Pre-launch hook failed: {}", e);
        }
    }

    // Build command with game path and args
    let mut command = vec![ctx.game_path.to_string_lossy().to_string()];
    command.extend(ctx.game_args);

    // Execute the game
    let exit_code = execute_game(&config, command)?;

    // Execute post-exit hook
    if let Some(ref hook) = config.post_exit_hook {
        info!("Executing post-exit hook");
        if let Err(e) = hooks::execute(hook) {
            warn!("Post-exit hook failed: {}", e);
        }
    }

    Ok(exit_code)
}
