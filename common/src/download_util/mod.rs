//! A couple of utilities for downloading from platform APIs

use anyhow::{Context, Result, anyhow};
use log::{debug, error, info, trace, warn};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use reqwest_leaky_bucket::leaky_bucket::RateLimiter;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use serde_json::Value;
use serde_jsonlines::json_lines;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::platforms::{IndexItem, Platform};

/// A default API client with middleware to rate limit and retry on failure.
/// Standard version wit no authentication header.
pub fn get_reqwest_client_ratelimited(
    request_count: usize,
    interval_ms: u64,
) -> ClientWithMiddleware {
    get_reqwest_client_ratelimited_with_auth(request_count, interval_ms, None)
}

/// A default API client with middleware to rate limit and retry on failure.
/// Optionally includes an Authorization header.
pub fn get_reqwest_client_ratelimited_with_auth(
    request_count: usize,
    interval_ms: u64,
    auth_header: Option<String>,
) -> ClientWithMiddleware {
    // convert to duration
    let interval_duration = std::time::Duration::from_millis(interval_ms);

    // log intention
    trace!(
        "Building new rate-limited client: {} requests per {} ms",
        request_count,
        interval_duration.as_millis()
    );

    // retry requests that get server errors with an exponential backoff timer
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);

    // rate limit to n requests per interval
    let rate_limiter = RateLimiter::builder()
        .initial(request_count) // start with n items in the bucket
        .refill(request_count) // add n items every interval
        .max(request_count) // maximum of n items in the bucket
        .interval(interval_duration)
        .build();

    // build default headers if auth is provided
    let client = if let Some(auth_value) = auth_header {
        let mut headers = HeaderMap::new();
        let header_value = HeaderValue::from_str(&auth_value)
            .expect("Failed to create header value from auth string");
        headers.insert(AUTHORIZATION, header_value);

        trace!("Building client with Authorization header");
        reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to build reqwest client with headers")
    } else {
        reqwest::Client::new()
    };

    ClientBuilder::new(client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .with(reqwest_leaky_bucket::rate_limit_all(rate_limiter))
        .build()
}

/// Standard method for sending a request and returning the output as a JSON value.
pub async fn send_request(req: reqwest_middleware::RequestBuilder) -> Result<Value> {
    // save the url for diagnostics in case this fails
    let cloned_req = req // not 100% sure why we have to clone this
        .try_clone() // this can fail if the request is a stream, but we don't use those
        .context("Failed to clone request builder.")?
        .build()
        .context("Failed to build request.")?;
    let final_url = cloned_req.url();
    trace!("Sending new request to: {final_url}");

    // send the request
    // automatic rate-limiting and exponential backoffs are applied here
    let response = req
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to send request: {}", e))?;

    // parse the response as text
    let status = response.status();
    let response_text = response
        .text()
        .await
        .context("Failed to get response body text.")?;

    // check if the server returned an error
    if !status.is_success() {
        return Err(anyhow!(
            "Query to {} returned {}: {}.",
            final_url,
            status,
            response_text
        ));
    }

    // parse the text as json
    let value =
        serde_json::from_str(&response_text).context("Failed to deserialize string as JSON.")?;
    trace!("Successfully processed response from: {final_url}");
    Ok(value)
}

/// Back up the existing file (if existing).
pub fn backup_file(file_path: &Path) -> Result<()> {
    if file_path.exists() {
        // Rename the file with .bak as a backup
        let backup_path = file_path.with_extension("bak");
        fs::rename(file_path, &backup_path).with_context(|| {
            format!(
                "Failed to back up the existing file. Could not rename {} to {}",
                file_path.display(),
                backup_path.display()
            )
        })?;
        debug!(
            "Backed up existing file {} to {}",
            file_path.display(),
            backup_path.display()
        );
    } else {
        debug!(
            "Requested backup of file {} but it does not exist.",
            file_path.display(),
        );
    }
    Ok(())
}

