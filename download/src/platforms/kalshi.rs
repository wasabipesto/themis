//! Tools to download and process markets from the Kalshi API.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use log::{debug, error, trace, warn};
use reqwest_middleware::ClientWithMiddleware;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_jsonlines::append_json_lines;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use super::{IndexItem, Platform};
use crate::util::{display_progress, get_reqwest_client_ratelimited, send_request};

const KALSHI_API_BASE: &str = "https://api.elections.kalshi.com/trade-api/v2";
const KALSHI_RATELIMIT: usize = 10;
const KALSHI_RATELIMIT_MS: u64 = 1000;

/// Format of data saved to JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KalshiItem {
    id: String,
    last_updated: DateTime<Utc>,
    market: Value,
    event: Value,
    series: Value,
    history: Vec<Value>,
}

/// Downloads the event data for this market.
/// An event usually contains multiple markets and has more information.
async fn get_event(client: &ClientWithMiddleware, market: &Value) -> Result<Value> {
    let event_ticker = market
        .get("event_ticker")
        .context("Expected 'event_ticker' field in market.")?
        .as_str()
        .context("Failed to interpret 'event_ticker' as string.")?;
    let api_url = format!("{KALSHI_API_BASE}/events/{event_ticker}");
    let response = send_request(client.get(&api_url)).await?;
    response
        .get("event")
        .context("Expected 'event' field in /events response.")
        .cloned()
}

/// Downloads the series data for this market.
/// A series usually contains multiple events and has more information.
/// You can usually guess the series ticker from the market ticker but there are a lot of gotchas.
/// Instead, we get the series ticker from the event data, which we get from the event ticker from the market.
async fn get_series(client: &ClientWithMiddleware, event: &Value) -> Result<Value> {
    let series_ticker = event
        .get("series_ticker")
        .context("Expected 'series_ticker' field in event.")?
        .as_str()
        .context("Failed to interpret 'series_ticker' as string.")?;
    let api_url = format!("{KALSHI_API_BASE}/series/{series_ticker}");
    let response = send_request(client.get(&api_url)).await?;
    response
        .get("series")
        .context("Expected 'series' field in /series response.")
        .cloned()
}

/// Download extended data from the `/markets/trades` endpoint.
/// Detect errors and warn but don't stop processing.
async fn get_trades(
    client: &ClientWithMiddleware,
    market: &Value,
    ticker: &str,
) -> Result<Vec<Value>> {
    // Kalshi has a *lot* of markets and the primary bottleneck to this
    // download is getting trade history. Most don't have any trade volume
    // so to avoid sending pointless requests that will return 0 trades,
    // we skip the request if the volume is 0.
    let volume = market
        .get("volume")
        .context("Expected 'volume' field in market.")?
        .as_u64()
        .context("Failed to interpret 'volume' as u64.")?;
    if volume == 0 {
        trace!("Kalshi volume is 0, skipping /trades request.");
        return Ok(Vec::new());
    }

    // prep for requests
    let api_url = KALSHI_API_BASE.to_owned() + "/markets/trades";
    let limit: usize = 1000;

    // loop until we have all history items
    let mut cursor: Option<String> = None;
    let mut all_trades = Vec::new();
    loop {
        let response = send_request(
            client
                .get(&api_url)
                .query(&[("limit", limit)])
                .query(&[("ticker", ticker)])
                .query(&[("cursor", cursor.clone())])
                .query(&[("min_ts", 0)]),
        )
        .await?;

        // get history array and save
        let trades = response
            .get("trades")
            .context("Expected 'trades' field in response.")?
            .as_array()
            .context("Failed to interpret 'trades' as array.")?
            .to_owned();
        all_trades.extend(trades.clone());

        // warn if there seems like too many trades
        // I had an issue once where it just kept going until it OOM'd
        if all_trades.len() > 500_000 && all_trades.len() % (limit * 10) == 0 {
            warn!(
                "Kalshi market {ticker} has accumulated {} trades, something may be wrong. Curent cursor: {}. Last trade: {:?}",
                all_trades.len(),
                cursor.unwrap(),
                all_trades.last().unwrap()
            );
        }

        // update the cursor or break
        if trades.len() == limit {
            let cursor_some = response
                .get("cursor")
                .context("Expected 'cursor' field in response.")?
                .as_str()
                .context("Failed to interpret 'cursor' as string.")?
                .to_owned();
            trace!(
                "Got {} items and new Kalshi history cursor: {cursor_some}",
                trades.len()
            );
            if cursor_some.is_empty() {
                debug!("Market returned {limit} trades but cursor was empty. Exiting.");
                break;
            }
            cursor = Some(cursor_some);
        } else {
            trace!(
                "Batch size {} was smaller than limit {}, we must be done here.",
                trades.len(),
                limit
            );
            break;
        }
    }

    Ok(all_trades)
}

/// Downloads everything to build a market item.
async fn get_data_and_build_item(
    client: &ClientWithMiddleware,
    index: &HashMap<String, IndexItem>,
    ticker: &str,
) -> Result<KalshiItem> {
    // get market from index
    let market = index
        .get(ticker)
        .ok_or_else(|| anyhow!("Index missing market key {ticker}!"))?
        .data
        .clone();
    // get event data...
    let event = get_event(client, &market).await?;
    // and series data...
    let series = get_series(client, &event).await?;
    // return the row ready for writing
    Ok(KalshiItem {
        id: ticker.to_owned(),
        last_updated: Utc::now(),
        market: market.clone(),
        event,
        series,
        history: get_trades(client, &market, ticker).await?,
    })
}

/// Downloads and returns a new index.
pub async fn download_index() -> Result<Vec<IndexItem>> {
    // set platform
    let platform = Platform::Kalshi;

    // get url, client, login token
    let api_url = KALSHI_API_BASE.to_owned() + "/markets";
    let client = get_reqwest_client_ratelimited(KALSHI_RATELIMIT, KALSHI_RATELIMIT_MS);

    // loop through questions endpoint until all are downloaded
    let limit = 1000;
    let mut index = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let response = send_request(
            client
                .get(&api_url)
                .query(&[("limit", limit)])
                .query(&[("cursor", cursor.clone())]),
        )
        .await?;
        let batch = response
            .get("markets")
            .context("Expected 'markets' field in response.")?
            .as_array()
            .context("Failed to interpret 'markets' as array.")?
            .to_owned();

        // add batch to index
        for market in batch.clone() {
            let market_ticker = market
                .get("ticker")
                .context("Expected 'ticker' field in response.")?
                .as_str()
                .context("Failed to interpret 'ticker' as string.")?
                .to_owned();
            let item = IndexItem {
                id: market_ticker.clone(),
                last_updated: Utc::now(),
                data: market,
            };
            index.push(item);
        }

        // update the cursor or break
        if batch.len() == limit {
            let cursor_some = response
                .get("cursor")
                .context("Expected 'cursor' field in response.")?
                .as_str()
                .context("Failed to interpret 'cursor' as string.")?
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
    let platform = Platform::Kalshi;
    let client = get_reqwest_client_ratelimited(KALSHI_RATELIMIT, KALSHI_RATELIMIT_MS);

    // Set progress counters
    let start_time = Instant::now();
    let download_count = ids_to_download.len();
    let mut completed: usize = 0;

    // Process in batches of 10
    for batch in ids_to_download.chunks(10) {
        let futures = batch
            .iter()
            .map(|ticker| get_data_and_build_item(&client, &index, ticker));

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
