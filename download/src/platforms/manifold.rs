//! Tools to download and process markets from the Manifold API.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use log::{debug, error, trace, warn};
use reqwest_middleware::ClientWithMiddleware;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serde_jsonlines::append_json_lines;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use super::{IndexItem, Platform};
use crate::util::{display_progress, get_id, get_reqwest_client_ratelimited, send_request};

const MANIFOLD_API_BASE: &str = "https://api.manifold.markets/v0";
const MANIFOLD_RATELIMIT: usize = 15;
const MANIFOLD_RATELIMIT_MS: u64 = 1000;

/// Format of data saved to JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifoldItem {
    id: String,
    last_updated: DateTime<Utc>,
    lite_market: Value,
    full_market: Value,
    bets: Vec<Value>,
}

/// Download extended data from the `/market/{id}` endpoint.
/// Detect errors and warn but don't stop processing.
async fn get_full_market(client: &ClientWithMiddleware, id: &str) -> Result<Value> {
    trace!("Getting Manifold extended market data for Market {id}");
    let api_url = MANIFOLD_API_BASE.to_owned() + "/market/" + id;

    // submit the request
    send_request(client.get(&api_url)).await.or_else(|err| {
        warn!("Failed to fetch extended market data for Market {id}: {err}");
        trace!("Returning null JSON object instead.");
        Ok(json!(null))
    })
}

/// Download extended data from the `/bets` endpoint.
/// Detect errors and warn but don't stop processing.
async fn get_bet_data(client: &ClientWithMiddleware, market_id: &str) -> Result<Vec<Value>> {
    trace!("Getting Manifold bet data for Market {market_id}");
    let api_url = MANIFOLD_API_BASE.to_owned() + "/bets";
    let limit = 1000;
    let mut before: Option<String> = None;
    let mut bets: Vec<Value> = Vec::new();

    // loop until all bets are downloaded
    loop {
        // send the request
        let response = match send_request(
            client
                .get(&api_url)
                .query(&[("contractId", market_id)])
                .query(&[("limit", &limit)])
                .query(&[("before", &before)]), // if value is None, param is not sent
        )
        .await
        {
            // if the response came through with no errors, pass along
            Ok(resp) => resp,
            // otherwise, pass an empty vec so we don't break processing
            Err(err) => {
                warn!("Failed to fetch bet data for Market {market_id}: {err}");
                trace!("Returning null JSON object instead.");
                return Ok(vec![json!(null)]);
            }
        };

        // format as an array
        let bet_arr = response
            .as_array()
            .ok_or_else(|| {
                anyhow!(
                    "Could not format API response as array. Response: {:?}",
                    response
                )
            })?
            .to_owned();

        // check the length of the returned array
        // if the length is less than the limit, we've reached the end of the bets
        if bet_arr.len() == limit {
            // update the cursor
            let last_bet = bet_arr
                .last()
                .ok_or_else(|| anyhow!("Bet batch missing items!"))?;
            let last_id = get_id(last_bet)?;
            before = Some(last_id);
            // save the bets
            bets.extend(bet_arr);
        } else {
            // save the bets
            bets.extend(bet_arr);
            // break out
            break;
        }
    }
    trace!(
        "Downloaded {} bet items for market {}",
        bets.len(),
        market_id
    );
    Ok(bets)
}

/// Downloads everything to build a market item.
async fn get_data_and_build_item(
    client: &ClientWithMiddleware,
    cache: &HashMap<String, IndexItem>,
    id: &str,
) -> Result<ManifoldItem> {
    // return the row ready for writing
    Ok(ManifoldItem {
        id: id.to_owned(),
        last_updated: Utc::now(),
        lite_market: cache
            .get(id)
            .ok_or_else(|| anyhow!("Cache missing market key {id}!"))?
            .data
            .clone(),
        full_market: get_full_market(client, id).await?,
        bets: get_bet_data(client, id).await?,
    })
}

/// Downloads and returns a new index.
pub async fn download_index() -> Result<Vec<IndexItem>> {
    // set platform
    let platform = Platform::Manifold;

    // get url and client
    let api_url = MANIFOLD_API_BASE.to_owned() + "/markets";
    let client = get_reqwest_client_ratelimited(MANIFOLD_RATELIMIT, MANIFOLD_RATELIMIT_MS);

    // loop through questions endpoint until all are downloaded
    let limit = 1000;
    let mut index = Vec::new();
    let mut before: Option<String> = None;
    loop {
        let response = send_request(
            client
                .get(&api_url)
                .query(&[("limit", limit)])
                .query(&[("before", before)]), // if value is None, param is not sent
        )
        .await?;

        let batch = response
            .as_array()
            .map(|response_array| response_array.to_owned())
            .ok_or_else(|| anyhow!("Could not format API reponse as array {}", response))?;

        // add batch to cache
        for question in batch.clone() {
            let question_id = get_id(&question)?;
            let item = IndexItem {
                id: question_id.clone(),
                last_updated: Utc::now(),
                data: question,
            };
            index.push(item);
        }

        // update the cursor or break
        if batch.len() == limit {
            let cursor_some = batch
                .last()
                .map(get_id)
                .transpose()
                .map_err(|e| anyhow!("Failed to get ID for the last batch item: {e}"))?
                .ok_or_else(|| anyhow!("Batch is empty!"))?;
            debug!(
                "Got {} items and new {platform} cursor: {cursor_some}",
                batch.len()
            );
            before = Some(cursor_some);
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
    let platform = Platform::Manifold;
    let client = get_reqwest_client_ratelimited(MANIFOLD_RATELIMIT, MANIFOLD_RATELIMIT_MS);

    // Set progress counters
    let start_time = Instant::now();
    let download_count = ids_to_download.len();
    let mut completed: usize = 0;

    // Process in batches of 10
    for batch in ids_to_download.chunks(10) {
        let futures = batch
            .iter()
            .map(|id| get_data_and_build_item(&client, &index, id));

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
