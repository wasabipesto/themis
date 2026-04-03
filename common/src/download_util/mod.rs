//! A couple of utilities for downloading from platform APIs

use anyhow::{Context, Result, anyhow, bail};
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
/// Standard version with no authentication header.
///
/// # Errors
/// Returns an error if:
/// - A TLS backend cannot be initialized
/// - The resolver cannot load the system configuration
pub fn get_reqwest_client_ratelimited(
    request_count: usize,
    interval_ms: u64,
) -> Result<ClientWithMiddleware> {
    get_reqwest_client_ratelimited_with_auth(request_count, interval_ms, None)
}

/// A default API client with middleware to rate limit and retry on failure.
/// Optionally includes an Authorization header.
///
/// # Errors
/// Returns an error if:
/// - Header value contains invalid characters
/// - A TLS backend cannot be initialized
/// - The resolver cannot load the system configuration
pub fn get_reqwest_client_ratelimited_with_auth(
    request_count: usize,
    interval_ms: u64,
    auth_header: Option<String>,
) -> Result<ClientWithMiddleware> {
    trace!("Building new rate-limited client: {request_count} requests per {interval_ms} ms");

    // Convert `interval_ms` to a duration
    let interval_duration = std::time::Duration::from_millis(interval_ms);

    // Set up an exponential backoff timer for requests that get server errors
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);

    // Set up a leaky bucket rate limiter with `request_count` requests per interval
    let rate_limiter = RateLimiter::builder()
        .initial(request_count) // Start with n items in the bucket
        .refill(request_count) // Add n items every interval
        .max(request_count) // Maximum of n items in the bucket
        .interval(interval_duration)
        .build();

    // Build the client with headers if auth is provided
    // Note: We cold set the auth header as sensitive but I'll take the tradeoff for easier debugging
    let client = if let Some(auth_value) = auth_header {
        let mut headers = HeaderMap::new();
        let header_value = HeaderValue::from_str(&auth_value)
            .context("Failed to create header value from auth string")?;
        headers.insert(AUTHORIZATION, header_value);

        trace!("Building client with Authorization header");
        reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to build reqwest client with headers")?
    } else {
        // Otherwise, just create a default client
        reqwest::Client::new()
    };

    // Build the client and attach all of our middleware
    Ok(ClientBuilder::new(client)
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .with(reqwest_leaky_bucket::rate_limit_all(rate_limiter))
        .build())
}

/// Standard method for sending a request and returning the output as a JSON value.
///
/// # Errors
/// Returns an error if:
/// - The request cannot be cloned (such as a stream).
/// - There is an error while sending request.
/// - A redirect loop is detected or redirect limit is exhausted.
/// - The response text cannot be decoded or deserialized as JSON.
/// - The server returns a non-200 status code.
pub async fn send_request(req: reqwest_middleware::RequestBuilder) -> Result<Value> {
    // Save the URL for diagnostics in case this fails
    let cloned_req = req // I'm not 100% sure why we have to clone this
        .try_clone() // This can fail if the request is a stream, but we don't use those
        .context("Failed to clone request builder.")?
        .build()
        .context("Failed to build request.")?;
    let final_url = cloned_req.url();
    trace!("Sending new request to: {final_url}");

    // Send the request
    // Automatic rate-limiting and exponential back-offs are applied here
    let response = req
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to send request: {e}"))?;

    // Parse the response as text
    let status = response.status();
    let response_text = response
        .text()
        .await
        .context("Failed to get response body text.")?;

    // Check if the server returned an error
    if !status.is_success() {
        bail!("Query to {final_url} returned {status}: {response_text}.");
    }

    // Parse the response body text as JSON
    let value =
        serde_json::from_str(&response_text).context("Failed to deserialize string as JSON.")?;
    trace!("Successfully processed response from: {final_url}");
    Ok(value)
}

/// Back up the existing file (if existing) by renaming it, appending ".bak".
///
/// # Errors
/// Returns an error if `fs::rename()` fails.
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
///
/// # Errors
/// - Returns Ok(None) if we should re-download (deserializing error, or file did not exist).
/// - Returns Err if we should halt (file system errors).
pub fn load_index_from_file(index_file_path: &PathBuf) -> Result<Option<Vec<IndexItem>>> {
    if index_file_path.exists() {
        // Attempt to load the index file
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

        // Check the contents, re-download if empty
        if index.is_empty() {
            // If the index exists but it's empty something must have gone wrong
            warn!(
                "Index file {} exists but is empty, overriding it.",
                index_file_path.display(),
            );
            Ok(None)
        } else {
            // The index loaded with some valid JSON, assume it's complete
            Ok(Some(index))
        }
    } else {
        // Touch a new index file and make sure it was created properly
        File::create(index_file_path).map_err(|e| {
            anyhow!(
                "Could not create new data file {}: {e}",
                index_file_path.display()
            )
        })?;
        // Index is not valid because it was just created
        trace!("Created new index file {}", index_file_path.display());
        Ok(None)
    }
}

