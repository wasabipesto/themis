//! Themis extract binary source.
//! Pulls all markets from cache files and standardizes them

use anyhow::{Context, Result};
use clap::Parser;
use dotenvy::dotenv;
use log::{debug, info};
use reqwest::blocking::Client;
use std::env;
use std::path::PathBuf;
use std::time::Duration;

use themis_extract::platforms::{MarketAndProbs, Platform};

const BATCH_SIZE: usize = 10000;

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

    // Get environment variables
    dotenv().ok();
    let postgrest_host =
        env::var("PGRST_HOST").expect("Required environment variable PGRST_HOST not set.");
    let postgrest_port =
        env::var("PGRST_PORT").expect("Required environment variable PGRST_PORT not set.");
    let postgrest_api_base = format!("http://{postgrest_host}:{postgrest_port}");
    let postgrest_api_key =
        env::var("PGRST_APIKEY").expect("Required environment variable PGRST_APIKEY not set.");

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

    for platform in platforms {
        info!("{platform}: Loading data from disk.");
        let lines = platform.load_data(&args.directory)?;
        if args.schema_only {
            info!(
                "{platform}: Data loaded. All {} items deserialized correctly.",
                lines.len()
            );
            continue;
        }

        info!(
            "{platform}: Data loaded. Extracting {} items...",
            lines.len()
        );
        let mut market_batch: Vec<MarketAndProbs> = Vec::with_capacity(BATCH_SIZE);
        for line in lines {
            let standardized_markets = platform.standardize(line)?;
            if !args.offline {
                for market_data in standardized_markets {
                    market_batch.push(market_data);

                    // When we reach batch size, upload and clear the batch
                    if market_batch.len() >= BATCH_SIZE {
                        upload_batch(
                            &client,
                            &postgrest_api_base,
                            &postgrest_api_key,
                            &market_batch,
                        )?;
                        market_batch.clear();
                    }
                }
            }
        }
        // Upload any remaining items in the final batch
        if !market_batch.is_empty() && !args.offline {
            upload_batch(
                &client,
                &postgrest_api_base,
                &postgrest_api_key,
                &market_batch,
            )?;
        }
        info!("{platform}: All items processed.");
    }

    Ok(())
}

/// Uploads a batch of standardized markets and their associated probability history.
/// PostgREST handles the insert/update logic. See docs here:
///   https://docs.postgrest.org/en/latest/references/api/tables_views.html#prefer-resolution
fn upload_batch(
    client: &Client,
    postgrest_api_base: &str,
    postgrest_api_key: &str,
    market_batch: &[MarketAndProbs],
) -> Result<()> {
    // Upload markets batch
    debug!("Uploading batch of {} markets", market_batch.len());
    let markets: Vec<_> = market_batch.iter().map(|m| &m.market).collect();
    let market_response = client
        .post(format!("{}/markets", postgrest_api_base))
        .bearer_auth(postgrest_api_key)
        .header("Prefer", "resolution=merge-duplicates")
        .header("On-Conflict-Update", "*")
        .json(&markets)
        .send()
        .context("Failed to send markets batch upload request")?;

    let market_status = market_response.status();
    if !market_status.is_success() {
        let market_body = market_response.text()?;
        return Err(anyhow::anyhow!(
            "Markets batch upload failed with status {} and body: {}",
            market_status,
            market_body
        ));
    }

    // Upload probabilities batch
    let all_probs: Vec<_> = market_batch
        .iter()
        .flat_map(|m| &m.daily_probabilities)
        .collect();

    debug!("Uploading batch of {} probabilities", all_probs.len());
    let probs_response = client
        .post(format!("{}/daily_probabilities", postgrest_api_base))
        .bearer_auth(postgrest_api_key)
        .header("Prefer", "resolution=merge-duplicates")
        .header("On-Conflict-Update", "*")
        .json(&all_probs)
        .send()
        .context("Failed to send probabilities batch upload request")?;

    let probs_status = probs_response.status();
    if !probs_status.is_success() {
        let probs_body = probs_response.text()?;
        return Err(anyhow::anyhow!(
            "Probabilities batch upload failed with status {} and body: {}",
            probs_status,
            probs_body
        ));
    }

    Ok(())
}
