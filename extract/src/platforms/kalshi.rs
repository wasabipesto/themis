//! Utilities for processing Kalshi market data.
//! Kalshi API docs: https://trading-api.readme.io/

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;

use super::helpers;
use super::{MarketAndProbs, ProbSegment, StandardMarket};

/// This is the container format we used to save items to disk earlier.
#[derive(Debug, Clone, Deserialize)]
pub struct KalshiData {
    /// Market ID used for lookups.
    /// For Kalshi, this is `ticker`.
    pub id: String,
    /// Timestamp the market was downloaded from the API.
    pub last_updated: DateTime<Utc>,
    /// Values returned from the `/markets` endpoint.
    pub market: KalshiMarket,
    // Values returned from the `/series` endpoint.
    //pub series: Vec<KalshiSeriesItem>, // TODO
    /// Values returned from the `/trades` endpoint.
    pub history: Vec<KalshiHistoryItem>,
}

/// What kind of market this is.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KalshiMarketType {
    /// Binary is the only market type at this time.
    /// Binary markets can be grouped together to create multiple-choice.
    Binary,
}

/// What stage of the market life-cycle this is in.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KalshiMarketStatus {
    Initialized,
    Active,
    Inactive,
    Closed,
    Determined,
    Settled,
    /// Finalized is the status used after everything is paid out and complete.
    /// We will filter to only finalized markets for the database.
    Finalized,
}

/// Details about how the strikes are configured.
/// Not particularly relevant unless we want to estimate a numerical estimation from these.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KalshiMarketStrikeType {
    /// For YES outcome the expiration value should be greater than "floor_strike".
    Greater,
    /// For YES outcome the expiration value should be greater than or equal to "floor_strike".
    GreaterOrEqual,
    /// For YES outcome the expiration value should be less than "cap_strike".
    Less,
    /// For YES outcome the expiration value should be less than or equal to "cap_strike".
    LessOrEqual,
    /// For YES outcome the expiration value should be greater than or equal to "floor_strike" and less than or equal to "cap_strike".
    Between,
    /// For scalar markets only (which don't yet exist).
    /// A mapping from expiration values to settlement values of the YES/LONG side will be in "functional_strike".
    /// This is currently not used.
    Functional,
    /// A key value map from relationship -> structured target IDs. Metadata for these structured targets can be fetched via the /structured_targets endpoints.
    Structured,
    /// It will be one or more non-numerical values. For YES outcome the expiration values should be equal to the values in "custom_strike".
    Custom,
}

/// Used for multiple types. Can be Yes, No, or empty string (Blank).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum YesNoBlank {
    Yes,
    No,
    #[serde(alias = "")]
    Blank,
}

/// Values returned from the `/markets` endpoint.
/// https://trading-api.readme.io/reference/getmarkets-1
#[derive(Debug, Clone, Deserialize)]
pub struct KalshiMarket {
    /// The unique ID of the individual binary market.
    pub ticker: String,
    /// ID of the "event" (collection of markets with the same terms).
    /// https://trading-api.readme.io/reference/terms
    pub event_ticker: String,

    /// The market title as displayed on the site.
    pub title: String,
    /// The subtitle, usually the strike ("$87,000 to 87,249.99").
    pub subtitle: String,

    /// A plain language description of the most important market terms.
    /// Few items with more than a single line (separated by \n).
    pub rules_primary: String,
    /// Additional resolution criteria for the market.
    /// Blank (empty string) about half of the time.
    pub rules_secondary: String,
    /// Shortened title for the yes side of this market.
    /// Usually the same as the subtitle.
    pub yes_sub_title: String,
    /// Shortened title for the no side of this market.
    /// Usually the same as the subtitle.
    pub no_sub_title: String,

    /// Status of the market from Initialized to Finalized.
    /// We only care about Finalized.
    pub status: KalshiMarketStatus,
    /// How the market resolved (YES or NO).
    /// Only blank for non-Finalized markets.
    pub result: YesNoBlank,
    /// Type of market, for potential future use.
    /// Currently the only market type is Binary.
    pub market_type: KalshiMarketType,