/// Loads each line in the data file at the specified file path.
/// If the file does not exist, creates it.
/// Reads and deserializes each line as JSON, then grabs the ID and saves it.
///
/// # Errors
/// Returns an error if `BufRead.lines()` or `File::create()` fails.
pub fn load_data_ids(data_file_path: &PathBuf) -> Result<HashSet<String>> {
    // Build a HashSet for the output
    let mut data_ids = HashSet::new();

    if data_file_path.exists() {
        // Open the data file
        let file = File::open(data_file_path).map_err(|e| {
            anyhow!(
                "Failed to open data file {}: {}",
                data_file_path.display(),
                e
            )
        })?;

        // Start reading line by line
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line.map_err(|e| {
                anyhow!(
                    "Failed to read line from {}: {}",
                    data_file_path.display(),
                    e
                )
            })?;

            // Deserialize into JSON
            match serde_json::from_str::<Value>(&line) {
                Ok(value) => {
                    // Get the ID as a string
                    match get_id(&value) {
                        Ok(id) => {
                            // `.insert()` adds to the set and returns false if it already contained the value
                            if !data_ids.insert(id.clone()) {
                                warn!("Duplicate data file ID: {id}");
                            }
                        }
                        Err(e) => {
                            // It must be valid JSON but we couldn't get the ID
                            error!("Failed to get ID from JSON {value}: {e}",);
                            return Err(anyhow!("Failed to get ID from JSON {value}: {e}"));
                        }
                    }
                }
                Err(e) => {
                    // Invalid JSON on this line
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
        // Touch a new index file and make sure it was created properly
        File::create(data_file_path).map_err(|e| {
            anyhow!(
                "Could not create new data file {}: {e}",
                data_file_path.display()
            )
        })?;
        trace!("Created new data file {}", data_file_path.display());
    }
    Ok(data_ids)
}

/// Reads a specific `IndexItem` from the index file by ID.
/// This is slow but it allows us to avoid keeping the entire index in memory.
///
/// # Errors
/// Returns an error if:
/// - The file cannot be opened
/// - The buffer reader cannot get a line
/// - The line cannot be parsed as an `IndexItem`
/// - The ID `target_id` cannot be found in the file
pub fn read_index_item_from_file(index_file_path: &Path, target_id: &str) -> Result<IndexItem> {
    // Open the file
    let file = File::open(index_file_path).map_err(|e| {
        anyhow!(
            "Failed to open index file {}: {e}",
            index_file_path.display()
        )
    })?;

    // Set up a new buffer reader
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.map_err(|e| {
            anyhow!(
                "Failed to read line from {}: {e}",
                index_file_path.display()
            )
        })?;

        // Deserialize the line as an `IndexItem`
        let item: IndexItem = serde_json::from_str(&line).map_err(|e| {
            anyhow!(
                "Failed to deserialize IndexItem from {}: {e}",
                index_file_path.display()
            )
        })?;

        // Check if this is the item we're looking for
        if item.id == target_id {
            return Ok(item);
        }
    }

    Err(anyhow!(
        "Item with ID '{target_id}' not found in index file {}",
        index_file_path.display()
    ))
}

/// Creates a temporary filepath for atomic writes.
/// Returns a path with .tmp extension.
#[must_use]
pub fn get_temp_file_path(file_path: &Path) -> PathBuf {
    trace!("Getting a temp file path in {}", file_path.display());
    let mut temp_path = file_path.to_path_buf();
    let current_extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let new_extension = if current_extension.is_empty() {
        "tmp".to_string()
    } else {
        format!("{current_extension}.tmp")
    };
    temp_path.set_extension(new_extension);
    temp_path
}

/// Atomically moves a temporary file to its final location.
///
/// # Errors
/// Returns an error if `fs::rename()` fails.
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

/// Get the ID from JSON object
///
/// # Errors
/// Returns an Error if:
/// - The JSON value cannot be parsed as an Object (Dict, Map, etc.)
/// - The JSON object does not include the key "id"
/// - The value associated with the "id" key cannot be converted to a string
pub fn get_id(item: &Value) -> Result<String> {
    // Convert the JSON value to an object and perform the lookup
    let id_value = item
        .as_object()
        .context("Failed to parse JSON value as object")?
        .get("id")
        .with_context(|| format!("Key 'id' not found in JSON object {item:?}"))?;

    // Convert the ID to a string if necessary
    // TODO: Use a match here to cover all cases
    if let Some(id_str) = id_value.as_str() {
        Ok(id_str.to_owned())
    } else if let Some(id_num) = id_value.as_i64() {
        Ok(id_num.to_string())
    } else if let Some(id_num) = id_value.as_f64() {
        Ok(id_num.to_string())
    } else if let Some(id_num) = id_value.as_number() {
        Ok(id_num.to_string())
    } else {
        Err(anyhow!(
            "Value associated with 'id' is neither a string nor a number: {id_value:?}"
        ))
    }
}

/// Formats a `Duration` into a human-readable string with
fn pretty_print_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        let minutes = secs / 60;
        let seconds = secs % 60;
        format!("{minutes}m {seconds}s")
    } else {
        let hours = secs / 3600;
        let minutes = (secs % 3600) / 60;
        format!("{hours}h {minutes}m")
    }
}

/// Display download progress and estimated time to complete after every n items.
pub fn pretty_print_download_progress(
    platform: &Platform,
    completed: usize,
    download_count: usize,
    start_time: &Instant,
) {
    // Only output progress every n items, this is lower for platforms that process more slowly
    let n = match platform {
        Platform::Kalshi => 1000,
        Platform::Manifold => 500,
        Platform::Metaculus => 15,
        Platform::Polymarket => 250,
    };
    if completed.is_multiple_of(n) {
        // Get the total time elapsed since the start of the download
        let elapsed_time = start_time.elapsed();

        // Estimate total time and remaining time
        #[allow(clippy::cast_precision_loss)]
        let percent_completed = completed as f64 / download_count as f64 * 100.0;
        let percent_completed_inverted = 100.0 / percent_completed;
        let estimated_total_time = elapsed_time.mul_f64(percent_completed_inverted);
        let remaining_time = estimated_total_time.saturating_sub(elapsed_time);

        info!(
            "{platform}: Processed {completed}/{download_count} items ({percent_completed:.1}%). Elapsed: {}, Remaining: {}",
            pretty_print_duration(elapsed_time),
            pretty_print_duration(remaining_time),
        );
    }
}
