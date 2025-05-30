//! Themis grader binary source.
//! Pulls data from the database and grades it

use anyhow::{Context, Result};
use clap::Parser;
use dotenvy::dotenv;
use log::{debug, info};
use reqwest::blocking::Client;
use std::env;
use std::time::Duration;

use themis_grader::{api, scores, Market, PostgrestParams};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about)]
struct Args {
    /// Set the log level (e.g., error, warn, info, debug, trace)
    #[arg(short, long, default_value = "info")]
    log_level: String,
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

    // Refresh views
    info!("Refreshing quick views...");
    api::refresh_quick_materialized_views(&client, &postgrest_params)?;

    // Get all platforms and categories
    info!("Downloading platforms and categories...");
    let platforms = api::get_all_platforms(&client, &postgrest_params)?;
    let categories = api::get_all_categories(&client, &postgrest_params)?;

    // Get all markets
    info!("Downloading markets and questions...");
    let markets = api::get_all_markets(&client, &postgrest_params)?;
    let questions = api::get_questions(&client, &postgrest_params)?;
    info!(
        "{} markets and {} questions downloaded.",
        markets.len(),
        questions.len(),
    );

    // Get probabilities for linked markets.
    info!("Downloading probabilities...");
    let criterion_probs = api::get_all_criterion_probs(&client, &postgrest_params)?;
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
    info!(
        "{} criterion probabilities and {} daily probabilities downloaded.",
        criterion_probs.len(),
        linked_market_probs.len(),
    );

    // Calculate absolute scores.
    info!("Calculating absolute market scores...");
    let absolute_scores = scores::calculate_absolute_scores(&markets, criterion_probs)?;
    info!("Calculating relative market scores...");
    let relative_scores =
        scores::calculate_relative_scores(&questions, &linked_markets, &linked_market_probs)?;
    let all_market_scores = [absolute_scores, relative_scores].concat();
    info!("Uploading market scores...");
    api::wipe_market_scores(&client, &postgrest_params)?;
    api::upload_market_scores(&client, &postgrest_params, &all_market_scores)?;
    info!("{} market scores uploaded.", all_market_scores.len());

    // Average market scores into platform-category scores.
    info!("Aggregating market scores across platforms, categories, and questions...");
    let (platform_category_scores, other_scores) = scores::aggregate_platform_category_scores(
        &platforms,
        &categories,
        &questions,
        &linked_markets,
        &all_market_scores,
    );
    api::wipe_platform_category_scores(&client, &postgrest_params)?;
    api::upload_platform_category_scores(&client, &postgrest_params, &platform_category_scores)?;
    api::wipe_other_scores(&client, &postgrest_params)?;
    api::upload_other_scores(&client, &postgrest_params, &other_scores)?;
    info!(
        "{} aggregate scores uploaded.",
        platform_category_scores.len() + other_scores.len()
    );

    // Refresh all materialized views
    info!("Refreshing all materialized views...");
    api::refresh_all_materialized_views(&postgrest_params)?;
    info!("All views refreshed.");

    info!("Grading complete.");
    Ok(())
}
