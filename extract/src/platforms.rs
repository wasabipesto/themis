//! Anything that switches based on platform.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::Serialize;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub mod helpers;
pub mod kalshi;
pub mod manifold;
pub mod metaculus;
pub mod polymarket;

/// Supported platforms.
#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum Platform {
    Kalshi,
    Manifold,
    Metaculus,
    Polymarket,
}

/// Standardized market with history data.
#[derive(Debug, Serialize, Clone)]
pub struct MarketAndProbs {
    pub market: StandardMarket,
    pub daily_probabilities: Vec<DailyProbability>,
}

/// Standardized market. It has everything we need.
#[derive(Debug, Serialize, Clone)]
pub struct StandardMarket {
    pub id: String,
    pub title: String,
    pub platform_slug: String,
    pub platform_name: String,
    pub description: String,
    pub url: String,
    pub open_datetime: DateTime<Utc>,
    pub close_datetime: DateTime<Utc>,
    pub traders_count: Option<u32>,
    pub volume_usd: Option<f32>,
    pub duration_days: u32,
    pub category: Option<String>,
    pub prob_at_midpoint: f32,
    pub prob_time_avg: f32,
    pub resolution: f32,
}

/// A fully-constructed probability data point.
#[derive(Debug, Serialize, Clone)]
pub struct DailyProbability {
    pub market_id: String,
    pub platform_slug: String,
    pub date: DateTime<Utc>,
    pub prob: f32,
}

/// A segment of time and the market probability during that period.
#[derive(Debug, Serialize, Clone)]
pub struct ProbSegment {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub prob: f32,
}

/// Deserialized JSONL line straight from the disk. One of any platform type.
/// Boxed due to large size differences between each platform.
#[derive(Clone)]
pub enum PlatformData {
    Kalshi(Box<kalshi::KalshiData>),
    Manifold(Box<manifold::ManifoldData>),
    Metaculus(Box<metaculus::MetaculusData>),
    Polymarket(Box<polymarket::PolymarketData>),
}

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

impl Platform {
    pub fn all() -> Vec<Platform> {
        vec![
            Platform::Kalshi,
            Platform::Manifold,
            Platform::Metaculus,
            Platform::Polymarket,
        ]
    }

    /// Based on platform, deserialize a line into that platform's datatype.
    pub fn deserialize_line(&self, line: &str) -> Result<PlatformData> {
        match self {
            Platform::Kalshi => Ok(PlatformData::Kalshi(serde_json::from_str(line)?)),
            Platform::Manifold => Ok(PlatformData::Manifold(serde_json::from_str(line)?)),
            Platform::Metaculus => Ok(PlatformData::Metaculus(serde_json::from_str(line)?)),
            Platform::Polymarket => Ok(PlatformData::Polymarket(serde_json::from_str(line)?)),
        }
    }

    /// Find the first line in the platform data file matching the search term and deserialize it.
    pub fn load_line_match(&self, base_dir: &Path, search: &str) -> Result<PlatformData> {
        let file_name = format!("{}-data.jsonl", self).to_lowercase();
        let data_file_path = base_dir.join(file_name);

        let file = File::open(&data_file_path)
            .with_context(|| format!("Failed to open file: {}", data_file_path.display()))?;
        let reader = BufReader::new(file);

        for (line_number, line) in reader.lines().enumerate() {
            match line {
                Ok(line_content) => {
                    if line_content.contains(search) {
                        return self.deserialize_line(&line_content).with_context(|| {
                            format!("Failed to deserialize matching line {}", line_number + 1)
                        });
                    }
                }
                Err(err) => {
                    log::error!("Failed to read line {}: {}", line_number + 1, err);
                    continue;
                }
            }
        }

        anyhow::bail!("No line found containing search term: {}", search)
    }

    /// Find the appropriate data file based on platform name, then load and deserialize all lines.
    pub fn load_data(&self, base_dir: &Path) -> Result<Vec<PlatformData>> {
        let file_name = format!("{}-data.jsonl", self).to_lowercase();
        let data_file_path = base_dir.join(file_name);

        let file = File::open(&data_file_path)
            .with_context(|| format!("Failed to open file: {}", data_file_path.display()))?;
        let reader = BufReader::new(file);

        Ok(reader
            .lines()
            .enumerate()
            .filter_map(move |(line_number, line)| match line {
                Ok(line_content) => match self.deserialize_line(&line_content) {
                    Ok(item) => Some(item),
                    Err(err) => {
                        log::error!("Failed to deserialize line {}: {}", line_number + 1, err);
                        None
                    }
                },
                Err(err) => {
                    log::error!("Failed to read line {}: {}", line_number + 1, err);
                    None
                }
            })
            .collect())
    }

    /// Call each platform's standardize function.
    pub fn standardize(&self, input_unsorted: PlatformData) -> Result<Vec<MarketAndProbs>> {
        let rovm = match input_unsorted {
            PlatformData::Kalshi(input) => kalshi::standardize(&input)?,
            PlatformData::Manifold(input) => manifold::standardize(&input)?,
            PlatformData::Metaculus(input) => metaculus::standardize(&input)?,
            PlatformData::Polymarket(input) => polymarket::standardize(&input)?,
        };
        // Convert None into empty vec
        match rovm {
            Some(market_vec) => Ok(market_vec),
            None => Ok(Vec::new()),
        }
    }
}
