use crate::error::AppError;
use crate::steam_api::search_games;
use tracing::info;

/// Handle the search command - search for Steam App IDs by game name
pub fn handle_search(query: String, limit: usize) -> Result<(), AppError> {
    info!("Searching for: {}", query);

    let results = search_games(&query, limit)?;

    if results.is_empty() {
        println!("No games found matching '{}'", query);
        return Ok(());
    }

    println!("Found {} result(s) for '{}':\n", results.len(), query);
    for (app_id, name) in results {
        println!("  {:>8}  {}", app_id, name);
    }

    Ok(())
}
