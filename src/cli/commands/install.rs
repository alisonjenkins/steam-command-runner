use crate::compat_tool::install::install_compat_tool;
use crate::error::AppError;
use std::path::PathBuf;
use tracing::info;

/// Handle the install command - install as a Steam compatibility tool
pub fn handle_install(
    name: Option<String>,
    steam_path: Option<PathBuf>,
    require_proton: Option<String>,
) -> Result<(), AppError> {
    let tool_name = name.unwrap_or_else(|| "steam-command-runner".to_string());
    info!("Installing compatibility tool: {}", tool_name);

    install_compat_tool(&tool_name, steam_path, require_proton)?;

    println!("Successfully installed '{}' as a Steam compatibility tool.", tool_name);
    println!("Restart Steam and select it in game properties under Compatibility.");

    Ok(())
}
