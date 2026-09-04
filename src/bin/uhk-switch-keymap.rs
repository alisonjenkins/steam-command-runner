use clap::Parser;
use std::process::ExitCode;
use steam_command_runner::uhk;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

/// Switch the active keymap on an Ultimate Hacking Keyboard over raw USB
/// HID -- no UHK Agent required. Meant to be wired into a game's
/// `hooks.pre_launch` / `hooks.post_exit` config so the keyboard follows
/// whichever game is running: point `pre_launch` at this binary with the
/// game's keymap abbreviation, and `post_exit` at it again with your
/// default keymap's abbreviation.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Keymap abbreviation, exactly as shown in UHK Agent (max 3 characters).
    abbreviation: String,

    #[arg(short, long)]
    verbose: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();

    let level = if args.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_ansi(true)
        .init();

    match uhk::switch_keymap(&args.abbreviation) {
        Ok(()) => {
            tracing::info!(abbreviation = %args.abbreviation, "switched UHK keymap");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}
