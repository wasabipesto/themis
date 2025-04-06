//! Themis grader binary source.
//! Pulls data from the database and grades it

use anyhow::{Context, Result};
use clap::Parser;
use dotenvy::dotenv;
use log::{debug, info};
use reqwest::blocking::Client;
use std::time::Duration;
use std::{collections::HashSet, env};

use themis_grader::{api, scores, Market, PostgrestParams};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about)]
struct Args {
    /// Set the log level (e.g., error, warn, info, debug, trace)
    #[arg(short, long, default_value = "info")]
    log_level: String,
    //
    // Future params:
    // - dry_run
    // - absolute_only
    // - relative_only
    // - platform_only
    // - calibration_only
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
    let postgrest_params = PostgrestParams {
        postgrest_url: env::var("PGRST_URL")
            .context("Required environment variable PGRST_URL not set.")?,
        postgrest_api_key: env::var("PGRST_APIKEY")
            .context("Required environment variable PGRST_APIKEY not set.")?,
    };

    // Initialize HTTP client
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    // Get all markets
    info!("Downloading all markets...");
    let markets = api::get_all_markets(&client, &postgrest_params)?;

    // Get probabilities for linked markets.
    info!("Downloading probabilities for linked markets...");
    let linked_markets: Vec<Market> = markets
        .iter()
        .filter(|market| market.question_id.is_some())
        .cloned()
        .collect();
    let linked_market_ids: Vec<String> = linked_markets
        .iter()
        .map(|market| market.id.clone())
        .collect();
    let linked_market_probs =
        api::get_market_probs(&client, &postgrest_params, &linked_market_ids)?;

    // Get questions for linked markets.
    info!("Downloading questions for linked markets...");
    let question_ids: Vec<u32> = linked_markets
        .iter()
        .map(|market| market.question_id.unwrap())
        .collect::<HashSet<u32>>()
        .iter()
        .cloned()
        .collect();
    let linked_questions = api::get_questions(&client, &postgrest_params, &question_ids)?;
    info!(
        "{} markets, {} questions, {} probabilities downloaded.",
        markets.len(),
        linked_questions.len(),
        linked_market_probs.len()
    );

    // Calculate absolute scores.
    info!("Calculating absolute scores...");
    let _absolute_scores = scores::calculate_absolute_scores(&markets)?;

    // Calculate relative scores.
    info!("Calculating relative scores...");
    let _relative_scores = scores::calculate_relative_scores(
        &linked_questions,
        &linked_markets,
        &linked_market_probs,
    )?;
    info!("All scores calculated.");

    // TODO:
    // Wipe market scores table.
    // Upload new market scores.
    // Average market scores into platform-category scores.
    // Wipe platform-category scores table.
    // Upload new platform-category scores.
    // Build calibration points.
    // Wipe calibration points table.
    // Upload new calibration points.

    // Refresh all materialized views
    info!("Refreshing all materialized views...");
    api::refresh_materialized_views(&postgrest_params)?;
    info!("All views refreshed.");

    info!("Grading complete.");
    Ok(())
}