    /// Strike type defines how the market strike (expiration value) is defined and evaluated.
    /// Can be missing sometimes, not sure why.
    pub strike_type: Option<KalshiMarketStrikeType>,
    /// Minimum value for a numerical strike.
    pub floor_strike: Option<f32>,
    /// Maximum value for a numerical strike.
    pub cap_strike: Option<f32>,
    /// Custom strike options for multiple choice.
    /// I haven't figured out how these work yet.
    pub custom_strike: Option<HashMap<String, String>>,

    // All dates are in ISO 8601 spec
    /// Moment the market was opened for trading.
    pub open_time: DateTime<Utc>,
    /// Moment that trading was halted.
    /// Unsure if they be closed and re-opened.
    pub close_time: DateTime<Utc>,
    /// Unsure what this refers to.
    pub expiration_time: DateTime<Utc>,
    /// Unsure what this refers to.
    pub latest_expiration_time: DateTime<Utc>,

    /// The total value of a single contract at settlement.
    /// Used as a conversion rate between contracts and dollars.
    /// One contract is always equal to 100 cents, so this is always 100.
    pub notional_value: f32,
    /// The minimum price movement in the market. All limit order prices must be in denominations of the tick size.
    /// Currently this is only ever 1 cent.
    pub tick_size: u32,
    /// Price for the last traded yes contract on this market.
    pub last_price: f32,

    /// Value for current offers in this market in cents.
    pub liquidity: f32,
    /// Number of contracts bought on this market.
    pub volume: f32,
    /// Number of contracts bought on this market dis-considering netting.
    pub open_interest: f32,
}

/// Values returned from the `/trades` endpoint.
/// TODO: Might change to GetMarketCandlesticks in the future.
/// https://trading-api.readme.io/reference/gettrades-1
#[derive(Debug, Clone, Deserialize)]
pub struct KalshiHistoryItem {
    /// Corresponds to market ticker.
    pub ticker: String,
    /// Unique ID for this trade.
    pub trade_id: String,
    /// Moment that the trade was made.
    pub created_time: DateTime<Utc>,
    /// Number of contracts to be bought or sold.
    pub count: u32,
    /// Yes price for this trade in cents.
    /// Always an integer, trades are always made at whole cents.
    pub yes_price: f32,
    /// Inversion of `yes_price`.
    pub no_price: f32,
    /// The maker is the user initiating the trade, while the taker is the
    /// opposite side. If the user was buying YES, then the taker will be on
    /// the NO side. Here, `taker_side` NO means the user bought YES shares.
    pub taker_side: YesNoBlank,
}

/// Convert data pulled from the API into a standardized market item.
/// Returns Error if there were any actual problems with the processing.
/// Returns None if the market was invalid in an expected way.
/// Otherwise, returns a list of markets with probabilities.
/// Note: This is not a 1:1 conversion because some inputs contain multiple
/// discrete markets, and each of those have their own histories.
pub fn standardize(input: &KalshiData) -> Result<Option<Vec<MarketAndProbs>>> {
    // Only process finalized markets
    match input.market.status {
        KalshiMarketStatus::Finalized => {}
        _ => return Ok(None),
    }

    // Convert based on market type
    match input.market.market_type {
        // Currently only binary markets exist
        KalshiMarketType::Binary => {
            // Get market ID. Construct from platform slug and ID within platform.
            let platform_slug = "kalshi".to_string();
            let market_id = format!("{}:{}", platform_slug, input.market.ticker);

            // Get probability segments. If there are none then skip.
            let probs = build_prob_segments(&input.history, &input.market.close_time);
            if probs.is_empty() {
                return Ok(None);
            }

            // Validate probability segments and collate into daily prob segments.
            if let Err(e) = helpers::validate_prob_segments(&probs) {
                log::error!("Error validating probability segments. ID: {market_id} Error: {e}");
                return Ok(None);
            }
            let daily_probabilities =
                helpers::get_daily_probabilities(&probs, &market_id, &platform_slug)?;

            // We only consider the market to be open while there are actual probabilities.
            let start = probs.first().unwrap().start;
            let end = probs.last().unwrap().end;

            // Build standard market item.
            let market = StandardMarket {
                id: market_id,
                title: input.market.title.clone(),
                platform_slug,
                platform_name: "Kalshi".to_string(),
                url: get_url(&input.market.ticker)?,
                open_datetime: start,
                close_datetime: end,
                traders_count: None, // Not available in API
                volume_usd: Some(input.market.volume),
                duration_days: helpers::get_market_duration(start, end)?,
                category: get_category(&input.market),
                prob_at_midpoint: helpers::get_prob_at_midpoint(&probs, start, end)?,
                prob_time_avg: helpers::get_prob_time_avg(&probs, start, end)?,
                resolution: match input.market.result {
                    YesNoBlank::Yes => 1.0,
                    YesNoBlank::No => 0.0,
                    YesNoBlank::Blank => return Ok(None),
                },
            };
            Ok(Some(vec![MarketAndProbs {
                market,
                daily_probabilities,
            }]))
        }
    }
}

