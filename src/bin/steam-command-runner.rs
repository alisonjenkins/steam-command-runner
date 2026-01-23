use clap::Parser;
use std::process::ExitCode;
use steam_command_runner::cli::commands::{
    handle_compat, handle_config, handle_install, handle_run, handle_search, handle_uninstall,
};
use steam_command_runner::{AppError, Cli, Commands};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Initialize logging
    let level = if cli.verbose { Level::DEBUG } else { Level::INFO };
    FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_ansi(true)
        .init();

    let result = run(cli);

    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode, AppError> {
    match cli.command {
        Some(Commands::Run { app_id, command }) => {
            handle_run(app_id, command, cli.config)?;
            Ok(ExitCode::SUCCESS)
        }

        Some(Commands::Install {
            name,
            steam_path,
            require_proton,
        }) => {
            handle_install(name, steam_path, require_proton)?;
            Ok(ExitCode::SUCCESS)
        }

        Some(Commands::Uninstall { steam_path }) => {
            handle_uninstall(steam_path)?;
            Ok(ExitCode::SUCCESS)
        }

        Some(Commands::Search { query, limit }) => {
            handle_search(query, limit)?;
            Ok(ExitCode::SUCCESS)
        }

        Some(Commands::Config { action }) => {
            handle_config(action)?;
            Ok(ExitCode::SUCCESS)
        }

        Some(Commands::Compat { verb, args }) => {
            handle_compat(verb, args)
        }

        None => {
            // No subcommand - check if args were passed (legacy mode)
            if !cli.args.is_empty() {
                // Legacy mode: treat args as a command to run
                handle_run(None, cli.args, cli.config)?;
                Ok(ExitCode::SUCCESS)
            } else {
                // No args either - print help
                use clap::CommandFactory;
                Cli::command().print_help()?;
                println!();
                Ok(ExitCode::SUCCESS)
            }
        }
    }
}
