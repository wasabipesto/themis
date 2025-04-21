//! Themis extract binary source.
//! Pulls all markets from cache files and standardizes them

use anyhow::{Context, Result};
use clap::Parser;
use dotenvy::dotenv;
use log::{debug, info};
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::time::Duration;

use themis_extract::platforms::Platform;
use themis_extract::{MarketAndProbs, MarketError};

/// Collect this many markets before uploading to database.
/// Not a firm limit, can be exceeded if the last line has multiple markets.
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

/// Parameters that we need in order to upload items to the database.
struct PostgrestParams {
    postgrest_url: String,
    postgrest_api_key: String,
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
    let postgrest_params = match args.offline {
        false => PostgrestParams {
            postgrest_url: env::var("PGRST_URL")
                .expect("Required environment variable PGRST_URL not set."),
            postgrest_api_key: env::var("PGRST_APIKEY")
                .expect("Required environment variable PGRST_APIKEY not set."),
        },
        true => PostgrestParams {
            postgrest_url: env::var("PGRST_URL").unwrap_or_default(),
            postgrest_api_key: env::var("PGRST_APIKEY").unwrap_or_default(),
        },
    };

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
        let num_input = lines.len();
        if args.schema_only {
            info!(
                "{platform}: Data loaded. All {} items deserialized correctly.",
                num_input
            );
            continue;
        }

        let mut num_skipped: usize = 0;
        let mut num_processed: usize = 0;
        let mut num_multiple: usize = 0;
        let mut num_uploaded: usize = 0;
        let mut error_counts: HashMap<String, usize> = HashMap::new();

        info!("{platform}: Data loaded. Extracting {} items...", num_input);
        let mut market_batch: Vec<MarketAndProbs> = Vec::with_capacity(BATCH_SIZE);
        for line in lines {
            let standardize_result = platform.standardize(line);
            let standardized_markets = match standardize_result {
                Ok(items) => items,
                Err(err) => {
                    // Track error counts by error type
                    let error_type = match &err {
                        MarketError::NotAMarket(_) => "Not Market",
                        MarketError::MarketNotResolved(_) => "Not Resolved",
                        MarketError::MarketCancelled(_) => "Cancelled",
                        MarketError::NoMarketTrades(_) => "No Trades",
                        MarketError::MarketTypeNotImplemented(_, _) => {
                            "Market Type Not Implemented"
                        }
                        MarketError::InvalidMarketTrades(_, _) => "*Invalid Trades",
                        MarketError::DataInvalid(_, _) => "*Invalid Data",
                        MarketError::ProcessingError(_, _) => "*Processing Error",
                    };

                    *error_counts.entry(error_type.to_string()).or_insert(0) += 1;

                    // Report errors by severity
                    match &err {
                        // Expected/informational errors
                        MarketError::NotAMarket(_)
                        | MarketError::MarketNotResolved(_)
                        | MarketError::MarketCancelled(_)
                        | MarketError::NoMarketTrades(_)
                        | MarketError::MarketTypeNotImplemented(_, _) => {
                            log::trace!("{err}");
                        }

                        // Actual errors that should be fixed
                        MarketError::InvalidMarketTrades(_, _)
                        | MarketError::DataInvalid(_, _)
                        | MarketError::ProcessingError(_, _) => {
                            log::error!("{err}");
                        }
                    }

                    // Return empty vector for all error cases
                    Vec::with_capacity(0)
                }
            };
            match standardized_markets.len() {
                0 => num_skipped += 1,
                1 => num_processed += 1,
                _ => {
                    num_processed += 1;
                    num_multiple += 1;
                }
            }
            if !args.offline {
                for market_data in standardized_markets {
                    market_batch.push(market_data);

                    // When we reach batch size, upload and clear the batch
                    if market_batch.len() >= BATCH_SIZE {
                        upload_batch(&client, &postgrest_params, &market_batch)?;
                        num_uploaded += market_batch.len();
                        market_batch.clear();
                    }
                }
            }
        }
        // Upload any remaining items in the final batch
        if !market_batch.is_empty() && !args.offline {
            upload_batch(&client, &postgrest_params, &market_batch)?;
            num_uploaded += market_batch.len();
        }