/// Converts Kalshi events into standard probability segments.
pub fn build_prob_segments(
    raw_history: &[KalshiHistoryItem],
    market_end: &DateTime<Utc>,
) -> Vec<ProbSegment> {
    // Sort the history by time
    // This is actually quite fast, basically no performance impact in the grand scheme
    let mut history = raw_history.to_vec();
    history.sort_by_key(|item| item.created_time);

    let mut segments: Vec<ProbSegment> = Vec::new();

    for (i, event) in history.iter().enumerate() {
        // The start of the segment will equal the end of the previous one unless we skipped some.
        // Err on the side of using the previous segment's end timestamp unless it's the first one.
        let start = match segments.last() {
            Some(previous_segment) => previous_segment.end,
            None => event.created_time,
        };

        // The end of the segment will be the beginning of the next event or
        // (for the last event) the end of the market.
        let end = if i < history.len() - 1 {
            history[i + 1].created_time
        } else {
            // Check if the market end is after the beginning of this segment
            if market_end > &start {
                market_end.to_owned()
            } else {
                // Sometimes the market end is set in the past or some other weird date
                // If it would make the last segment negative, skip it
                continue;
            }
        };

        // The probability of the event is based on the event's yes_price.
        // The yes price is in cents so we divide by 100 to get a value in [0, 1].
        let prob = event.yes_price / 100.0;

        // These were originally added to reduce the total number of segments and save a little memory.
        // I've commented them out because there were some inconsistencies with how trades were being
        // handled and they weren't saving much memory after all. I think any remaining inconsistencies
        // are more likely due to how Kalshi orders their events.
        /*

        // If the duration is less than 1 millisecond, skip.
        if (end - start).num_milliseconds() < 1 {
            continue;
        }

        // If the probability is the same as the previous segment's prob, skip.
        if let Some(previous_segment) = segments.last() {
            if (previous_segment.prob - prob).abs() < f32::EPSILON {
                continue;
            }
        }

        */

        // If the duration is exactly 0, skip.
        // Decided to keep this due to issues with how the windowing functions work.
        if start == end {
            continue;
        }

        segments.push(ProbSegment { start, end, prob });
    }
    segments
}

/// TODO: Get the rest of the items needed for this, somehow.
/// Kalshi market URLs follow the form:
///   https://kalshi.com/markets/{series_ticker}/{series_slug}#{event_ticker}
/// You can't link to a specific market within an event, but you can target an event within a series.
/// Tickers are constructed by combining the series, event, and market IDs:
///   Series Ticker: KXETHD (Ethereum price)
///   Event Ticker:  KXETHD-24DEC1721 (Ethereum price on Dec 17th 2024 at 21:00 EST)
///   Market Ticker: KXETHD-24DEC1721-T3939.99
///     (Ethereum price on Dec 17th 2024 at 21:00 EST is $3,940 or above.)
/// Currently I'm not sure how to get the series_slug. That portion is not required
/// for basic links but it is required to target a specific event.
/// In this case the series URL is:
///   https://kalshi.com/markets/kxethd/ethereum-price-abovebelow#kxethd-24dec1721
fn get_url(market_ticker: &str) -> Result<String> {
    let mut ticker_parts = market_ticker.split('-');
    let series_ticker = ticker_parts
        .next()
        .ok_or_else(|| anyhow!("Invalid ticker format (missing hyphen): {market_ticker}"))?;
    Ok(format!("https://kalshi.com/markets/{series_ticker}"))
}

/// TODO: Get the series data and pull category.
/// Categories used to be in the market item but they moved to the series level.
/// It's not necessary but I'd like to have it for easy grouping and client-side charts later.
fn get_category(_market: &KalshiMarket) -> Option<String> {
    None
}
