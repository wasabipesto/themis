//! Tools to download and process markets from the Metaculus API.

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use log::{debug, trace, warn};
use reqwest_middleware::ClientWithMiddleware;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use serde_jsonlines::append_json_lines;

use std::env;
use std::path::Path;
use std::time::Instant;

use super::{IndexItem, Platform};
use crate::download_util::{
    display_progress, finalize_temp_file, get_id, get_reqwest_client_ratelimited_with_auth,
    get_temp_file_path, read_index_item_from_file, send_request,
};

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

/// Downloads index and streams it directly to disk.
pub async fn download_index(index_file_path: &Path) -> Result<()> {
    // set platform
    let platform = Platform::Metaculus;

    // get API key from environment
    let api_key =
        env::var("METACULUS_API_KEY").expect("METACULUS_API_KEY environment variable is required");
    let auth_header = format!("Token {}", api_key);

    // get client
    let api_url = METACULUS_API_BASE.to_owned() + "/posts/";
    let client = get_reqwest_client_ratelimited_with_auth(
        METACULUS_RATELIMIT,
        METACULUS_RATELIMIT_MS,
        Some(auth_header),
    );

    // write to temporary file first for atomic operation
    let temp_file_path = get_temp_file_path(index_file_path);
    debug!(
        "{platform}: Writing index to temp file: {}",
        temp_file_path.display()
    );

    // loop through questions endpoint until all are downloaded
    let limit = 100;
    let mut total_items = 0;
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
            Some(results) => results
                .as_array()
                .map(|results_array| results_array.to_owned())
                .ok_or_else(|| {
                    anyhow!("Metaculus API Error: 'results' is not an array at offset {offset}")
                }),
            None => Err(anyhow!(
                "Metaculus API Error: No 'results' key in response from url {api_url} at offset {offset}"
            )),
        }?;

        // break if the batch returns no items
        if batch.is_empty() {
            trace!("No items in batch, breaking from download loop.");
            break;
        }

        // build items from batch
        let mut items = Vec::with_capacity(batch.len());
        for question in batch.clone() {
            let question_id = get_id(&question)?;
            let item = IndexItem {
                id: question_id.clone(),
                last_updated: Utc::now(),
                data: question,
            };
            items.push(item);
        }

        // immediately write batch to temp file
        append_json_lines(&temp_file_path, items)?;
        total_items += batch.len();
        trace!(
            "{platform}: Wrote {} items to temp file (total: {})",
            batch.len(),
            total_items
        );

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

    // atomically move temp file to final location
    finalize_temp_file(&temp_file_path, index_file_path)?;
    debug!("{platform}: Index download complete with {total_items} total items");
    Ok(())
}

/// Downloads extended data for all markets that haven't been downloaded.
/// Appends directly into data file.
pub async fn download_data(
    index_file_path: &Path,
    ids_to_download: &[String],
    data_file_path: &Path,
) -> Result<()> {
    // get client
    let platform = Platform::Metaculus;

    // get API key from environment
    let api_key =
        env::var("METACULUS_API_KEY").expect("METACULUS_API_KEY environment variable is required");
    let auth_header = format!("Token {}", api_key);

    let client = get_reqwest_client_ratelimited_with_auth(
        METACULUS_RATELIMIT,
        METACULUS_RATELIMIT_MS,
        Some(auth_header),
    );

    // Set progress counters
    let start_time = Instant::now();
    let download_count = ids_to_download.len();
    let mut completed: usize = 0;

    // could paralleize this but the rate limit is so low that it doesn't have any benefit
    for id in ids_to_download.iter() {
        // download extended data
        let details = get_extended_data(&client, id).await?;

        // get post from index file
        let index_item = read_index_item_from_file(index_file_path, id)?;

        // append row to data json file
        let line = json!(MetaculusItem {
            id: id.clone(),
            last_updated: Utc::now(),
            post: index_item.data.clone(),
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
