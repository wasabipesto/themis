//! Module containing common API functions.

use crate::{
    scores::{MarketScore, OtherScore, PlatformCategoryScore},
    Category, DailyProbabilityPoint, Platform, Question,
};

use super::PostgrestParams;
use anyhow::{Context, Result};
use log::debug;
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

/// Make a simple get request
fn make_get_request<T>(client: &Client, params: &PostgrestParams, endpoint: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let response = make_request(
        client,
        params,
        endpoint,
        HttpMethod::GET,
        Option::<&()>::None,
    )?;
    process_response(response, endpoint)
}

/// Make a simple post request
fn make_post_request(
    client: &Client,
    params: &PostgrestParams,
    endpoint: &str,
    body: Option<&impl serde::Serialize>,
) -> Result<()> {
    let response = make_request(client, params, endpoint, HttpMethod::POST, body)?;
    process_empty_response(response, endpoint)
}

/// Make a simple delete request
fn make_delete_request(client: &Client, params: &PostgrestParams, endpoint: &str) -> Result<()> {
    let response = make_request(
        client,
        params,
        endpoint,
        HttpMethod::DELETE,
        Option::<&()>::None,
    )?;
    process_empty_response(response, endpoint)
}

/// Makes an API request with the specified method and handles errors consistently
fn make_request(
    client: &Client,
    params: &PostgrestParams,
    endpoint: &str,
    method: HttpMethod,
    body: Option<&impl serde::Serialize>,
) -> Result<Response> {
    // Build the URL
    let url = format!("{}{}", params.postgrest_url, endpoint);
    debug!("Sending a {method} request to {url}");

    // Set the correct method
    let mut request_builder = match method {
        HttpMethod::GET => client.get(url),
        HttpMethod::POST => client.post(url),
        HttpMethod::DELETE => client.delete(url),
    };

    // Add bearer auth
    request_builder = request_builder.bearer_auth(&params.postgrest_api_key);

    // Add the body to the request if provided
    if let Some(data) = body {
        request_builder = request_builder.json(data);
    }

    // Ship it
    request_builder
        .send()
        .with_context(|| format!("Failed to send {method} request to {endpoint}"))
}

/// Process API response, returning deserialized data or an error
fn process_response<T>(response: Response, endpoint: &str) -> Result<T>
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
            "Request to {endpoint} failed with status {status} and body: {body}",
        ))
    }
}

/// Process a response when we don't need the body content, just success/failure
fn process_empty_response(response: Response, endpoint: &str) -> Result<()> {
    let status = response.status();
    if status.is_success() {
        Ok(())
    } else {
        let body = response.text()?;
        Err(anyhow::anyhow!(
            "Request to {endpoint} failed with status {status} and body: {body}",
        ))
    }
}

/// Downloads all markets from the database.
/// Paginates through the PostgREST endpoint until all are downloaded.
pub fn get_all_markets(client: &Client, params: &PostgrestParams) -> Result<Vec<Market>> {
    let limit = 10000;
    let mut offset = 0;
    let mut markets = Vec::new();

    loop {
        let endpoint = format!(
            "/market_details?order=id.asc&limit={}&offset={}",
            limit, offset
        );
        let body: Vec<Market> = make_get_request(client, params, &endpoint)?;
        if body.is_empty() {
            break;
        }
        markets.extend(body);
        offset += limit;
    }

    Ok(markets)
}

/// Downloads all platforms from the database.
pub fn get_all_platforms(client: &Client, params: &PostgrestParams) -> Result<Vec<Platform>> {
    let response: Vec<Platform> = make_get_request(client, params, "/platform_details")?;
    Ok(response)
}

/// Downloads all categories from the database.
pub fn get_all_categories(client: &Client, params: &PostgrestParams) -> Result<Vec<Category>> {
    let response: Vec<Category> = make_get_request(client, params, "/category_details")?;
    Ok(response)
}

/// Downloads all requested questions by ID.
pub fn get_questions(client: &Client, params: &PostgrestParams) -> Result<Vec<Question>> {
    let response: Vec<Question> = make_get_request(client, params, "/question_details")?;
    Ok(response)
}

/// Downloads all requested probability points by market ID.
pub fn get_market_probs(
    client: &Client,
    params: &PostgrestParams,
    market_ids: &[String],
) -> Result<Vec<DailyProbabilityPoint>> {
    let mut probs = Vec::new();

    for market_id in market_ids {
        let endpoint = format!(
            "/daily_probabilities?order=date.asc&market_id=eq.{}",
            market_id
        );
        let response: Vec<DailyProbabilityPoint> = make_get_request(client, params, &endpoint)?;
        probs.extend(response);
    }

    Ok(probs)
}

/// Wipes all market scores from the database.
pub fn wipe_market_scores(client: &Client, params: &PostgrestParams) -> Result<()> {
    make_delete_request(client, params, "/market_scores")
}

/// Uploads market scores to the database.
pub fn upload_market_scores(
    client: &Client,
    params: &PostgrestParams,
    scores: &Vec<MarketScore>,
) -> Result<()> {
    make_post_request(client, params, "/market_scores", Some(scores))
}

/// Wipes all platform-category scores from the database.
pub fn wipe_platform_category_scores(client: &Client, params: &PostgrestParams) -> Result<()> {
    make_delete_request(client, params, "/platform_category_scores")
}

/// Uploads platform-category scores to the database.
pub fn upload_platform_category_scores(
    client: &Client,
    params: &PostgrestParams,
    scores: &Vec<PlatformCategoryScore>,
) -> Result<()> {
    make_post_request(client, params, "/platform_category_scores", Some(scores))
}

/// Wipes all other scores from the database.
pub fn wipe_other_scores(client: &Client, params: &PostgrestParams) -> Result<()> {
    make_delete_request(client, params, "/other_scores")
}

/// Uploads other scores to the database.
pub fn upload_other_scores(
    client: &Client,
    params: &PostgrestParams,
    scores: &Vec<OtherScore>,
) -> Result<()> {
    make_post_request(client, params, "/other_scores", Some(scores))
}

/// Refreshes the market and question materialized views in the database.
pub fn refresh_quick_materialized_views(client: &Client, params: &PostgrestParams) -> Result<()> {
    make_post_request(
        client,
        params,
        "/rpc/refresh_quick_materialized_views",
        Option::<&()>::None,
    )
}

/// Refreshes all materialized views in the database.
/// Should be called after all data has been uploaded to ensure views are up-to-date.
/// Uses a longer timeout since this operation can take around 60 seconds.
pub fn refresh_all_materialized_views(params: &PostgrestParams) -> Result<()> {
    // Create a new client with a longer timeout specifically for this operation
    let timeout = Duration::from_secs(180); // 3 minute timeout
    let client = Client::builder()
        .timeout(timeout)
        .build()
        .context("Failed to create HTTP client with extended timeout")?;

    make_post_request(
        &client,
        params,
        "/rpc/refresh_all_materialized_views",
        Option::<&()>::None,
    )
}
