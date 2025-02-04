//! Tools to download and process markets from the Polymarket API.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use log::{debug, error, trace};
use reqwest_middleware::ClientWithMiddleware;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_jsonlines::append_json_lines;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use super::{IndexItem, Platform};
use crate::util::{display_progress, get_reqwest_client_ratelimited, send_request};

const POLYMARKET_CLOB_API_BASE: &str = "https://clob.polymarket.com";
const POLYMARKET_RATELIMIT: usize = 15;
const POLYMARKET_RATELIMIT_MS: u64 = 1000;

/// Format of data saved to JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymarketItem {
    id: String,
    last_updated: DateTime<Utc>,
    market: Value,
    prices_history: Vec<Value>,
}

/// Get Polymarket's CLOB ID, which is not a top-level item.
/// This is sometimes null, so we can't use it as the question ID.
fn get_clob_id(item: &Value) -> Result<String> {
    let tokens = item
        .get("tokens")
        .with_context(|| format!("Market missing 'tokens' field: {:?}", item))?
        .as_array()
        .context("Expected 'tokens' field to be an array")?;

    // TODO: get the token that resolved true (winner), not the first one
    // also make sure there can only be one winning token
    // I guess just don't get any price history for still-active markets?
    let first_token = tokens
        .first()
        .context("No tokens found in 'tokens' array")?;

    let token_id = first_token
        .get("token_id")
        .with_context(|| format!("Token missing 'token_id' field: {}", first_token))?
        .as_str()
        .context("Expected 'token_id' to be a string")?;

    trace!("Got token ID: {}", token_id);
    Ok(token_id.to_owned())
}

/// Download extended data from the `/prices-history` endpoint.
/// Detect errors and warn but don't stop processing.
async fn get_prices_history(client: &ClientWithMiddleware, market: &Value) -> Result<Vec<Value>> {
    // get the CLOB ID, which is not the same as the market ID
    let clob_id = get_clob_id(market)?;
    if clob_id.is_empty() {
        // sometimes this is empty even when the market ID is not
        // the API will throw an error if we try to submit a blank market ID so we'll just skip it
        debug!("Polymarket CLOB ID is empty, skipping market.");
        return Ok(Vec::new());
    }

    let api_url = POLYMARKET_CLOB_API_BASE.to_owned() + "/prices-history";
    let mut history = Vec::new();
    let fidelity_levels = [10, 60, 180, 360, 1200, 3600];
    for fidelity in fidelity_levels {
        //trace!("Attempting to get Polymarket price history for Token ID {clob_id} at fidelity level {fidelity}");
        let response = send_request(
            client
                .get(&api_url)
                .query(&[("interval", "all")])
                .query(&[("market", &clob_id)])
                .query(&[("fidelity", fidelity)]),
        )
        .await?;
        history = response
            .get("history")
            .context("Expected 'history' field in market.")?
            .as_array()
            .context("Failed to interpret 'history' as array.")?
            .to_owned();
        if history.is_empty() {
            trace!("Polymarket price history for Token ID {clob_id} at fidelity level {fidelity} returned no items, escalating to next fidelity level.");
        } else {
            trace!("Polymarket price history for Token ID {clob_id} at fidelity level {fidelity} returned {} items, saving and escaping.", history.len());
            break;
        }
    }
    if history.is_empty() {
        debug!("Polymarket price history for Token ID {clob_id} returned no items at any fidelity level.");
    }
    // return history even if it has no items
    Ok(history)
}

/// Downloads everything to build a market item.
async fn get_data_and_build_item(
    client: &ClientWithMiddleware,
    cache: &HashMap<String, IndexItem>,
    market_id: &str,
) -> Result<PolymarketItem> {
    let market = cache
        .get(market_id)
        .ok_or_else(|| anyhow!("Cache missing market key {market_id}!"))?
        .data
        .clone();
    // return the row ready for writing
    Ok(PolymarketItem {
        id: market_id.to_owned(),
        last_updated: Utc::now(),
        market: market.clone(),
        prices_history: get_prices_history(client, &market).await?,
    })
}

/// Downloads and returns a new index.
pub async fn download_index() -> Result<Vec<IndexItem>> {
    // set platform
    let platform = Platform::Polymarket;

    // get url and client
    let api_url = POLYMARKET_CLOB_API_BASE.to_owned() + "/markets";
    let client = get_reqwest_client_ratelimited(POLYMARKET_RATELIMIT, POLYMARKET_RATELIMIT_MS);

    // loop through questions endpoint until all are downloaded
    let limit = 500;
    let mut index = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let response = send_request(
            client
                .get(&api_url)
                .query(&[("limit", limit)])
                .query(&[("next_cursor", &cursor)]), // if value is None, param is not sent
        )
        .await?;

        let batch = match response.get("data") {
            Some(results) => {
                results.as_array()
                    .map(|results_array| results_array.to_owned())
                    .ok_or_else(|| anyhow!("{platform} API Error: 'results' is not an array at offset {:?}", cursor))
            },
            None => Err(anyhow!("{platform} API Error: No 'results' key in response from url {api_url} at offset {:?}", cursor)),
        }?;

        // add batch to cache
        for market in batch.clone() {
            let id = market
                .get("question_id")
                .context("Expected 'question_id' field in market.")?
                .as_str()
                .context("Failed to interpret 'question_id' as string.")?
                .to_string();
            if id.is_empty() {
                // I don't know what's up with Polymarket but there's a bunch of these without IDs
                trace!("Market ID is an empty string. Skipping.");
                continue;
            }
            let item = IndexItem {
                id,
                last_updated: Utc::now(),
                data: market,
            };
            index.push(item);
        }

        // update the cursor or break
        if batch.len() == limit {
            let cursor_some = response
                .get("next_cursor")
                .context("Expected 'next_cursor' field in response.")?
                .as_str()
                .context("Failed to interpret 'next_cursor' as string.")?
                .to_owned();
            debug!(
                "Got {} items and new {platform} cursor: {cursor_some}",
                batch.len()
            );
            cursor = Some(cursor_some);
        } else {
            debug!(
                "Batch size {} was smaller than limit {}, we must be done here.",
                batch.len(),
                limit
            );
            break;
        }
    }
    Ok(index)
}

/// Downloads extended data for all markets that haven't been downloaded.
/// Appends directly into data file.
pub async fn download_data(
    index: HashMap<String, IndexItem>,
    ids_to_download: &[String],
    data_file_path: &Path,
) -> Result<()> {
    // Get client
    let platform = Platform::Polymarket;
    let client = get_reqwest_client_ratelimited(POLYMARKET_RATELIMIT, POLYMARKET_RATELIMIT_MS);

    // Set progress counters
    let start_time = Instant::now();
    let download_count = ids_to_download.len();
    let mut completed: usize = 0;

    // Process in batches of 10
    for batch in ids_to_download.chunks(10) {
        let futures = batch
            .iter()
            .map(|market_id| get_data_and_build_item(&client, &index, market_id));

        // Wait for all tasks in the batch to finish
        let results = futures::future::join_all(futures).await;

        // Log any errors
        let mut lines = Vec::new();
        for (id, result) in batch.iter().zip(results) {
            match result {
                Ok(item) => {
                    trace!("Item processed: {:?}", item.id);
                    lines.push(item)
                }
                Err(e) => error!("Error downloading item {id}: {e}"),
            }
        }

        // Save batch items to disk
        append_json_lines(data_file_path, lines)?;
        trace!("Successfully appended {} items to file.", batch.len());

        // Calculate progress and elapsed time every n items
        completed += batch.len();
        display_progress(&platform, completed, download_count, &start_time);
    }
    Ok(())
}
