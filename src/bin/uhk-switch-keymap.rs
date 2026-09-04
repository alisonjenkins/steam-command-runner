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

    /// Override the UHK's USB vendor id (defaults to the UHK 80's).
    #[arg(long)]
    vendor_id: Option<u16>,

    /// Override the UHK's USB product id (defaults to the UHK 80's).
    #[arg(long)]
    product_id: Option<u16>,

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

    let vendor_id = args.vendor_id.unwrap_or(uhk::UHK_VENDOR_ID);
    let product_id = args.product_id.unwrap_or(uhk::UHK_PRODUCT_ID);

    match uhk::switch_keymap(&args.abbreviation, vendor_id, product_id) {
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
