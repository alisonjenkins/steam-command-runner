use crate::config::MergedConfig;
use crate::hooks;
use crate::shim::launch::{
    binary_arch, direct_command, effective_ld_preload, filter_overlay, find_game_binary,
    gamescope_decision, launch_mode, overlay_to_keep, LaunchMode,
};
use crate::shim::resolution;
use crate::shim::stream_target::{self, StreamTarget};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Check if the current binary was invoked as "gamescope"
pub fn is_invoked_as_gamescope() -> bool {
    std::env::args()
        .next()
        .map(|arg0| {
            Path::new(&arg0)
                .file_name()
                .map(|name| name == "gamescope")
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

/// Parse gamescope arguments, splitting at "--" into (gamescope_args, command)
fn parse_gamescope_args(args: Vec<String>) -> (Vec<String>, Vec<String>) {
    let mut gamescope_args = Vec::new();
    let mut command = Vec::new();
    let mut found_separator = false;

    for arg in args.into_iter().skip(1) {
        // Skip argv[0]
        if !found_separator && arg == "--" {
            found_separator = true;
            continue;
        }

        if found_separator {
            command.push(arg);
        } else {
            gamescope_args.push(arg);
        }
    }

    (gamescope_args, command)
}

/// Get the Steam App ID from environment
fn get_app_id() -> Option<u32> {
    env::var("SteamAppId").ok().and_then(|s| s.parse().ok())
}

/// Find the real gamescope binary, excluding ourselves
fn find_real_gamescope() -> Option<PathBuf> {
    // Get our own inode to exclude from search
    let self_path = std::env::current_exe().ok()?;
    let self_inode = fs::metadata(&self_path).ok()?.ino();

    // Search PATH for gamescope
    let path_env = std::env::var("PATH").ok()?;

    for dir in path_env.split(':') {
        let candidate = Path::new(dir).join("gamescope");

        if !candidate.exists() {
            continue;
        }

        // Check if it's a different file (by inode) to skip our symlink
        if let Ok(metadata) = fs::metadata(&candidate) {
            // Follow symlinks to get the real file
            if let Ok(canonical) = fs::canonicalize(&candidate) {
                if let Ok(canonical_meta) = fs::metadata(&canonical) {
                    if canonical_meta.ino() != self_inode {
                        return Some(candidate);
                    }
                }
            } else if metadata.ino() != self_inode {
                return Some(candidate);
            }
        }
    }

    None
}

/// Handle execution when invoked as the gamescope shim
/// Load the full merged configuration
fn load_config() -> Option<MergedConfig> {
    let app_id = get_app_id();
    MergedConfig::load(app_id, None).ok()
}

/// Handle execution when invoked as the gamescope shim
pub fn handle_gamescope_shim() -> ExitCode {
    // Load config first to check logging preference
    let config = load_config();
    let debug_enabled = config.as_ref().map(|c| c.shim_debug).unwrap_or(false);

    log_to_file("Shim started", debug_enabled);
    let args: Vec<String> = std::env::args().collect();
    log_to_file(&format!("Args: {:?}", args), debug_enabled);

    // Before pre_launch, so a hook that backs up or syncs settings never sees
    // the stream size a killed launch left behind.
    let journal = get_app_id().and_then(resolution::journal_path);
    if let Some(path) = &journal {
        resolution::Restore::recover(path.clone(), log_decision);
    }

    if let Some(hook) = config.as_ref().and_then(|c| c.pre_launch_hook.as_ref()) {
        log_to_file(
            &format!("Running pre_launch hook: {}", hook.command),
            debug_enabled,
        );
        if let Err(e) = hooks::execute(hook) {
            log_to_file(&format!("pre_launch hook failed: {}", e), debug_enabled);
            eprintln!("pre_launch hook failed: {}", e);
        }
    }
    let (cli_gamescope_args, command) = parse_gamescope_args(args);

    let stream_target = StreamTarget::detect();
    let bypass_when_streaming = config
        .as_ref()
        .map(|c| c.stream_bypass_gamescope)
        .unwrap_or(true);
    let mode = launch_mode(
        config.as_ref().map(|c| c.gamescope_enabled).unwrap_or(true),
        bypass_when_streaming,
        stream_target.is_some(),
    );
    // With no game command there is nothing to run directly; keep gamescope.
    let direct = mode == LaunchMode::Direct && !command.is_empty();
    // Only a direct launch: under gamescope, -W/-H already set the size.
    let (resolution_args, resolution_applied) = if direct {
        stream_resolution(config.as_ref(), &command, stream_target.as_ref())
    } else {
        (Vec::new(), Vec::new())
    };
    let restore_resolution = resolution::Restore::new(resolution_applied, journal, log_decision);
    let mut cmd = if direct {
        direct_launch(
            config.as_ref(),
            &command,
            stream_target.as_ref(),
            &resolution_args,
            debug_enabled,
        )
    } else {
        log_decision(&gamescope_decision(
            stream_target.as_ref().map(|t| t.output.as_str()),
            bypass_when_streaming,
            !command.is_empty(),
        ));
        match gamescope_launch(
            config.as_ref(),
            cli_gamescope_args,
            &command,
            stream_target,
            debug_enabled,
        ) {
            Some(cmd) => cmd,
            None => return ExitCode::FAILURE,
        }
    };

    // spawn()+wait() rather than exec(): a post_exit hook needs this process
    // to still be here once the game has exited. Command::spawn() inherits
    // the parent's environment the same way exec() did, so this doesn't change
    // what Steam-set vars (LIBEI_SOCKET, LD_PRELOAD) the game sees.
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            log_to_file(
                &format!("Error: Failed to spawn {:?}: {}", cmd, e),
                debug_enabled,
            );
            eprintln!("Error: Failed to spawn {:?}: {}", cmd.get_program(), e);
            return ExitCode::FAILURE;
        }
    };

    let status = match child.wait() {
        Ok(status) => status,
        Err(e) => {
            log_to_file(
                &format!("Error: Failed to wait on game: {}", e),
                debug_enabled,
            );
            eprintln!("Error: Failed to wait on game: {}", e);
            return ExitCode::FAILURE;
        }
    };

    // Restore before post_exit, so the hook sees the player's own settings.
    drop(restore_resolution);

    if let Some(hook) = config.as_ref().and_then(|c| c.post_exit_hook.as_ref()) {
        log_to_file(
            &format!("Running post_exit hook: {}", hook.command),
            debug_enabled,
        );
        if let Err(e) = hooks::execute(hook) {
            log_to_file(&format!("post_exit hook failed: {}", e), debug_enabled);
            eprintln!("post_exit hook failed: {}", e);
        }
    }

    match status.code() {
        Some(code) => ExitCode::from(code.clamp(0, 255) as u8),
        None => ExitCode::FAILURE,
    }
}

/// Make a streamed direct launch render at the client's resolution.
///
/// Returns engine arguments for the game's command line, and the settings
/// rules applied, which are undone once the game exits. Nothing for a local
/// launch, or when the client's size is unknown.
fn stream_resolution(
    config: Option<&MergedConfig>,
    command: &[String],
    target: Option<&StreamTarget>,
) -> (Vec<String>, Vec<resolution::Applied>) {
    let Some(target) = target.filter(|t| t.width > 0 && t.height > 0) else {
        return (Vec::new(), Vec::new());
    };
    if !config.is_none_or(|c| c.stream_set_resolution) {
        return (Vec::new(), Vec::new());
    }
    let (width, height) = (target.width, target.height);
    let game = find_game_binary(command);

    let prefix = env::var("STEAM_COMPAT_DATA_PATH")
        .ok()
        .map(|p| format!("{p}/pfx"));
    let mut applied = Vec::new();

    let args = match game
        .as_deref()
        .and_then(|g| Some((g, resolution::detect_engine(g)?)))
    {
        Some((game, engine)) => {
            let args = resolution::engine_args(engine, width, height);
            log_decision(&format!(
                "rendering at {width}x{height}: {engine:?} engine, {}",
                args.join(" ")
            ));
            applied.extend(
                resolution::engine_saves(engine, game, prefix.as_deref())
                    .iter()
                    .filter_map(resolution::snapshot),
            );
            args
        }
        None => Vec::new(),
    };

    let game_dir = game
        .as_deref()
        .and_then(Path::parent)
        .map(|d| d.to_string_lossy().into_owned());
    for rule in config.map_or(&[][..], |c| &c.stream_resolution_rules) {
        let Some(file) =
            resolution::expand_path(&rule.file, prefix.as_deref(), game_dir.as_deref())
        else {
            log_decision(&format!(
                "resolution rule skipped, cannot place {}",
                rule.file
            ));
            continue;
        };
        match resolution::apply_rule(rule, &file, width, height) {
            Ok(done) => {
                log_decision(&format!(
                    "rendering at {width}x{height}: set in {}",
                    file.display()
                ));
                applied.push(done);
            }
            Err(e) => log_decision(&format!("resolution rule failed: {e}")),
        }
    }
    (args, applied)
}

/// Run the game on the host's own display instead of inside gamescope.
///
/// Steam only streams in game mode, and only then lets the client capture the
/// mouse, when it can find the game window on its own X display. gamescope
/// moves the window to a nested one, so a streamed game is launched bare.
fn direct_launch(
    config: Option<&MergedConfig>,
    command: &[String],
    stream_target: Option<&StreamTarget>,
    resolution_args: &[String],
    debug_enabled: bool,
) -> std::process::Command {
    let mut full = direct_command(
        config.and_then(|c| c.effective_pre_command()),
        command,
        config.and_then(|c| c.game_args.as_deref()),
    );
    full.extend_from_slice(resolution_args);
    // full is non-empty: the caller only takes this path with a game command.
    let (program, rest) = full
        .split_first()
        .map_or(("", &[][..]), |(p, r)| (p.as_str(), r));

    let mut cmd = std::process::Command::new(program);
    cmd.args(rest);
    if let Some(c) = config {
        cmd.envs(&c.env);
        cmd.envs(&c.inner_env);
    }

    let mut summary = match stream_target {
        Some(target) => format!(
            "streaming to {}, launching without gamescope",
            if target.output.is_empty() {
                "a Remote Play client"
            } else {
                &target.output
            }
        ),
        None => "gamescope disabled, launching without gamescope".to_string(),
    };

    // Outside a stream Steam's own LD_PRELOAD is left exactly as it set it.
    if stream_target.is_some() {
        let game = find_game_binary(command);
        let game_arch = game.as_deref().and_then(binary_arch);
        let policy = config.map(|c| c.stream_overlay).unwrap_or_default();
        let keep = overlay_to_keep(policy, game_arch);

        let empty = HashMap::new();
        let ld_preload = effective_ld_preload(
            config.map_or(&empty, |c| &c.env),
            config.map_or(&empty, |c| &c.inner_env),
            env::var("LD_PRELOAD").ok(),
        );
        if let Some(ld_preload) = ld_preload {
            cmd.env("LD_PRELOAD", filter_overlay(&ld_preload, keep));
        }

        summary.push_str(&match keep {
            Some(arch) => format!(", overlay {:?} only", arch),
            None => ", both overlays".to_string(),
        });
        summary.push_str(&match (&game, game_arch) {
            (Some(path), Some(_)) => format!(" (from {})", path.display()),
            (Some(path), None) => format!(" (architecture of {} unknown)", path.display()),
            (None, _) => " (no game binary found in the command)".to_string(),
        });
    }

    eprintln!("steam-command-runner: {}", summary);
    log_decision(&summary);
    log_to_file(&format!("Executing directly: {:?}", full), debug_enabled);
    cmd
}

/// Build the gamescope command wrapping the game.
fn gamescope_launch(
    config: Option<&MergedConfig>,
    cli_gamescope_args: Vec<String>,
    command: &[String],
    stream_target: Option<StreamTarget>,
    debug_enabled: bool,
) -> Option<std::process::Command> {
    // Get gamescope args from config
    let config_gamescope_args = if let Some(c) = &config {
        if c.gamescope_enabled {
            match &c.gamescope_args {
                Some(args_str) => shlex::split(args_str).unwrap_or_default(),
                None => Vec::new(),
            }
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let mut all_gamescope_args = config_gamescope_args;
    all_gamescope_args.extend(cli_gamescope_args);

    // A game launched while a client is streaming has to render at that
    // client's resolution. gamescope fixes its render size at startup, so a
    // game started with the desktop's geometry is only scaled into the
    // smaller output afterwards and stays letterboxed. This is the one point
    // in Steam's launch chain that sees the arguments in time.
    // A target known only from Steam's environment may lack a readable size.
    if let Some(target) = stream_target.filter(|t| t.width > 0 && t.height > 0) {
        log_to_file(
            &format!(
                "Stream target active: {:?}, rewriting size and output",
                target
            ),
            debug_enabled,
        );

        // gamescope resolves --prefer-output when it starts, so launching
        // before the output exists puts the game on the desktop and nothing
        // moves it afterwards. The host creates the output when a client
        // connects, well before a game is normally launched; this only covers
        // a launch that races it.
        if !target.output.is_empty() && !stream_target::wait_for_output(&target.output) {
            log_to_file(
                &format!(
                    "Output {} did not appear; launching anyway at its size",
                    target.output
                ),
                debug_enabled,
            );
        }

        all_gamescope_args = stream_target::apply(all_gamescope_args, &target);
    }

    // Find the real gamescope binary
    let real_gamescope = match find_real_gamescope() {
        Some(path) => {
            log_to_file(
                &format!("Found real gamescope at: {:?}", path),
                debug_enabled,
            );
            path
        }
        None => {
            log_to_file(
                "Error: Real gamescope binary not found in PATH",
                debug_enabled,
            );
            eprintln!("Error: Real gamescope binary not found in PATH");
            eprintln!("Make sure gamescope is installed and the steam-command-runner symlink");
            eprintln!("is not shadowing the real gamescope binary.");
            return None;
        }
    };

    let mut cmd = std::process::Command::new(&real_gamescope);
    cmd.args(&all_gamescope_args);
    log_to_file(
        &format!(
            "Executing: {:?} args: {:?}",
            real_gamescope, all_gamescope_args
        ),
        debug_enabled,
    );

    // Apply environment variables from config
    if let Some(c) = &config {
        for (key, value) in &c.env {
            log_to_file(&format!("Setting env: {}={}", key, value), debug_enabled);
            if is_compositor_hostile(key) {
                let warning = format!(
                    "Warning: {} is set in [env], so gamescope inherits it. \
                     Move it to [inner_env] to keep it out of the compositor.",
                    key
                );
                log_to_file(&warning, debug_enabled);
                eprintln!("{}", warning);
            }
            cmd.env(key, value);
        }
    }

    // We CANNOT successfully set LD_PRELOAD on the gamescope process itself
    // because gamescope has capabilities (cap_sys_nice) which causes the OS to strip insecure env vars.
    // Instead, we must inject it into the INNER command using 'env'.

    // Set Gamescope Overlay variables (These are likely safe from stripping or gamescope might use them)
    log_to_file(
        "Setting ENABLE_VK_LAYER_VALVE_steam_overlay_1=1",
        debug_enabled,
    );
    cmd.env("ENABLE_VK_LAYER_VALVE_steam_overlay_1", "1");

    log_to_file("Setting ENABLE_GAMESCOPE_WSI=1", debug_enabled);
    cmd.env("ENABLE_GAMESCOPE_WSI", "1");

    // Copy STEAM_GAMESCOPE_* env vars
    cmd.env("STEAM_GAMESCOPE_NIS_SUPPORTED", "1");
    cmd.env("STEAM_GAMESCOPE_HDR_SUPPORTED", "1");
    cmd.env("STEAM_GAMESCOPE_VRR_SUPPORTED", "1");
    cmd.env("STEAM_GAMESCOPE_TEARING_SUPPORTED", "1");
    cmd.env("STEAM_GAMESCOPE_HAS_TEARING_SUPPORT", "1");

    if !command.is_empty() {
        cmd.arg("--");

        // Inject the Steam overlay's LD_PRELOAD and any inner_env vars via an
        // 'env' wrapper on the inner command
        let inner_assignments = config
            .as_ref()
            .map(|c| c.inner_env_assignments())
            .unwrap_or_default();
        let env_wrapper = build_inner_env_wrapper(
            build_ld_preload_with_overlay(debug_enabled),
            inner_assignments,
        );
        if !env_wrapper.is_empty() {
            log_to_file(
                &format!("Injecting inner 'env' wrapper: {:?}", env_wrapper),
                debug_enabled,
            );
            cmd.args(&env_wrapper);
        }

        // Inject pre_command (e.g., mangohud) into inner command
        // This ensures it runs AFTER gamescope has started, avoiding capability stripping
        if let Some(c) = &config {
            if let Some(pre_cmd) = c.effective_pre_command() {
                log_to_file(
                    &format!("Injecting pre_command: {}", pre_cmd),
                    debug_enabled,
                );
                if let Some(pre_args) = shlex::split(pre_cmd) {
                    cmd.args(pre_args);
                }
            }
        }

        cmd.args(command);

        // Append explicit game_args from config (e.g. --skip-intro)
        if let Some(c) = &config {
            if let Some(args_str) = &c.game_args {
                log_to_file(&format!("Appending game_args: {}", args_str), debug_enabled);
                if let Some(extra_args) = shlex::split(args_str) {
                    cmd.args(extra_args);
                }
            }
        }
    }

    Some(cmd)
}

/// Variables that break gamescope if the compositor process inherits them
///
/// MangoHud and vkBasalt are loaded by implicit Vulkan layers keyed off these
/// variables, so gamescope picks up an overlay in its own Vulkan instance —
/// useless (the visible HUD comes from the game's instance) and, for MangoHud,
/// fatal at exit.
const COMPOSITOR_HOSTILE_VARS: &[&str] = &["MANGOHUD", "MANGOHUD_CONFIG", "ENABLE_VKBASALT"];

fn is_compositor_hostile(key: &str) -> bool {
    COMPOSITOR_HOSTILE_VARS.contains(&key)
}

/// Build the `env KEY=VALUE ...` prefix for the inner command
///
/// Returns an empty vector when there is nothing to set, so the caller can skip
/// the wrapper entirely.
fn build_inner_env_wrapper(ld_preload: Option<String>, inner_env: Vec<String>) -> Vec<String> {
    let mut assignments = Vec::new();

    if let Some(ld_preload) = ld_preload {
        assignments.push(format!("LD_PRELOAD={}", ld_preload));
    }
    assignments.extend(inner_env);

    if assignments.is_empty() {
        return Vec::new();
    }

    let mut wrapper = vec!["env".to_string()];
    wrapper.extend(assignments);
    wrapper
}

/// Get the Steam overlay library paths for LD_PRELOAD
fn get_steam_overlay_paths(debug: bool) -> Option<String> {
    // Try to find Steam installation path
    let home = std::env::var("HOME").ok()?;
    let steam_path = PathBuf::from(&home).join(".local/share/Steam");

    let overlay_64 = steam_path.join("ubuntu12_64/gameoverlayrenderer.so");
    let overlay_32 = steam_path.join("ubuntu12_32/gameoverlayrenderer.so");

    if overlay_64.exists() {
        let mut paths = overlay_64.to_string_lossy().to_string();
        if overlay_32.exists() {
            paths.push(':');
            paths.push_str(&overlay_32.to_string_lossy());
        }
        log_to_file(&format!("Found Steam overlay paths: {}", paths), debug);
        Some(paths)
    } else {
        log_to_file("Steam overlay 64-bit library not found!", debug);
        None
    }
}

/// Build LD_PRELOAD value with Steam overlay added
fn build_ld_preload_with_overlay(debug: bool) -> Option<String> {
    let overlay_paths = get_steam_overlay_paths(debug)?;

    // Check existing LD_PRELOAD
    let existing_preload = std::env::var("LD_PRELOAD").ok();

    if let Some(existing) = existing_preload {
        if existing.contains("gameoverlayrenderer.so") {
            log_to_file(
                "LD_PRELOAD already contains gameoverlayrenderer.so, mimicking it",
                debug,
            );
            Some(existing)
        } else {
            let new_preload = format!("{}:{}", overlay_paths, existing);
            log_to_file("Prepending overlay to existing LD_PRELOAD", debug);
            Some(new_preload)
        }
    } else {
        log_to_file("Setting new LD_PRELOAD with overlay", debug);
        Some(overlay_paths)
    }
}

/// The one line written whatever `shim_debug` says. Steam discards a launched
/// game's stderr, so without it nothing shows which way a launch went.
fn log_decision(message: &str) {
    let app = get_app_id().map_or_else(|| "?".to_string(), |id| id.to_string());
    log_to_file(
        &format!(
            "{} app {}: {}",
            humantime::format_rfc3339_seconds(std::time::SystemTime::now()),
            app,
            message
        ),
        true,
    );
}

fn log_to_file(message: &str, enabled: bool) {
    if !enabled {
        return;
    }
    if let Ok(home) = std::env::var("HOME") {
        let log_path = PathBuf::from(&home).join(".steam-command-runner-shim.log");
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            let _ = writeln!(file, "{}", message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gamescope_args_with_command() {
        let args = vec![
            "gamescope".to_string(),
            "-w".to_string(),
            "1920".to_string(),
            "-h".to_string(),
            "1080".to_string(),
            "--".to_string(),
            "/path/to/game".to_string(),
            "arg1".to_string(),
        ];

        let (gs_args, cmd) = parse_gamescope_args(args);

        assert_eq!(gs_args, vec!["-w", "1920", "-h", "1080"]);
        assert_eq!(cmd, vec!["/path/to/game", "arg1"]);
    }

    #[test]
    fn test_parse_gamescope_args_no_command() {
        let args = vec![
            "gamescope".to_string(),
            "-f".to_string(),
            "--fullscreen".to_string(),
        ];

        let (gs_args, cmd) = parse_gamescope_args(args);

        assert_eq!(gs_args, vec!["-f", "--fullscreen"]);
        assert!(cmd.is_empty());
    }

    #[test]
    fn test_inner_env_wrapper_combines_preload_and_inner_env() {
        let wrapper = build_inner_env_wrapper(
            Some("/steam/overlay.so".to_string()),
            vec!["MANGOHUD=1".to_string()],
        );

        assert_eq!(
            wrapper,
            vec!["env", "LD_PRELOAD=/steam/overlay.so", "MANGOHUD=1"]
        );
    }

    #[test]
    fn test_inner_env_wrapper_without_preload() {
        let wrapper = build_inner_env_wrapper(None, vec!["MANGOHUD=1".to_string()]);

        assert_eq!(wrapper, vec!["env", "MANGOHUD=1"]);
    }

    #[test]
    fn test_inner_env_wrapper_empty_when_nothing_to_set() {
        assert!(build_inner_env_wrapper(None, Vec::new()).is_empty());
    }

    #[test]
    fn test_compositor_hostile_vars() {
        assert!(is_compositor_hostile("MANGOHUD"));
        assert!(is_compositor_hostile("ENABLE_VKBASALT"));
        assert!(!is_compositor_hostile("DXVK_ASYNC"));
    }

    #[test]
    fn test_parse_gamescope_args_empty() {
        let args = vec!["gamescope".to_string()];

        let (gs_args, cmd) = parse_gamescope_args(args);

        assert!(gs_args.is_empty());
        assert!(cmd.is_empty());
    }
}
