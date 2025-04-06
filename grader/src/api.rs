//! Module containing common API functions.

use crate::{DailyProbabilityPoint, Question};

use super::PostgrestParams;
use anyhow::{Context, Result};
use reqwest::blocking::Client;
use std::time::Duration;

use super::Market;

/// Downloads all markets from the database.
/// Paginates through the PostgREST endpoint until all are downloaded.
pub fn get_all_markets(client: &Client, params: &PostgrestParams) -> Result<Vec<Market>> {
    let limit = 1000;
    let mut offset = 0;
    let mut markets = Vec::new();

    loop {
        let response = client
            .get(format!(
                "{}/market_details?limit={}&offset={}",
                params.postgrest_url, limit, offset
            ))
            .bearer_auth(&params.postgrest_api_key)
            .send()
            .context("Failed to send download markets request")?;

        let status = response.status();
        if status.is_success() {
            let body = response.json::<Vec<Market>>()?;
            if body.is_empty() {
                break;
            }
            markets.extend(body);
            offset += limit;
        } else {
            let body = response.text()?;
            return Err(anyhow::anyhow!(
                "Download markets failed with status {} and body: {}",
                status,
                body
            ));
        }
    }

    Ok(markets)
}

/// Downloads all requested questions by ID.
pub fn get_questions(
    client: &Client,
    params: &PostgrestParams,
    question_ids: &[u32],
) -> Result<Vec<Question>> {
    let mut questions = Vec::with_capacity(question_ids.len());

    for question_id in question_ids {
        let response = client
            .get(format!(
                "{}/question_details?id=eq.{}",
                params.postgrest_url, question_id
            ))
            .bearer_auth(&params.postgrest_api_key)
            .send()
            .context("Failed to send download questions request")?;

        let status = response.status();
        if status.is_success() {
            let body = response.json::<Vec<Question>>()?;
            if let Some(question) = body.first() {
                questions.push(question.clone());
            }
        } else {
            let body = response.text()?;
            return Err(anyhow::anyhow!(
                "Download questions failed with status {} and body: {}",
                status,
                body
            ));
        }
    }

    Ok(questions)
}

/// Downloads all requested probability points by market ID.
pub fn get_market_probs(
    client: &Client,
    params: &PostgrestParams,
    market_ids: &[String],
) -> Result<Vec<DailyProbabilityPoint>> {
    let mut probs = Vec::new();

    for market_id in market_ids {
        let response = client
            .get(format!(
                "{}/daily_probability_details?market_id=eq.{}",
                params.postgrest_url, market_id
            ))
            .bearer_auth(&params.postgrest_api_key)
            .send()
            .context("Failed to send download probabilities request")?;

        let status = response.status();
        if status.is_success() {
            let body = response.json::<Vec<DailyProbabilityPoint>>()?;
            probs.extend(body);
        } else {
            let body = response.text()?;
            return Err(anyhow::anyhow!(
                "Download probabilities failed with status {} and body: {}",
                status,
                body
            ));
        }
    }

    Ok(probs)
}

/// Refreshes all materialized views in the database.
/// Should be called after all data has been uploaded to ensure views are up-to-date.
/// Uses a longer timeout since this operation can take around 60 seconds.
pub fn refresh_materialized_views(params: &PostgrestParams) -> Result<()> {
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
    if status.is_success() {
        Ok(())
    } else {
        let body = response.text()?;
        Err(anyhow::anyhow!(
            "Refresh materialized views failed with status {} and body: {}",
            status,
            body
        ))
    }
}
