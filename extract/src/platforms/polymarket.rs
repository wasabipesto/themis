//! Tools to download and process markets from the Polymarket API.
//! Polymarket API docs: https://docs.polymarket.com/#clob-api

use anyhow::Result;
use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::MarketAndProbs;

/// This is the container format we used to save items to disk earlier.
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketData {
    /// Market ID used for lookups.
    pub id: String,
    /// Timestamp the market was downloaded from the API.
    pub last_updated: DateTime<Utc>,
    // Values returned from the `/markets` endpoint.
    pub market: PolymarketMarket,
    /// Values returned from the `/prices-history` endpoint.
    pub prices_history: Vec<PolymarketPricePoint>,
}

/// Data on each token part of the market.
/// This is where we get resolution data.
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketToken {
    /// The unique ID for this token.
    /// TODO: Verify there will always be two tokens per market.
    pub token_id: String,
    /// The outcome for this token.
    /// For binary markets, this is Yes or No. Both tokens will be present.
    /// For multiple choice, this will be each option name (Even/Odd, Cheifs/Eagles).
    pub outcome: String,
    /// The (current?) price of this option.
    /// TODO: Verify all prices in this list should sum to 1.0.
    pub price: f32,
    /// True on the token that paid out to holders (resolved Yes).
    /// False on all others including still-active markets.
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
    /// The URL slug for this market.
    /// TODO: Find proper prefix, https://polymarket.com/event/ does not seem to work.
    pub market_slug: String,
    /// List of tags applied to this market.
    pub tags: Option<Vec<String>>,
    /// The image associated with the market.
    pub image: Option<String>,
    /// End date of the market.
    pub end_date_iso: Option<DateTime<Utc>>,

    /// Whether the market is live.
    pub active: bool,
    /// Whether the market is closed for trading.
    /// This can be true at the same time as `active`.
    pub closed: bool,
    /// Whether the market has been archived.
    /// Unsure of the effect of archiving.
    pub archived: bool,
    /// Uncertian. Very few markets have this.
    pub is_50_50_outcome: bool,

    /// Negative risk markets are mutually exclusive with another market.
    /// This allows users to hold positions in both wihtout spending extra capital.
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
    /// Each token corresponds to a posisble outcome.
    pub tokens: Vec<PolymarketToken>,
}

/// Values returned from the `/prices-history` endpoint.
/// Undocumented API endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct PolymarketPricePoint {
    /// Timestamp of provided probability point.
    /// TODO: Verify these are evenly-spaced in time.
    /// TODO: Check if the timestamp is the start, middle, or end of he time bucket.
    #[serde(with = "ts_milliseconds")]
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
pub fn standardize(_input: &PolymarketData) -> Result<Option<Vec<MarketAndProbs>>> {
    todo!();
}
