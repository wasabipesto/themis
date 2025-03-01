//! Tools to download and process markets from the Manifold API.
//! Manifold API docs: https://docs.manifold.markets/api
//! Source code: https://github.com/manifoldmarkets/manifold/tree/main/backend/api/src

use anyhow::Result;
use chrono::serde::{ts_milliseconds, ts_milliseconds_option};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::MarketAndProbs;

/// This is the container format we used to save items to disk earlier.
#[derive(Debug, Clone, Deserialize)]
pub struct ManifoldData {
    /// Market ID used for lookups.
    pub id: String,
    /// Timestamp the market was downloaded from the API.
    pub last_updated: DateTime<Utc>,
    // Values returned from the `/markets` endpoint.
    // Ignoring this because everything is also in `full_market`.
    // pub lite_market: Value,
    /// Values returned from the `/market` endpoint.
    pub full_market: ManifoldMarket,
    /// Values returned from the `/bets` endpoint.
    pub bets: Vec<ManifoldBet>,
}

/// Yes or No, used for betting up or down in a few different places.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum YesNo {
    Yes,
    No,
}

/// The mechanism for automatic market-making (AMM).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ManifoldMechanism {
    /// CPMM-1 is the mechanism for all binary markets.
    /// Replaced the old DPM-1 mechanism some time ago.
    #[serde(alias = "cpmm-1")]
    Cpmm1,
    /// A variation of CPMM-1 for multiple-choice markets.
    #[serde(alias = "cpmm-multi-1")]
    CpmmMulti1,
    /// Quadratic funding, not a market so we don't worry about them.
    Qf,
    /// Indicates there is no market mechanism.
    /// Used for polls, bounties, and maybe other things in the future.
    None,
}

/// The axis for the market (binary, MC, numeric, etc.).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ManifoldOutcomeType {
    /// Typical binary market.
    Binary,
    /// Multiple choice market. May or may not use auto-arbitrage.
    MultipleChoice,
    /// Older numeric type, like a typical binary market that resolves to MKT.
    PseudoNumeric,
    /// Newer numeric type, uses pre-defined bins and functions like multiple choice.
    Number,
    /// Pseudo-numeric market that does not resolve. Irrelevant.
    Stonk,
    /// Basic post with awards for comments.
    BountiedQuestion,
    /// Basic post with awards and fund matching.
    QuadraticFunding,
    /// Basic post with user poll.
    Poll,
}

/// Which in-app currency the market uses.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ManifoldToken {
    /// Default, play-money.
    Mana,
    /// Special, real money.
    Cash,
}

/// For multiple-choice markets, details on each answer.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifoldAnswer {
    pub id: String,
}

/// Values returned from the `/market` endpoint.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifoldMarket {
    /// The unique ID of this market.
    /// Can conatin multiple sub-markets (multiple choice, numeric).
    pub id: String,

    /// Question text, also used as the title.
    pub question: String,
    /// Full text description without special formatting.
    pub text_description: String,
    /// Canonical public URL to the market page.
    pub url: String,
    /// Public URL to the AI-generated or user-uploaded hero image.
    pub cover_image_url: Option<String>,
    /// The URL slug for this market.
    /// You should probably be using the pre-build URL instead.
    pub slug: String,
    /// A list of group slugs that have been added to this market.
    #[serde(default)] // default to empty vec
    pub group_slugs: Vec<String>,

    /// Market creator's user ID.
    pub creator_id: String,
    /// Market creator's avatar URL.
    pub creator_avatar_url: Option<String>,
    /// Market creator's username.
    pub creator_username: String,
    /// Market creator's display name.
    pub creator_name: String,

    /// Moment the market was created.
    /// Manifold markets are open for trade immedtaely upon creation.
    /// All times are in milliseconds since epoch.
    #[serde(with = "ts_milliseconds")]
    pub created_time: DateTime<Utc>,
    /// If in the future, the time the market creator has scheduled for the market
    /// to automatically close. If in the past, implies the market is closed.
    /// Note that this can be wildly in the future or past, should not be relied upon.
    #[serde(with = "ts_milliseconds_option")]
    #[serde(default)] // default to None
    pub close_time: Option<DateTime<Utc>>,
    /// Most recent moment the market was resolved.
    /// `None` if the market is not resolved.
    #[serde(with = "ts_milliseconds_option")]
    #[serde(default)] // default to None
    pub resolution_time: Option<DateTime<Utc>>,
    /// Most recent moment the market was updated.
    /// I'm not sure what is included as an update.
    #[serde(with = "ts_milliseconds")]
    pub last_updated_time: DateTime<Utc>,

    /// Whether or not this market is resolved.
    #[serde(default)] // default to false
    pub is_resolved: bool,
    /// How this market was resolved.
    /// This can be YES, NO, MKT, CANCEL, or an answer ID for multiple-choice.
    /// Note that Manifold markets can be unresolved and even reopened!
    /// TODO: Make an enum with default?
    pub resolution: Option<String>,
    /// When `resolution` is MKT, this is the value that it was resolved to.
    pub resolution_probability: Option<f32>,

    /// The mechanism for automatic market-making (AMM).
    /// Non-market items will have a mechanism but it will be None.
    pub mechanism: ManifoldMechanism,
    /// The axis for the market (binary, MC, numeric, etc.).
    pub outcome_type: ManifoldOutcomeType,
    /// Which in-app currency the market uses.
    pub token: ManifoldToken,

    /// For multiple-choice markets, details on each answer.
    pub answers: Option<Vec<ManifoldAnswer>>,
    /// If true, enables auto-arbitrage. In this case, we should treat this
    /// as a single market with one "choice" selected.
    /// If false, this is essentially a collection of "mini" markets.
    pub should_answers_sum_to_one: Option<bool>,

    /// Number of unique bettors in this market.
    pub unique_bettor_count: u32,
    /// Liquidity in the AMM, does not include open limit orders.
    /// Note that much of this is inaccessible since betting is limited to 1-99%.
    pub total_liquidity: Option<f32>,
    /// Total volume spent on this market.
    /// Note this includes sells so is susceptible to wash trading inflation.
    pub volume: f32,
}

