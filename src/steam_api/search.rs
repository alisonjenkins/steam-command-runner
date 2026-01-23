use crate::error::AppError;
use serde::Deserialize;
use tracing::{debug, info};

/// Search for games by name and return matching App IDs
pub fn search_games(query: &str, limit: usize) -> Result<Vec<(u32, String)>, AppError> {
    info!("Searching Steam store for: {}", query);

    let results = search_steam_store(query, limit)?;

    Ok(results)
}

/// Search the Steam store for games
fn search_steam_store(query: &str, limit: usize) -> Result<Vec<(u32, String)>, AppError> {
    // Use Steam's storefront search API
    let url = format!(
        "https://store.steampowered.com/api/storesearch/?term={}&l=english&cc=US",
        urlencoding::encode(query)
    );

    debug!("Fetching: {}", url);

    let client = reqwest::blocking::Client::builder()
        .user_agent("steam-command-runner/0.2.0")
        .build()?;

    let response: StoreSearchResponse = client.get(&url).send()?.json()?;

    let results: Vec<_> = response
        .items
        .into_iter()
        .take(limit)
        .map(|item| (item.id, item.name))
        .collect();

    info!("Found {} results", results.len());
    Ok(results)
}

#[derive(Deserialize)]
struct StoreSearchResponse {
    #[serde(default)]
    items: Vec<StoreItem>,
}

#[derive(Deserialize)]
struct StoreItem {
    id: u32,
    name: String,
}

// Simple URL encoding for the query
mod urlencoding {
    pub fn encode(input: &str) -> String {
        let mut encoded = String::new();
        for byte in input.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(byte as char);
                }
                b' ' => encoded.push('+'),
                _ => {
                    encoded.push('%');
                    encoded.push_str(&format!("{:02X}", byte));
                }
            }
        }
        encoded
    }
}
