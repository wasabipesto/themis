//! Tools to download and process markets from the Polymarket API.
//! Polymarket API docs: https://docs.polymarket.com/#clob-api

use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;

use super::{helpers, MarketAndProbs, MarketError, MarketResult, ProbSegment, StandardMarket};

/// This is the container format we used to save items to disk earlier.
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketData {
    /// Market ID used for lookups.
    pub id: String,
    /// Timestamp the market was downloaded from the API.
    pub last_updated: DateTime<Utc>,
    // Values returned from the `/markets` endpoint of the CLOB API.
    pub market: PolymarketMarket,
    /// The token that we got the price history for.
    pub prices_history_token: String,
    /// Values returned from the `/prices-history` endpoint.
    pub prices_history: Vec<PolymarketPricePoint>,
    /// Values returned from the `/markets` endpoint of the Gamma API.
    /// Gamma API is not always up to date and may not have the requested market.
    pub market_gamma: Option<PolymarketGammaMarket>,
}

/// Data on each token part of the market.
/// This is where we get resolution data.
/// There are always exactly two tokens per market.
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketToken {
    /// The unique ID for this token.
    pub token_id: String,
    /// The outcome for this token.
    /// For binary markets, this is Yes or No. Both tokens will be present.
    /// For multiple choice, this will be each option name (Even/Odd, Chiefs/Eagles).
    pub outcome: String,
    /// The most recent price of this option.
    pub price: f32,
    /// True on the token that paid out to holders (resolved Yes).
    /// It is possible but rare for both tokens to have winner = true.
    pub winner: bool,
}

/// Values returned from the CLOB API `/market` endpoint.
/// https://docs.polymarket.com/#markets
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketMarket {
    /// The unique ID of this market.
    pub question_id: String,
    /// The CTF condition ID. We do not use this.
    pub condition_id: String,

    /// Question text, also used as the title.
    pub question: String,
    /// Full text description with line breaks (\n\n).
    pub description: String,
    /// The slug for the market.
    pub market_slug: String,
    /// The URL slug for the overall event.
    pub event_slug: Option<String>,
    /// List of tags applied to this market.
    pub tags: Option<Vec<String>>,
    /// The image associated with the market.
    pub image: Option<String>,
    /// End date of the market.
    pub end_date_iso: Option<DateTime<Utc>>,

    /// Whether the market is live.
    /// Unsure what this signifies.
    pub active: bool,
    /// Whether the market is closed for trading.
    /// This can be true at the same time as `active`!
    pub closed: bool,
    /// Whether the market has been archived.
    /// Unsure what this signifies.
    pub archived: bool,
    /// Uncertain. Very few markets have this.
    pub is_50_50_outcome: bool,

    /// Negative risk markets are mutually exclusive with another market.
    /// This allows users to hold positions in both without spending extra capital.
    pub neg_risk: bool,
    pub neg_risk_market_id: Option<String>,
    pub neg_risk_request_id: Option<String>,

    /// Fees that the maker pays on this market.
    /// Currently 0 for all markets.
    pub maker_base_fee: f32,
    /// Fees that the taker pays on this market.
    /// Currently 0 for most markets, 200 on a few.
    pub taker_base_fee: f32,

    /// All tokens part of this market.
    /// Each token corresponds to a possible outcome.
    pub tokens: Vec<PolymarketToken>,
}

/// Values returned from the `/prices-history` endpoint.
/// https://docs.polymarket.com/#timeseries-data
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketPricePoint {
    /// Timestamp of provided probability point.
    #[serde(with = "ts_seconds")]
    pub t: DateTime<Utc>,
    /// Probability at the given timestamp.
    pub p: f32,
}

/// Values returned from the Gamma API `/markets` endpoint.
/// https://docs.polymarket.com/#markets-2
/// This endpoint has been unstable so we only use it when necessary.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolymarketGammaMarket {
    /// The total traded volume for this market.
    #[serde(default)] // Default to 0 volume
    pub volume_num: f32,
}

