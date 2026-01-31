//! Configuration and definitions for market platforms

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::ValueEnum;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::Path;

use crate::download_util::{backup_file, load_data_ids, load_index_from_file};

mod kalshi;
mod manifold;
mod metaculus;
mod polymarket;

/// Formatted platform name
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Default)]
pub struct PlatformName(String);

/// Slugified platform name
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Default)]
pub struct PlatformSlug(String);

/// A specific market platform that has everything implemented
#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum, PartialOrd, Serialize, Deserialize)]
pub enum Platform {
    Kalshi,
    Manifold,
    Metaculus,
    Polymarket,
}

/// Easily get a platform name for use in e.g. logs
impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Platform::Kalshi => write!(f, "Kalshi"),
            Platform::Manifold => write!(f, "Manifold"),
            Platform::Metaculus => write!(f, "Metaculus"),
            Platform::Polymarket => write!(f, "Polymarket"),
        }
    }
}

/// Easily get a platform's name in the appropriate type
impl From<Platform> for PlatformName {
    fn from(platform: Platform) -> Self {
        PlatformName(platform.to_string())
    }
}

/// Easily get a platform's slug in the appropriate type
impl From<Platform> for PlatformSlug {
    fn from(platform: Platform) -> Self {
        PlatformSlug(platform.to_string().to_lowercase())
    }
}

/// Format of data saved to JSON for basic index data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexItem {
    pub id: String,
    pub last_updated: DateTime<Utc>,
    pub data: Value,
}

/// Lightweight version of `IndexItem` that only contains minimal data needed for filtering.
#[derive(Debug, Clone)]
pub struct LightweightIndexItem {
    pub id: String,
    pub close_datetime: Option<DateTime<Utc>>,
}

impl Platform {
    /// Returns a list of all supported platform types.
    #[must_use]
    pub fn all() -> Vec<Platform> {
        vec![
            Platform::Kalshi,
            Platform::Manifold,
            Platform::Metaculus,
            Platform::Polymarket,
        ]
    }

    /// Parses just enough of an index item to get the close datetime.
    ///
    /// # Errors
    /// Returns an Error if:
    /// - The index item doesn't have the appropriate key
    ///   (`close_time`, `resolutionTime`, `actual_close_time`, or `end_date_iso`)
    /// - The value couldn't be converted to a `DateTime` with `Timestamp`
    /// - The market hasn't closed yet
    fn get_close_datetime(self, item: &IndexItem) -> Result<DateTime<Utc>> {
        let close_datetime = match self {
            Platform::Kalshi => {
                let close_time = item
                    .data
                    .get("close_time")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing or invalid close_time"))?;
                DateTime::parse_from_rfc3339(close_time)
                    .context(format!("Failed to parse Kalshi close_time {close_time}"))?
                    .with_timezone(&Utc)
            }
            Platform::Manifold => {
                let resolution_time = item
                    .data
                    .get("resolutionTime")
                    .and_then(Value::as_number)
                    .and_then(Number::as_i64)
                    .ok_or_else(|| anyhow::anyhow!("Missing or invalid resolutionTime"))?;
                DateTime::from_timestamp_millis(resolution_time)
                    .ok_or_else(|| {
                        anyhow::anyhow!("Failed to parse Manifold resolutionTime {resolution_time}")
                    })?
                    .with_timezone(&Utc)
            }
            Platform::Metaculus => {
                let actual_close_time = item
                    .data
                    .get("actual_close_time")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing or invalid actual_close_time"))?;
                DateTime::parse_from_rfc3339(actual_close_time)
                    .context(format!(
                        "Failed to parse Metaculus actual_close_time {actual_close_time}"
                    ))?
                    .with_timezone(&Utc)
            }
            Platform::Polymarket => {
                let end_date_iso = item
                    .data
                    .get("end_date_iso")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing or invalid end_date_iso"))?;
                DateTime::parse_from_rfc3339(end_date_iso)
                    .context(format!(
                        "Failed to parse Polymarket end_date_iso {end_date_iso}"
                    ))?
                    .with_timezone(&Utc)
            }
        };
        Ok(close_datetime)
    }

    /// Converts a full index into a lightweight version with minimal memory footprint.
    fn build_lightweight_index(
        self,
        index: Vec<IndexItem>,
    ) -> HashMap<String, LightweightIndexItem> {
        index
            .into_iter()
            .map(|item| {
                let id = item.id.clone();
                let close_datetime = match self.get_close_datetime(&item) {
                    Ok(dt) => Some(dt),
                    Err(e) => {
                        debug!("Failed to get close datetime for item {id}: {e}");
                        None
                    }
                };
                (id.clone(), LightweightIndexItem { id, close_datetime })
            })
            .collect()
    }

    /// Takes all items from the index and returns the IDs that need to be downloaded.
    fn get_ids_to_download(
        self,
        index_map: &HashMap<String, LightweightIndexItem>,
        data_ids: &HashSet<String>,
        resolved_since: Option<DateTime<Utc>>,
    ) -> Vec<String> {
        let now = Utc::now();
        let mut ids_to_download = Vec::with_capacity(index_map.len());

        for (id, item) in index_map {
            // Skip if already downloaded
            if data_ids.contains(id) {
                continue;
            }

            if let Some(cutoff_date) = resolved_since {
                match &item.close_datetime {
                    None => {
                        // Skip if market is not resolved yet
                        // Or if resolution date is just missing
                        continue;
                    }
                    Some(resolved_at) => {
                        // Skip if resolution date is before cutoff
                        if resolved_at < &cutoff_date {
                            continue;
                        }
                        // Skip if resolution date is in the future
                        if &now < resolved_at {
                            continue;
                        }
                    }
                }
            }

            // Add item to the download list
            ids_to_download.push(id.clone());
        }

        debug!(
            "{self}: Selected {}/{} items to download",
            ids_to_download.len(),
            index_map.len(),
        );
        ids_to_download
    }
}

