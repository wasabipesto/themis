//! Themis extract binary source.
//! Pulls all markets from cache files and standardizes them

use anyhow::{Context, Result};
use clap::Parser;
use log::{debug, info};
use reqwest::blocking::Client;
use std::env;
use std::path::PathBuf;
use std::time::Duration;

use themis_extract::platforms::{MarketAndProbs, Platform};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about)]
struct Args {
    /// Override the default platform list to only process one platform
    #[arg(short, long)]
    platform: Option<Platform>,

    /// Directory for JSON files
    #[arg(short, long, default_value = "../cache")]
    directory: PathBuf,

    /// Set the log level (e.g., error, warn, info, debug, trace)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Only check the schema, do not convert or upload to database.
    #[arg(short, long)]
    schema_only: bool,

    /// Only load and convert items, do not upload to database.
    #[arg(short, long)]
    offline: bool,

    /// API endpoint URL
    #[arg(short, long, default_value = "http://localhost:8000/api")]
    api_url: String,
}

fn main() -> Result<()> {
    // Get command line args
    let args = Args::parse();

    // Read log level from arg and update environment variable
    let log_level = args.log_level.to_lowercase();
    match log_level.as_str() {
        "error" | "warn" | "info" | "debug" | "trace" => env::set_var("RUST_LOG", log_level),
        _ => {
            println!("Invalid log level, resetting to INFO.");
            env::set_var("RUST_LOG", "info")
        }
    }
    env_logger::init();
    debug!("Command line args: {:?}", args);

    // If the user requested a specific platform, format it into a list
    // Otherwise, return the default platform list
    let platforms: Vec<Platform> = match args.platform {
        Some(platform) => Vec::from([platform]),
        None => Platform::all(),
    };
    debug!("Platforms to process: {:?}", platforms);

    // Initialize HTTP client
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    info!("Processing data for each platform");
    for platform in platforms {
        let mut items_processed = 0;

        for line in platform.load_data(&args.directory)? {
            if !args.schema_only {
                let standardized_markets = platform.standardize(line)?;
                if !args.offline {
                    for market_data in standardized_markets {
                        debug!("Uploading item {} for {}", items_processed, platform);
                        upload_item(&client, &args.api_url, &market_data)?;
                    }
                }
            }
            items_processed += 1;
        }

        info!("{}: {} items processed", platform, items_processed);
    }

    Ok(())
}

fn upload_item(client: &Client, api_url: &str, market_data: &MarketAndProbs) -> Result<()> {
    // Upload market
    debug!("Uploading market: {}", market_data.market.title);
    client
        .post(format!("{}/markets", api_url))
        .json(&market_data.market)
        .send()
        .context("Failed to upload market")?;
    // TODO: Get returned ID, set linked market prob
    // Or ideally submit them all at once and have PG link them

    // Upload probabilities
    debug!(
        "Uploading {} probabilities for market",
        market_data.daily_probabilities.len()
    );
    client
        .post(format!("{}/probabilities", api_url))
        .json(&market_data.daily_probabilities)
        .send()
        .context("Failed to upload probabilities")?;

    Ok(())
}
