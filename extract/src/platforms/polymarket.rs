//! Tools to download and process markets from the Polymarket API.
//! Polymarket API docs: https://docs.polymarket.com/#clob-api

use anyhow::{anyhow, Result};
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;

use super::{helpers, MarketAndProbs, ProbSegment, StandardMarket};

/// This is the container format we used to save items to disk earlier.
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketData {
    /// Market ID used for lookups.
    pub id: String,
    /// Timestamp the market was downloaded from the API.
    pub last_updated: DateTime<Utc>,
    // Values returned from the `/markets` endpoint.
    pub market: PolymarketMarket,
    /// The token that we got the price history for.
    pub prices_history_token: String,
    /// Values returned from the `/prices-history` endpoint.
    pub prices_history: Vec<PolymarketPricePoint>,
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

/// Values returned from the `/market` endpoint.
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
    /// The URL slug for this market in the form https://polymarket.com/event/{slug}
    /// This pattern does not always seem to work - need to establish why.
    pub market_slug: String,
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
/// Undocumented API endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketPricePoint {
    /// Timestamp of provided probability point.
    #[serde(with = "ts_seconds")]
    pub t: DateTime<Utc>,
    /// Probability at the given timestamp.
    pub p: f32,
}

/// Convert data pulled from the API into a standardized market item.
/// Returns Error if there were any actual problems with the processing.
/// Returns None if the market was invalid in an expected way.
/// Otherwise, returns a list of markets with probabilities.
/// Note: This is not a 1:1 conversion because some inputs contain multiple
/// discrete markets, and each of those have their own histories.
pub fn standardize(input: &PolymarketData) -> Result<Option<Vec<MarketAndProbs>>> {
    // Only process closed markets
    if !input.market.closed {
        return Ok(None);
    }

    // Get market ID. Construct from platform slug and ID within platform.
    let platform_slug = "polymarket".to_string();
    let market_id = format!("{}:{}", platform_slug, input.market.question_id);

    // Get probability segments. If there are none then skip.
    let probs = build_prob_segments(&input.prices_history);
    if probs.is_empty() {
        return Ok(None);
    }

    // Validate probability segments and collate into daily prob segments.
    if let Err(e) = helpers::validate_prob_segments(&probs) {
        log::error!("Error validating probability segments. ID: {market_id} Error: {e}");
        return Ok(None);
    }
    let daily_probabilities = helpers::get_daily_probabilities(&probs, &market_id, &platform_slug)?;

    // We only consider the market to be open while there are actual probabilities.
    let start = probs.first().unwrap().start;
    let end = probs.last().unwrap().end;

    // Sanity check for number of tokens (should always be 2).
    let num_tokens = input.market.tokens.len();
    if num_tokens != 2 {
        return Err(anyhow!(
            "Expected 2 tokens, found {num_tokens} tokens! ID: {market_id}"
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
            anyhow!(
                "Tracked price history token ID {} not found in market ID: {market_id}",
                input.prices_history_token
            )
        })?;

    // Check number of winners. For one winner, check the token status. For two winners = 50/50.
    let num_winners = input.market.tokens.iter().filter(|t| t.winner).count();
    let resolution = match num_winners {
        0 => return Ok(None), // Market not yet finalized
        1 => {
            // Normal case, check if our token won
            if tracked_token.winner {
                1.0
            } else {
                0.0
            }
        }
        2 => 0.5, // Two winners, prizes were split 50/50
        _ => {
            // More than two winners (not possible)
            log::error!(
                "Expected 1 or 2 winning tokens, found {num_winners} winning tokens! ID: {market_id}"
            );
            return Ok(None);
        }
    };

    // Sanity check for token prices (should always sum to 1).
    let sum_prices: f32 = input.market.tokens.iter().map(|t| t.price).sum();
    if !(0.99..1.01).contains(&sum_prices) {
        log::debug!("Expected token prices to sum to 1.0, found they summed to {sum_prices}! ID: {market_id}");
    }

    // Append the tracked outcome to the market title so we know which side we're tracking.
    let market_title = input.market.question.clone();
    let title = match tracked_token.outcome.as_str() {
        "Yes" => market_title,
        _ => format!("{market_title} | {}", tracked_token.outcome),
    };

    // Build standard market item.
    let market = StandardMarket {
        id: market_id,
        title,
        platform_slug,
        platform_name: "Polymarket".to_string(),
        question_id: None,
        question_invert: false,
        question_dismissed: 0,
        url: format!("https://polymarket.com/event/{}", input.market.market_slug),
        open_datetime: start,
        close_datetime: end,
        traders_count: None, // TODO: Polymarket has never exposed this easily but should be doable
        volume_usd: None, // TODO: Used to be in /markets, not sure where to find now (maybe /search?)
        duration_days: helpers::get_market_duration(start, end)?,
        category: get_category(&input.market.tags),
        prob_at_midpoint: helpers::get_prob_at_midpoint(&probs, start, end)?,
        prob_time_avg: helpers::get_prob_time_avg(&probs, start, end)?,
        resolution,
    };
    Ok(Some(vec![MarketAndProbs {
        market,
        daily_probabilities,
    }]))
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
        ("AI", "AI"),
        ("Business", "Economics"),
        ("CBB", "Sports"),
        ("Coronavirus", "Science"),
        ("Crypto", "Crypto"),
        ("EPL", "Sports"),
        //("Games", "Sports"),
        //("Mentions", "Culture"),
        ("NBA", "Sports"),
        ("NFL", "Sports"),
        ("NFTs", "Crypto"),
        ("Politics", "Politics"),
        ("Pop Culture", "Culture"),
        ("Science", "Science"),
        ("Soccer", "Sports"),
        ("Sports", "Sports"),
        ("Trump", "Politics"),
        ("Trump Presidency", "Politics"),
        ("USA Election", "Politics"),
    ];

    let category_map: HashMap<&str, &str> = CATEGORIES.iter().cloned().collect();

    tags.as_ref()?
        .iter()
        .find_map(|tag| category_map.get(tag.as_str()).map(|&cat| cat.to_string()))
}
