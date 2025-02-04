use anyhow::{Context, Result};
use clap::ValueEnum;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub mod kalshi;
use kalshi::KalshiData;
pub mod manifold;
use manifold::ManifoldData;
pub mod metaculus;
use metaculus::MetaculusData;
pub mod polymarket;
use polymarket::PolymarketData;

/// All possible platforms that are supported by this application.
#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum Platform {
    Kalshi,
    Manifold,
    Metaculus,
    Polymarket,
}

/// Container for different types of platform data as deserialized from file.
pub enum PlatformData {
    Kalshi(KalshiData),
    Manifold(ManifoldData),
    Metaculus(MetaculusData),
    Polymarket(PolymarketData),
}

impl fmt::Display for Platform {
    /// Allow formatting as a string.
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
    /// Returns a list of all supported platform types.
    pub fn all() -> Vec<Platform> {
        vec![
            Platform::Kalshi,
            Platform::Manifold,
            Platform::Metaculus,
            Platform::Polymarket,
        ]
    }
}

pub trait PlatformHandler {
    /// Locate, read, and deserialize the data file.
    fn load_data(&self, base_dir: &Path) -> Result<Vec<PlatformData>>;
}

impl PlatformHandler for Platform {
    fn load_data(&self, base_dir: &Path) -> Result<Vec<PlatformData>> {
        match self {
            Platform::Kalshi => load_jsonl::<KalshiData, _>(base_dir, *self, PlatformData::Kalshi),
            Platform::Manifold => {
                load_jsonl::<ManifoldData, _>(base_dir, *self, PlatformData::Manifold)
            }
            Platform::Metaculus => {
                load_jsonl::<MetaculusData, _>(base_dir, *self, PlatformData::Metaculus)
            }
            Platform::Polymarket => {
                load_jsonl::<PolymarketData, _>(base_dir, *self, PlatformData::Polymarket)
            }
        }
    }
}

/// Fetch the JSON file then deserialize each line with the appropriate platform's data scructure.
fn load_jsonl<T, F>(base_dir: &Path, platform: Platform, wrap: F) -> Result<Vec<PlatformData>>
where
    T: serde::de::DeserializeOwned,
    F: Fn(T) -> PlatformData,
{
    // build the file path
    let file_name = format!("{}-data.jsonl", platform).to_lowercase();
    let data_file_path: PathBuf = base_dir.join(file_name);

    // get the file and open it
    let file = File::open(&data_file_path)
        .with_context(|| format!("Failed to open file: {}", data_file_path.display()))?;
    let reader = BufReader::new(file);
    let mut items = Vec::new();

    // iterate over each line of the file
    for (line_number, line) in reader.lines().enumerate() {
        let line = line.with_context(|| {
            format!(
                "Failed to read line {} in file: {}",
                line_number + 1,
                data_file_path.display()
            )
        })?;

        // deserialize and check result
        match serde_json::from_str::<T>(&line) {
            Ok(parsed) => items.push(wrap(parsed)),
            Err(err) => {
                let trim_chars = 1000;
                let trimmed_line = if line.len() > trim_chars {
                    format!("{}...", &line[..trim_chars])
                } else {
                    line.clone()
                };
                log::error!(
                    "Failed to deserialize JSON at line {} in file {}: {} \nProblematic line: {}",
                    line_number + 1,
                    data_file_path.display(),
                    err,
                    trimmed_line
                );
            }
        }
    }

    Ok(items)
}