/// Values returned from the `/bets` endpoint.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifoldBet {
    /// The unique ID of this bet.
    pub bet_id: String,
    /// Correponds to the market ID this bet was placed on.
    pub contract_id: String,
    /// Bettor's user ID.
    pub user_id: String,

    /// Moment the bet was submitted.
    /// Bet may have been filled later, see `fills` for more info.
    #[serde(with = "ts_milliseconds")]
    pub created_time: DateTime<Utc>,
    /// Unsure when this would be different than `created_time`.
    #[serde(with = "ts_milliseconds")]
    pub updated_time: DateTime<Utc>,

    /// Whether the user is trading YES or NO.
    /// Note that sells will be negative amounts, a sell YES order is
    /// equivalent to a buy NO order.
    pub outcome: YesNo,
    /// If multiple choice, which answer the trade is on.
    /// Note that is auto-arbitrage is enabled, this will affect other answers' probabilities.
    pub answer_id: Option<String>,

    /// The amount of mana/cash spent on the order.
    /// This can be negative if the user is selling their held shares.
    pub amount: f32,
    /// The total amount the user was willing to spend on the limit order.
    /// This may be less than `amount` if the entire order wasn't filled.
    pub order_amount: Option<f32>,
    /// The number of shares that the user has received from this order.
    /// They may get some from the AMM and some from matching orders.
    pub shares: f32,

    /// The implied probability of the market immediately before the trade was placed.
    pub prob_before: f32,
    /// The implied probability of the market immediately after the trade was placed.
    pub prob_after: f32,
    /// For limit orders, the probability the order will trigger at.
    pub limit_prob: Option<f32>,

    /// True if this was the bet to create a new multiple-choice answer.
    pub is_ante: Option<bool>,
    /// True if the order was placed via the public API.
    pub is_api: Option<bool>,
    /// True if the order was part of a challenge bet.
    /// Challenge bets are placed at a probability agreed upon by both sides,
    /// not necessarialy the market probability. These are no longer possible to make.
    pub is_challenge: Option<bool>,
    /// True if the order is completely filled.
    pub is_filled: Option<bool>,
    /// True if this trade caused the user to own simultaneous shares in
    /// YES and NO, automatically redeeming them for cash.
    pub is_redemption: Option<bool>,
}

/// Convert data pulled from the API into a standardized market item.
/// Returns Error if there were any actual problems with the processing.
/// Returns None if the market was invalid in an expected way.
/// Otherwise, returns a list of markets with probabilities.
/// Note: This is not a 1:1 conversion because some inputs contain multiple
/// discrete markets, and each of those have their own histories.
pub fn standardize(_input: &ManifoldData) -> Result<Option<Vec<MarketAndProbs>>> {
    todo!();
}
