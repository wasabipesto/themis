//! Module containing common API functions.

use crate::{scores::MarketScore, DailyProbabilityPoint, Question};

use super::PostgrestParams;
use anyhow::{Context, Result};
use reqwest::blocking::{Client, Response};
use std::{fmt::Display, time::Duration};

use super::Market;

/// HTTP methods supported by our API wrapper
pub enum HttpMethod {
    GET,
    POST,
    DELETE,
}
impl Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::GET => write!(f, "GET"),
            HttpMethod::POST => write!(f, "POST"),
            HttpMethod::DELETE => write!(f, "DELETE"),
        }
    }
}

/// Makes an API request with the specified method and handles errors consistently
fn make_request(
    client: &Client,
    url: String,
    method: HttpMethod,
    auth_key: &str,
) -> Result<Response> {
    let request_builder = match method {
        HttpMethod::GET => client.get(&url),
        HttpMethod::POST => client.post(&url),
        HttpMethod::DELETE => client.delete(&url),
    };

    request_builder
        .bearer_auth(auth_key)
        .send()
        .with_context(|| format!("Failed to send {method} request to {url}"))
}

/// Process API response, returning deserialized data or an error
fn process_response<T>(response: Response, error_context: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let status = response.status();
    if status.is_success() {
        response
            .json::<T>()
            .context("Failed to parse response JSON")
    } else {
        let body = response.text()?;
        Err(anyhow::anyhow!(
            "{} failed with status {} and body: {}",
            error_context,
            status,
            body
        ))
    }
}

/// Process a response when we don't need the body content, just success/failure
fn process_empty_response(response: Response, error_context: &str) -> Result<()> {
    let status = response.status();
    if status.is_success() {
        Ok(())
    } else {
        let body = response.text()?;
        Err(anyhow::anyhow!(
            "{} failed with status {} and body: {}",
            error_context,
            status,
            body
        ))
    }
}

/// Downloads all markets from the database.
/// Paginates through the PostgREST endpoint until all are downloaded.
pub fn get_all_markets(client: &Client, params: &PostgrestParams) -> Result<Vec<Market>> {
    let limit = 1000;
    let mut offset = 0;
    let mut markets = Vec::new();

    loop {
        let url = format!(
            "{}/market_details?order=id&limit={}&offset={}",
            params.postgrest_url, limit, offset
        );

        let response = make_request(client, url, HttpMethod::GET, &params.postgrest_api_key)?;
        let body: Vec<Market> = process_response(response, "Download markets")?;

        if body.is_empty() {
            break;
        }

        markets.extend(body);
        offset += limit;
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
        let url = format!(
            "{}/question_details?id=eq.{}",
            params.postgrest_url, question_id
        );

        let response = make_request(client, url, HttpMethod::GET, &params.postgrest_api_key)?;
        let body: Vec<Question> = process_response(response, "Download questions")?;

        if let Some(question) = body.first() {
            questions.push(question.clone());
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
        let url = format!(
            "{}/daily_probabilities?order=date&market_id=eq.{}",
            params.postgrest_url, market_id
        );

        let response = make_request(client, url, HttpMethod::GET, &params.postgrest_api_key)?;
        let body: Vec<DailyProbabilityPoint> =
            process_response(response, "Download probabilities")?;

        probs.extend(body);
    }

    Ok(probs)
}

/// Wipes all market scores from the database.
pub fn wipe_market_scores(client: &Client, params: &PostgrestParams) -> Result<()> {
    let url = format!("{}/market_scores", params.postgrest_url);

    let response = make_request(client, url, HttpMethod::DELETE, &params.postgrest_api_key)?;
    process_empty_response(response, "Wipe market scores")
}

/// Uploads market scores to the database.
pub fn upload_market_scores(
    client: &Client,
    params: &PostgrestParams,
    scores: &[MarketScore],
) -> Result<()> {
    let url = format!("{}/market_scores", params.postgrest_url);

    let response = client
        .post(&url)
        .bearer_auth(&params.postgrest_api_key)
        .json(scores)
        .send()
        .with_context(|| format!("Failed to send POST request to {url}"))?;
    process_empty_response(response, "Wipe market scores")
}

/// Refreshes the market and question materialized views in the database.
pub fn refresh_quick_materialized_views(client: &Client, params: &PostgrestParams) -> Result<()> {
    let url = format!(
        "{}/rpc/refresh_quick_materialized_views",
        params.postgrest_url
    );

    let response = make_request(client, url, HttpMethod::POST, &params.postgrest_api_key)?;
    process_empty_response(response, "Refresh quick materialized views")
}

/// Refreshes all materialized views in the database.
/// Should be called after all data has been uploaded to ensure views are up-to-date.
/// Uses a longer timeout since this operation can take around 60 seconds.
pub fn refresh_all_materialized_views(params: &PostgrestParams) -> Result<()> {
    // Create a new client with a longer timeout specifically for this operation
    let timeout = Duration::from_secs(180); // 3 minute timeout
    let long_timeout_client = Client::builder()
        .timeout(timeout)
        .build()
        .context("Failed to create HTTP client with extended timeout")?;

    let url = format!(
        "{}/rpc/refresh_all_materialized_views",
        params.postgrest_url
    );

    let response = make_request(
        &long_timeout_client,
        url,
        HttpMethod::POST,
        &params.postgrest_api_key,
    )?;
    process_empty_response(response, "Refresh all materialized views")
}
