mod native;
mod proton;

use crate::config::{ExecutionMode, MergedConfig};
use crate::error::AppError;
use std::process::ExitCode;
use tracing::{debug, info};

pub use native::NativeRunner;
pub use proton::ProtonRunner;

/// Execute a game with the given configuration
pub fn execute_game(config: &MergedConfig, command: Vec<String>) -> Result<ExitCode, AppError> {
    if command.is_empty() {
        return Err(AppError::NoCommand);
    }

    let game_path = &command[0];

    // Determine execution mode
    let mode = match config.mode {
        ExecutionMode::Auto => detect_execution_mode(game_path),
        m => m,
    };

    info!("Execution mode: {:?}", mode);

    match mode {
        ExecutionMode::Native | ExecutionMode::Auto => {
            let runner = NativeRunner::new(config);
            runner.run(command)
        }
        ExecutionMode::Proton => {
            let runner = ProtonRunner::new(config)?;
            runner.run(command)
        }
    }
}

/// Detect execution mode based on file extension
fn detect_execution_mode(path: &str) -> ExecutionMode {
    let path_lower = path.to_lowercase();
    if path_lower.ends_with(".exe") || path_lower.ends_with(".msi") || path_lower.ends_with(".bat") {
        debug!("Detected Windows executable, using Proton mode");
        ExecutionMode::Proton
    } else {
        debug!("Detected native executable, using Native mode");
        ExecutionMode::Native
    }
}