/// Convert data pulled from the API into a standardized market item or an error.
/// Note: This is not a 1:1 conversion because some inputs contain multiple
/// discrete markets, and each of those have their own histories.
pub fn standardize(input: &PolymarketData) -> MarketResult<Vec<MarketAndProbs>> {
    // Get market ID. Construct from platform slug and ID within platform.
    let platform_slug = "polymarket".to_string();
    let market_id = format!("{}:{}", platform_slug, input.market.question_id);

    // Only process closed markets
    if !input.market.closed {
        return Err(MarketError::MarketNotResolved(market_id.to_owned()));
    }

    // Get probability segments. If there are none then skip.
    let probs = build_prob_segments(&input.prices_history);
    if probs.is_empty() {
        return Err(MarketError::NoMarketTrades(market_id.to_owned()));
    }

    // Validate probability segments and collate into daily prob segments.
    helpers::validate_prob_segments(&probs)
        .map_err(|e| MarketError::InvalidMarketTrades(market_id.to_owned(), e.to_string()))?;
    let daily_probabilities = helpers::get_daily_probabilities(&probs, &market_id)
        .map_err(|e| MarketError::ProcessingError(market_id.to_owned(), e.to_string()))?;

    // We only consider the market to be open while there are actual probabilities.
    let start = probs.first().unwrap().start;
    let end = probs.last().unwrap().end;

    // Sanity check for number of tokens (should always be 2).
    let num_tokens = input.market.tokens.len();
    if num_tokens != 2 {
        return Err(MarketError::DataInvalid(
            market_id.to_owned(),
            "Invalid number of tokens (expected 2).".to_string(),
        ));
    }

    // Get the token that we tracked in order to determine which outcome resolution to pick.
    let tracked_token = input
        .market
        .tokens
        .iter()
        .find(|t| t.token_id == input.prices_history_token)
        .cloned()
        .ok_or_else(|| {
            MarketError::DataInvalid(
                market_id.to_owned(),
                "Tracked price history token not found in market".to_string(),
            )
        })?;

    // Check number of winners. For one winner, check the token status. For two winners = 50/50.
    let num_winners = input.market.tokens.iter().filter(|t| t.winner).count();
    let resolution = match num_winners {
        0 => {
            // Market not yet finalized
            return Err(MarketError::MarketNotResolved(market_id.to_owned()));
        }
        1 => {
            // Normal case, check if our token won
            if tracked_token.winner {
                1.0
            } else {
                0.0
            }
        }
        2 => {
            // Two winners, prizes were split 50/50
            0.5
        }
        _ => {
            // More than two winners (not possible)
            return Err(MarketError::DataInvalid(
                market_id.to_owned(),
                "Invalid number of winning tokens (expected 1 or 2)".to_string(),
            ));
        }
    };

    // Sanity check for token prices (should always sum to 1).
    let sum_prices: f32 = input.market.tokens.iter().map(|t| t.price).sum();
    if !(0.99..1.01).contains(&sum_prices) {
        return Err(MarketError::DataInvalid(
            market_id.to_owned(),
            "Invalid token price sum (expected 1)".to_string(),
        ));
    }

    // Append the tracked outcome to the market title so we know which side we're tracking.
    let market_title = input.market.question.clone();
    let title = match tracked_token.outcome.as_str() {
        "Yes" => market_title,
        _ => format!("{market_title} | {}", tracked_token.outcome),
    };

    // Build the URL from the event slug if existing, otherwise use the market slug.
    let url = match &input.market.event_slug {
        Some(event_slug) => format!("https://polymarket.com/event/{}", event_slug),
        None => format!("https://polymarket.com/event/{}", input.market.market_slug),
    };

    // Build standard market item.
    let market = StandardMarket {
        id: market_id.to_owned(),
        title,
        url,
        description: input.market.description.clone(),
        platform_slug,
        category_slug: get_category(&input.market.tags),
        open_datetime: start,
        close_datetime: end,
        traders_count: None,
        volume_usd: input.market_gamma.as_ref().map(|item| item.volume_num),
        duration_days: helpers::get_market_duration(start, end)
            .map_err(|e| MarketError::ProcessingError(market_id.to_owned(), e.to_string()))?,
        prob_at_midpoint: helpers::get_prob_at_midpoint(&probs, start, end)
            .map_err(|e| MarketError::ProcessingError(market_id.to_owned(), e.to_string()))?,
        prob_time_avg: helpers::get_prob_time_avg(&probs, start, end)
            .map_err(|e| MarketError::ProcessingError(market_id.to_owned(), e.to_string()))?,
        resolution,
    };
    Ok(vec![MarketAndProbs {
        market,
        daily_probabilities,
    }])
}

/// Converts Polymarket price points into standard probability segments.
pub fn build_prob_segments(raw_history: &[PolymarketPricePoint]) -> Vec<ProbSegment> {
    // Sort the history by time.
    let mut history = raw_history.to_vec();
    history.sort_by_key(|item| item.t);

    let mut segments: Vec<ProbSegment> = Vec::new();

    for (i, point) in history.iter().enumerate() {
        // The start of the segment will equal the end of the previous one unless we skipped some.
        // Err on the side of using the previous segment's end timestamp unless it's the first one.
        let start = match segments.last() {
            Some(previous_segment) => previous_segment.end,
            None => point.t,
        };

        // The end of the segment will be the beginning of the next event.
        // We don't trust end dates so the last trade is the end of the market.
        let end = if i < history.len() - 1 {
            history[i + 1].t
        } else {
            continue;
        };

        // If the duration is exactly 0, skip.
        // Decided to keep this due to issues with how the windowing functions work.
        if start == end {
            continue;
        }

        // Get the probability after the bet was made.
        let prob = point.p;

        segments.push(ProbSegment { start, end, prob });
    }
    segments
}

/// Manual mapping of tags to our standard categories.
fn get_category(tags: &Option<Vec<String>>) -> Option<String> {
    const CATEGORIES: [(&str, &str); 17] = [
        ("ai", "technology"),
        ("Business", "economics"),
        ("CBB", "sports"),
        ("Coronavirus", "science"),
        ("Crypto", "economics"),
        ("EPL", "sports"),
        //("Games", "sports"),
        //("Mentions", "culture"),
        ("NBA", "sports"),
        ("NFL", "sports"),
        ("NFTs", "economics"),
        ("Politics", "politics"),
        ("Pop Culture", "culture"),
        ("Science", "science"),
        ("Soccer", "sports"),
        ("Sports", "sports"),
        ("Trump", "politics"),
        ("Trump Presidency", "politics"),
        ("USA Election", "politics"),
    ];

    let category_map: HashMap<&str, &str> = CATEGORIES.iter().cloned().collect();

    tags.as_ref()?
        .iter()
        .find_map(|tag| category_map.get(tag.as_str()).map(|&cat| cat.to_string()))
}