/// Loads the index from the specified file path.
/// If the file does not exist, creates it.
/// Returns Ok(Some) if the data is valid and non-empty.
/// Returns Ok(None) if we should re-download (deserializing error, or file did not exist).
/// Returns Err if we should halt (file system error).
pub fn load_index_from_file(index_file_path: &PathBuf) -> Result<Option<Vec<IndexItem>>> {
    if index_file_path.exists() {
        // attempt to load the index file
        let index = match json_lines::<IndexItem, _>(&index_file_path) {
            Ok(lines) => match lines.collect::<std::io::Result<Vec<IndexItem>>>() {
                Ok(data) => data,
                Err(e) => {
                    warn!(
                        "Failed to deserialize JSON lines from {}: {}.",
                        index_file_path.display(),
                        e
                    );
                    return Ok(None);
                }
            },
            Err(e) => {
                warn!(
                    "Failed to read JSON lines from {}: {}.",
                    index_file_path.display(),
                    e
                );
                return Ok(None);
            }
        };

        // check the contents, re-download if empty
        if index.is_empty() {
            // if the index exists but it's empty something must have gone wrong
            warn!(
                "Index file {} exists but is empty. Overriding",
                index_file_path.display(),
            );
            Ok(None)
        } else {
            // the index loaded with some valid JSON, assume it's complete
            Ok(Some(index))
        }
    } else {
        // touch a new index file and make sure it was created properly
        File::create(index_file_path).map_err(|e| {
            anyhow!(
                "Could not create new data file {}: {}",
                index_file_path.display(),
                e
            )
        })?;
        // index is not valid because it was just created
        trace!("Created new index file {}", index_file_path.display());
        Ok(None)
    }
}

/// Loads each line in the data file at the specified file path.
/// If the file does not exist, creates it.
/// Reads and deserializes each line as JSON, then grabs the ID and saves it.
pub fn load_data_ids(data_file_path: &PathBuf) -> Result<HashSet<String>> {
    // build hash set for output
    let mut data_ids = HashSet::new();

    if data_file_path.exists() {
        // open the data file
        let file = File::open(data_file_path).map_err(|e| {
            anyhow!(
                "Failed to open data file {}: {}",
                data_file_path.display(),
                e
            )
        })?;
        // start reading line by line
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line.map_err(|e| {
                anyhow!(
                    "Failed to read line from {}: {}",
                    data_file_path.display(),
                    e
                )
            })?;
            // deserialize into JSON
            match serde_json::from_str::<Value>(&line) {
                Ok(value) => {
                    // get the ID as a string
                    match get_id(&value) {
                        Ok(id) => {
                            // adds to set and returns false if it already contained the value
                            if !data_ids.insert(id.clone()) {
                                warn!("Duplicate data file ID: {id}");
                            }
                        }
                        Err(e) => {
                            // valid JSON but no ID
                            error!("Failed to get ID from JSON {value}: {e}",);
                            return Err(anyhow!("Failed to get ID from JSON {value}: {e}"));
                        }
                    }
                }
                Err(e) => {
                    // invalid JSON on this line
                    error!(
                        "Failed to deserialize JSON from {}: {e}",
                        data_file_path.display(),
                    );
                    return Err(anyhow!(
                        "Failed to deserialize JSON from {}: {e}",
                        data_file_path.display(),
                    ));
                }
            }
        }
    } else {
        // touch a new index file and make sure it was created properly
        File::create(data_file_path).map_err(|e| {
            anyhow!(
                "Could not create new data file {}: {}",
                data_file_path.display(),
                e
            )
        })?;
        trace!("Created new data file {}", data_file_path.display());
    }
    Ok(data_ids)
}