pub trait PlatformHandler {
    fn download(
        &self,
        output_dir: &Path,
        reset_index: &bool,
        reset_cache: &bool,
        resolved_since: Option<DateTime<Utc>>,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}
impl PlatformHandler for Platform {
    async fn download(
        &self,
        output_dir: &Path,
        reset_index: &bool,
        reset_cache: &bool,
        resolved_since: Option<DateTime<Utc>>,
    ) -> Result<()> {
        // Build file paths
        let index_file_path = output_dir.join(format!("{self}-index.jsonl").to_lowercase());
        let data_file_path = output_dir.join(format!("{self}-data.jsonl").to_lowercase());

        // Back up files if requested
        if *reset_index || *reset_cache {
            info!("{self}: Backing up file {}", index_file_path.display());
            backup_file(&index_file_path).context(format!(
                "Failed to back up file {}",
                index_file_path.display()
            ))?;
        }
        if *reset_cache {
            info!("{self}: Backing up file {}", data_file_path.display());
            backup_file(&data_file_path).context(format!(
                "Failed to back up file {}",
                data_file_path.display()
            ))?;
        }

        // Attempt to load the index file, or download if needed
        let needs_download = match load_index_from_file(&index_file_path).context(format!(
            "{self}: Failed to access index file {}",
            index_file_path.display()
        ))? {
            // Index file exists and is valid, keep it
            Some(index) => {
                info!("{self}: Index loaded from disk with {} items.", index.len());
                false
            }
            // Index file needs to be downloaded
            None => true,
        };

        if needs_download {
            info!("{self}: Downloading new index.");
            // Download the platform index directly to disk
            match self {
                Platform::Kalshi => kalshi::download_index(&index_file_path).await,
                Platform::Manifold => manifold::download_index(&index_file_path).await,
                Platform::Metaculus => metaculus::download_index(&index_file_path).await,
                Platform::Polymarket => polymarket::download_index(&index_file_path).await,
            }
            .context(format!("{self}: Failed to download index"))?;
            info!("{self}: Index downloaded and saved to disk.");
        }

        // Load the index for lightweight conversion
        let index = load_index_from_file(&index_file_path)
            .context(format!("{self}: Failed to load index file after download"))?
            .ok_or_else(|| anyhow::anyhow!("{self}: Index file should exist after download"))?;

        // Convert index into a lightweight hashmap to reduce memory usage
        // TODO: Can we cut out this step? Just get_ids_to_download and then discard the index?
        debug!("{self}: Converting index into lightweight HashMap.");
        let index_map = self.build_lightweight_index(index);

        // Load the data file from the disk
        // If it does not exist, create an empty file
        // Note that this can be very large!
        info!("{self}: Loading cached data progress from disk.");
        let data_ids = load_data_ids(&data_file_path)
            .context(format!("{self}: Failed to load cached data progress"))?;
        info!(
            "{self}: Data cache loaded from disk with {} items.",
            data_ids.len()
        );

        // Get the IDs in index file that aren't in data file
        debug!("{self}: Getting IDs to download.");
        let ids_to_download = self.get_ids_to_download(&index_map, &data_ids, resolved_since);
        let num_to_download = ids_to_download.len();

        // Check if anything needs to be downloaded
        if num_to_download == 0 {
            info!("{self}: All {} items already downloaded.", data_ids.len());
        } else {
            info!(
                "{self}: Starting data download: {} downloaded, {} pending",
                data_ids.len(),
                num_to_download
            );
            if let Err(err) = match self {
                Platform::Kalshi => {
                    kalshi::download_data(&index_file_path, &ids_to_download, &data_file_path).await
                }
                Platform::Manifold => {
                    manifold::download_data(&index_file_path, &ids_to_download, &data_file_path)
                        .await
                }
                Platform::Metaculus => {
                    metaculus::download_data(&index_file_path, &ids_to_download, &data_file_path)
                        .await
                }
                Platform::Polymarket => {
                    polymarket::download_data(&index_file_path, &ids_to_download, &data_file_path)
                        .await
                }
            } {
                return Err(err).context(format!("{self}: Error downloading data"));
            }
            debug!("{self}: Main download task complete.");

            // Confirm how many we actually got
            debug!("{self}: Checking data on disk.");
            let downloaded_ids = load_data_ids(&data_file_path)
                .context(format!("{self}: Failed to check downloaded data"))?;
            let num_downloaded = ids_to_download
                .iter()
                .filter(|id| downloaded_ids.contains(*id))
                .count();
            if num_downloaded == num_to_download {
                info!("{self}: All {num_to_download} items downloaded");
            } else {
                #[allow(clippy::cast_precision_loss)]
                let percent_complete = if num_to_download > 0 {
                    (num_downloaded as f64 / num_to_download as f64) * 100.0
                } else {
                    0.0
                };
                warn!(
                    "{self}: {num_downloaded} out of {num_to_download} items downloaded ({percent_complete:.1}%)"
                );
                warn!("Re-run the download program to retry the failed items.");
            }
        }

        Ok(())
    }
}