        // Report summary statistics
        info!("{platform} extraction summary:");
        info!("  Items in file: {num_input}");
        info!("  Skipped: {num_skipped}");
        info!("  Processed: {num_processed} ({num_multiple} groups)");
        info!("  Uploaded: {num_uploaded}");

        // Report error counts
        if !error_counts.is_empty() {
            let mut sorted_errors: Vec<(&String, &usize)> = error_counts.iter().collect();
            sorted_errors.sort_by(|a, b| a.0.cmp(b.0));
            info!("{platform} error counts by type:");
            for (error_type, count) in sorted_errors {
                info!("  {error_type}: {count}");
            }
        }
    }

    // Refresh materialized views
    if !args.offline {
        refresh_materialized_views(&postgrest_params)?;
    }

    Ok(())
}

/// Uploads a batch of standardized markets and their associated probability history.
/// PostgREST handles the insert/update logic. See docs here:
///   https://docs.postgrest.org/en/latest/references/api/tables_views.html#prefer-resolution
fn upload_batch(
    client: &Client,
    params: &PostgrestParams,
    market_batch: &[MarketAndProbs],
) -> Result<()> {
    // Upload markets batch
    debug!("Uploading batch of {} markets", market_batch.len());
    let markets: Vec<_> = market_batch.iter().map(|m| &m.market).collect();
    let market_response = client
        .post(format!("{}/markets", params.postgrest_url))
        .bearer_auth(&params.postgrest_api_key)
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

    // Upload daily probabilities batch
    let daily_probs: Vec<_> = market_batch
        .iter()
        .flat_map(|m| &m.daily_probabilities)
        .collect();

    debug!("Uploading batch of {} probabilities", daily_probs.len());
    let probs_response = client
        .post(format!("{}/daily_probabilities", params.postgrest_url))
        .bearer_auth(&params.postgrest_api_key)
        .header("Prefer", "resolution=merge-duplicates")
        .header("On-Conflict-Update", "*")
        .json(&daily_probs)
        .send()
        .context("Failed to send daily probabilities batch upload request")?;

    let probs_status = probs_response.status();
    if !probs_status.is_success() {
        let probs_body = probs_response.text()?;
        return Err(anyhow::anyhow!(
            "Daily probability batch upload failed with status {} and body: {}",
            probs_status,
            probs_body
        ));
    }

    // Upload criterion probabilities batch
    let criteria_probs: Vec<_> = market_batch
        .iter()
        .flat_map(|m| &m.criterion_probabilities)
        .collect();

    debug!("Uploading batch of {} probabilities", criteria_probs.len());
    let probs_response = client
        .post(format!("{}/criterion_probabilities", params.postgrest_url))
        .bearer_auth(&params.postgrest_api_key)
        .header("Prefer", "resolution=merge-duplicates")
        .header("On-Conflict-Update", "*")
        .json(&criteria_probs)
        .send()
        .context("Failed to send criterion probabilities batch upload request")?;

    let probs_status = probs_response.status();
    if !probs_status.is_success() {
        let probs_body = probs_response.text()?;
        return Err(anyhow::anyhow!(
            "Criterion probability batch upload failed with status {} and body: {}",
            probs_status,
            probs_body
        ));
    }

    Ok(())
}

/// Refreshes all materialized views in the database.
/// Should be called after all data has been uploaded to ensure views are up-to-date.
/// Uses a longer timeout since this operation can take around 60 seconds.
fn refresh_materialized_views(params: &PostgrestParams) -> Result<()> {
    info!("Refreshing all materialized views (this may take up to 2 minutes)");

    // Create a new client with a longer timeout specifically for this operation
    let timeout = Duration::from_secs(180); // 3 minute timeout
    let long_timeout_client = Client::builder()
        .timeout(timeout)
        .build()
        .context("Failed to create HTTP client with extended timeout")?;

    let response = long_timeout_client
        .post(format!(
            "{}/rpc/refresh_all_materialized_views",
            params.postgrest_url
        ))
        .bearer_auth(&params.postgrest_api_key)
        .send()
        .context("Failed to send refresh materialized views request")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text()?;
        return Err(anyhow::anyhow!(
            "Refresh materialized views failed with status {} and body: {}",
            status,
            body
        ));
    }

    info!("All materialized views refreshed successfully");
    Ok(())
}