/// Reads a specific IndexItem from the index file by ID.
/// This is slow but it allows us to avoid keeping the entire index in memory.
pub fn read_index_item_from_file(index_file_path: &Path, target_id: &str) -> Result<IndexItem> {
    let file = File::open(index_file_path).map_err(|e| {
        anyhow!(
            "Failed to open index file {}: {}",
            index_file_path.display(),
            e
        )
    })?;

    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.map_err(|e| {
            anyhow!(
                "Failed to read line from {}: {}",
                index_file_path.display(),
                e
            )
        })?;

        // Deserialize the line as an IndexItem
        let item: IndexItem = serde_json::from_str(&line).map_err(|e| {
            anyhow!(
                "Failed to deserialize IndexItem from {}: {}",
                index_file_path.display(),
                e
            )
        })?;

        // Check if this is the item we're looking for
        if item.id == target_id {
            return Ok(item);
        }
    }

    Err(anyhow!(
        "Item with ID '{}' not found in index file {}",
        target_id,
        index_file_path.display()
    ))
}

/// Creates a temporary file path for atomic writes.
/// Returns a path with .tmp extension.
pub fn get_temp_file_path(file_path: &Path) -> PathBuf {
    let mut temp_path = file_path.to_path_buf();
    let current_extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let new_extension = if current_extension.is_empty() {
        "tmp".to_string()
    } else {
        format!("{}.tmp", current_extension)
    };
    temp_path.set_extension(new_extension);
    temp_path
}

/// Atomically moves a temporary file to its final location.
pub fn finalize_temp_file(temp_path: &Path, final_path: &Path) -> Result<()> {
    fs::rename(temp_path, final_path).with_context(|| {
        format!(
            "Failed to rename temp file {} to {}",
            temp_path.display(),
            final_path.display()
        )
    })?;
    debug!(
        "Successfully renamed {} to {}",
        temp_path.display(),
        final_path.display()
    );
    Ok(())
}

/// Get ID from JSON object
pub fn get_id(item: &Value) -> Result<String> {
    // convert to an object and perform the lookup
    let id_value = item
        .as_object()
        .context("Failed to parse JSON value as object")?
        .get("id")
        .with_context(|| format!("Key 'id' not found in JSON object {:?}", item))?;

    // convert to a string if necessary
    if let Some(id_str) = id_value.as_str() {
        Ok(id_str.to_owned())
    } else if let Some(id_num) = id_value.as_i64() {
        Ok(id_num.to_string())
    } else if let Some(id_num) = id_value.as_f64() {
        Ok(id_num.to_string())
    } else if let Some(id_num) = id_value.as_number() {
        Ok(id_num.to_string())
    } else {
        Err(anyhow::anyhow!(
            "Value associated with 'id' is neither a string nor a number: {:?}",
            id_value
        ))
    }
}

/// Formats a `Duration` into a human-readable string
fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        let minutes = secs / 60;
        let seconds = secs % 60;
        format!("{}m {}s", minutes, seconds)
    } else {
        let hours = secs / 3600;
        let minutes = (secs % 3600) / 60;
        format!("{}h {}m", hours, minutes)
    }
}

/// Display download progress and estimated time to complete after every n items.
pub fn display_progress(
    platform: &Platform,
    completed: usize,
    download_count: usize,
    start_time: &Instant,
) {
    let n = match platform {
        Platform::Kalshi => 1000,
        Platform::Manifold => 500,
        Platform::Metaculus => 15,
        Platform::Polymarket => 250,
    };
    if completed.is_multiple_of(n) {
        let elapsed = start_time.elapsed();

        // Estimate total time and remaining time
        let estimated_total = elapsed.mul_f64(download_count as f64 / completed as f64);
        let remaining = if estimated_total > elapsed {
            estimated_total - elapsed
        } else {
            Duration::ZERO
        };

        info!(
            "{platform}: Processed {completed}/{download_count} items ({:.1}%). Elapsed: {}, Remaining: {}",
            completed as f64 / download_count as f64 * 100.0,
            format_duration(elapsed),
            format_duration(remaining),
        );
    }
}
