//! Tools to download and process markets from the Metaculus API.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use log::{debug, trace, warn};
use reqwest_middleware::ClientWithMiddleware;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serde_jsonlines::append_json_lines;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use super::{IndexItem, Platform};
use crate::util::{display_progress, get_id, get_reqwest_client_ratelimited, send_request};

const METACULUS_API_BASE: &str = "https://www.metaculus.com/api";
const METACULUS_RATELIMIT: usize = 8;
const METACULUS_RATELIMIT_MS: u64 = 30_000;

/// Format of data saved to JSON for extended data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaculusItem {
    id: String,
    last_updated: DateTime<Utc>,
    post: Value,
    details: Value,
}

/// Download extended data from the `/posts/{id}/` endpoint.
/// Detect errors and warn but don't stop processing.
async fn get_extended_data(client: &ClientWithMiddleware, id: &str) -> Result<Value> {
    trace!("Getting Metaculus extended data for Question {id}");
    let api_url = METACULUS_API_BASE.to_owned() + "/posts/" + id + "/";

    // submit the request
    send_request(client.get(&api_url)).await.or_else(|err| {
        warn!("Failed to fetch extended data for Question {id}: {err}");
        trace!("Returning null JSON object instead.");
        Ok(json!(null))
    })
}

/// Downloads and returns a new index.
pub async fn download_index() -> Result<Vec<IndexItem>> {
    // set platform
    let platform = Platform::Metaculus;

    // get client
    let api_url = METACULUS_API_BASE.to_owned() + "/posts/";
    let client = get_reqwest_client_ratelimited(METACULUS_RATELIMIT, METACULUS_RATELIMIT_MS);

    // loop through questions endpoint until all are downloaded
    let limit = 100;
    let mut index = Vec::new();
    let mut offset: usize = 0;
    loop {
        // submit the request
        let response = send_request(
            client
                // Options: https://www.metaculus.com/api/
                .get(&api_url)
                .query(&[("limit", limit)])
                .query(&[("offset", offset)])
                // Required. Currently we only need resolved but I'd like to expand this.
                .query(&[("statuses", "resolved")])
                // Required. Wish there was an "all" option so we don't miss new types.
                .query(&[("forecast_type", "binary")])
                .query(&[("forecast_type", "numeric")])
                .query(&[("forecast_type", "date")])
                .query(&[("forecast_type", "multiple_choice")])
                .query(&[("forecast_type", "conditional")])
                .query(&[("forecast_type", "group_of_questions")])
                // Whether or not to return community predictions.
                // Even if true, does not return all series! Get those in step 2.
                .query(&[("with_cp", false)])
                // How to order the results.
                .query(&[("order_by", "published_at")]),
        )
        .await?;

        // check the results
        let batch = match response.get("results") {
            Some(results) => {
                results.as_array()
                    .map(|results_array| results_array.to_owned())
                    .ok_or_else(|| anyhow!("Metaculus API Error: 'results' is not an array at offset {offset}"))
            },
            None => Err(anyhow!("Metaculus API Error: No 'results' key in response from url {api_url} at offset {offset}")),
        }?;

        // break if the batch returns no items
        if batch.is_empty() {
            trace!("No items in batch, breaking from download loop.");
            break;
        }

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

        // update the cursor
        if batch.len() == limit {
            offset += batch.len();
            debug!(
                "Got {} items and new {platform} cursor: {offset}",
                batch.len()
            );
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
    // get client
    let platform = Platform::Metaculus;
    let client = get_reqwest_client_ratelimited(METACULUS_RATELIMIT, METACULUS_RATELIMIT_MS);

    // Set progress counters
    let start_time = Instant::now();
    let download_count = ids_to_download.len();
    let mut completed: usize = 0;

    // could paralleize this but the rate limit is so low that it doesn't have any benefit
    for id in ids_to_download.iter() {
        // download extended data
        let details = get_extended_data(&client, id).await?;

        // append row to data json file
        let line = json!(MetaculusItem {
            id: id.clone(),
            last_updated: Utc::now(),
            post: index
                .get(id)
                .ok_or_else(|| anyhow!("Cache missing key!"))?
                .data
                .clone(),
            details,
        });
        append_json_lines(data_file_path, [line])?;
        trace!("Successfully appended 1 item to file.");

        // Calculate progress and elapsed time every n items
        completed += 1;
        display_progress(&platform, completed, download_count, &start_time);
    }
    Ok(())
}
