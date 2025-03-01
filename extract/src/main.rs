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
        info!(
            "{platform}: Data loaded. Extracting {} items...",
            lines.len()
        );

        if !args.schema_only {
            for line in lines {
                let standardized_markets = platform.standardize(line)?;
                if !args.offline {
                    for market_data in standardized_markets {
                        upload_item(
                            &client,
                            &postgrest_api_base,
                            &postgrest_api_key,
                            &market_data,
                        )?;
                    }
                }
            }
        }

        info!("{platform}: All items processed.");
    }

    Ok(())
}

/// TODO: This needs some work. Upsert correctly? Send probs with market? Batch uploads?
/// https://docs.postgrest.org/en/latest/references/api/tables_views.html#prefer-resolution
fn upload_item(
    client: &Client,
    postgrest_api_base: &str,
    postgrest_api_key: &str,
    market_data: &MarketAndProbs,
) -> Result<()> {
    // Upload market
    debug!("Uploading market: {}", market_data.market.title);
    let market_response = client
        .post(format!("{}/markets", postgrest_api_base))
        .bearer_auth(postgrest_api_key)
        .header("Prefer", "resolution=merge-duplicates")
        .header("On-Conflict-Update", "*")
        .json(&market_data.market)
        .send()
        .context("Failed to send market upload request")?;

    let market_status = market_response.status();
    let market_body = market_response
        .text()
        .context("Failed to read market response body")?;
    debug!(
        "Market upload response ({}): {}",
        market_status, market_body
    );

    if !market_status.is_success() {
        return Err(anyhow::anyhow!(
            "Market upload failed with status {} and body: {}",
            market_status,
            market_body
        ));
    }

    // Upload probabilities
    debug!(
        "Uploading {} probabilities for market",
        market_data.daily_probabilities.len()
    );
    let probs_response = client
        .post(format!("{}/daily_probabilities", postgrest_api_base))
        .bearer_auth(postgrest_api_key)
        .header("Prefer", "resolution=merge-duplicates")
        .header("On-Conflict-Update", "*")
        .json(&market_data.daily_probabilities)
        .send()
        .context("Failed to send probabilities upload request")?;

    let probs_status = probs_response.status();
    let probs_body = probs_response
        .text()
        .context("Failed to read probabilities response body")?;
    debug!(
        "Probabilities upload response ({}): {}",
        probs_status, probs_body
    );

    if !probs_status.is_success() {
        return Err(anyhow::anyhow!(
            "Probabilities upload failed with status {} and body: {}",
            probs_status,
            probs_body
        ));
    }

    Ok(())
}
