use crate::cli::ProtonAction;
use crate::error::AppError;
use crate::proton::list_proton_versions;

/// Handle the proton command and its subcommands
pub fn handle_proton(action: ProtonAction) -> Result<(), AppError> {
    match action {
        ProtonAction::List { paths } => list_versions(paths),
    }
}

fn list_versions(show_paths: bool) -> Result<(), AppError> {
    let versions = list_proton_versions();

    if versions.is_empty() {
        println!("No Proton versions found.");
        println!("\nSearched locations:");
        println!("  ~/.steam/root/compatibilitytools.d/");
        println!("  ~/.local/share/Steam/compatibilitytools.d/");
        println!("  Steam library paths");
        return Ok(());
    }

    println!("Available Proton versions:\n");

    if show_paths {
        for (name, path) in versions {
            println!("  {}  {}", name, path.display());
        }
    } else {
        for (name, _) in versions {
            println!("  {}", name);
        }
    }

    Ok(())
}
