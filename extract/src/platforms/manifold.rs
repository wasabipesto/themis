//! Tools to download and process markets from the Manifold API.
//! Manifold API docs: https://docs.manifold.markets/api
//! Source code: https://github.com/manifoldmarkets/manifold/tree/main/backend/api/src

use anyhow::{anyhow, Result};
use chrono::serde::{ts_milliseconds, ts_milliseconds_option};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

use super::{helpers, MarketAndProbs, ProbSegment, StandardMarket};

const MANIFOLD_EXCHANGE_RATE: f32 = 100.0;

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
    /// Newest numeric type, like a re-skinned multiple choice market.
    MultiNumeric,
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
    /// The ID of this specific answer.
    pub id: String,
    /// The index of the answer on the market UI.
    pub index: usize,
    /// The time this answer was added.
    #[serde(with = "ts_milliseconds")]
    pub created_time: DateTime<Utc>,
    /// The text of the answer.
    pub text: String,
    /// 'Other', the answer that represents all other answers, including answers added in the future.
    pub is_other: Option<bool>,
    /// If resolved, the latest resolution time.
    /// This is usually in ts_milliseconds, but occasionally an ISO 8601 string.
    /// WHY?????????????
    #[serde(default)]
    pub resolution_time: Option<Value>,
    /// If resolved to multiple, the proportion awarded to this answer.
    pub resolution_probability: Option<f32>,
}

/// Values returned from the `/market` endpoint.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifoldMarket {
    /// The unique ID of this market.
    /// Can contain multiple sub-markets (multiple choice, numeric).
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
    /// Manifold markets are open for trade immediately upon creation.
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
    /// Corresponds to the market ID this bet was placed on.
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
    /// not necessarily the market probability. These are no longer possible to make.
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
pub fn standardize(input: &ManifoldData) -> Result<Option<Vec<MarketAndProbs>>> {
    let platform_slug = "manifold".to_string();

    // Skip markets that have not resolved
    if !input.full_market.is_resolved {
        return Ok(None);
    }
    // Skip markets that were canceled
    if input.full_market.resolution == Some("CANCEL".to_string()) {
        return Ok(None);
    }

    // Reference full market response
    let market = &input.full_market;

    // Convert based on market type
    match market.outcome_type {
        // Typical single binary market
        ManifoldOutcomeType::Binary => {
            // Get market ID. Construct from platform slug and ID within platform.
            let market_id = format!("{}:{}", platform_slug, market.id);

            // Get probability segments. If there are none then skip.
            let probs = build_prob_segments(&input.bets);
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
                title: market.question.clone(),
                platform_slug,
                platform_name: "Manifold".to_string(),
                description: market.text_description.clone(), // TODO: Get links
                url: market.url.to_owned(),
                open_datetime: start,
                close_datetime: end,
                traders_count: Some(get_traders_count(&input.bets)),
                volume_usd: Some(get_volume_usd(&market.volume)),
                duration_days: helpers::get_market_duration(start, end)?,
                category: get_category(&market.group_slugs),
                prob_at_midpoint: helpers::get_prob_at_midpoint(&probs, start, end)?,
                prob_time_avg: helpers::get_prob_time_avg(&probs, start, end)?,
                resolution: match get_resolution_value_binary(market)? {
                    Some(res) => res,
                    None => return Ok(None),
                },
            };
            Ok(Some(vec![MarketAndProbs {
                market,
                daily_probabilities,
            }]))
        }
        // Multiple choice markets
        ManifoldOutcomeType::MultipleChoice => match market.should_answers_sum_to_one {
            Some(true) => {
                // Market has many potential outcomes but prices are automatically arbitraged
                // in order to keep everything summed to 100%.

                // Get market ID. Construct from platform slug and ID within platform.
                let market_id = format!("{}:{}", platform_slug, market.id);

                // Make sure answers are present
                let answers = market.answers.to_owned().ok_or(anyhow!(
                    "Multiple choice market does not have answers: {:?}",
                    market
                ))?;

                // Either one resolution is picked (most common), or multiple are picked and their
                // winnings are split proportionally.
                let resolved_answer = match &market.resolution {
                    Some(resolution) => {
                        if resolution == "CHOOSE_MULTIPLE" || resolution == "MKT" {
                            // This is currently not implemented. I'm not exactly sure how we would do this.
                            return Ok(None);
                        } else {
                            answers
                                .iter()
                                .find(|answer| &answer.id == resolution)
                                .ok_or(anyhow!(
                                "Market {}: No answer found matching the resolution ID {}: {:?}",
                                market.id,
                                resolution,
                                answers
                            ))?
                                .to_owned()
                        }
                    }
                    None => return Err(anyhow!("Market lacks resolution value: {:?}", market)),
                };

                // Since we only allow markets where one answer is selected and only refer to the
                // winning answer, the resolution will always be YES.
                let resolution = 1.0;

                // Append the tracked outcome to the market title so we know which side we're tracking.
                let title = format!("{} | {}", market.question, resolved_answer.text);

                // Filter bets for the resolved answer
                let bets: Vec<ManifoldBet> = input
                    .bets
                    .iter()
                    .filter(|bet| bet.answer_id.as_ref() == Some(&resolved_answer.id))
                    .cloned()
                    .collect();

                // Get probability segments. If there are none then skip.
                let probs = build_prob_segments(&bets);
                if probs.is_empty() {
                    return Ok(None);
                }

                // Validate probability segments and collate into daily prob segments.
                if let Err(e) = helpers::validate_prob_segments(&probs) {
                    log::error!(
                        "Error validating probability segments. ID: {market_id} Error: {e}"
                    );
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
                    title,
                    platform_slug,
                    platform_name: "Manifold".to_string(),
                    description: market.text_description.clone(), // TODO: Get links
                    url: market.url.to_owned(),
                    open_datetime: start,
                    close_datetime: end,
                    traders_count: Some(get_traders_count(&input.bets)),
                    volume_usd: Some(get_volume_usd(&market.volume)),
                    duration_days: helpers::get_market_duration(start, end)?,
                    category: get_category(&market.group_slugs),
                    prob_at_midpoint: helpers::get_prob_at_midpoint(&probs, start, end)?,
                    prob_time_avg: helpers::get_prob_time_avg(&probs, start, end)?,
                    resolution,
                };
                Ok(Some(vec![MarketAndProbs {
                    market,
                    daily_probabilities,
                }]))
            }
            Some(false) => {
                // Collection of independent markets grouped in the user interface.
                // We will treat each one as an independent binary market.
                Ok(None)
            }
            None => Err(anyhow!(
                "Market is multiple choice but should_answers_sum_to_one is not present: {:?}",
                market
            )),
        },
        // Various ways of implementing numeric markets
        ManifoldOutcomeType::PseudoNumeric => Ok(None),
        ManifoldOutcomeType::Number => Ok(None),
        ManifoldOutcomeType::MultiNumeric => Ok(None),
        // The remaining types are not actual markets - skip them
        ManifoldOutcomeType::Stonk => Ok(None),
        ManifoldOutcomeType::BountiedQuestion => Ok(None),
        ManifoldOutcomeType::QuadraticFunding => Ok(None),
        ManifoldOutcomeType::Poll => Ok(None),
    }
}

/// Converts Manifold bets into standard probability segments.
/// Manifold's close dates are so unreliable we don't even consider them.
pub fn build_prob_segments(raw_history: &[ManifoldBet]) -> Vec<ProbSegment> {
    // Sort the history by time.
    let mut history = raw_history.to_vec();
    history.sort_by_key(|item| item.created_time);

    let mut segments: Vec<ProbSegment> = Vec::new();

    for (i, bet) in history.iter().enumerate() {
        // The start of the segment will equal the end of the previous one unless we skipped some.
        // Err on the side of using the previous segment's end timestamp unless it's the first one.
        let start = match segments.last() {
            Some(previous_segment) => previous_segment.end,
            None => bet.created_time,
        };

        // The end of the segment will be the beginning of the next event.
        // We don't trust Manifold end dates so the last trade is the end of the market.
        let end = if i < history.len() - 1 {
            history[i + 1].created_time
        } else {
            continue;
        };

        // If the duration is exactly 0, skip.
        // Decided to keep this due to issues with how the windowing functions work.
        if start == end {
            continue;
        }

        // Get the probability after the bet was made.
        let prob = bet.prob_after;

        segments.push(ProbSegment { start, end, prob });
    }
    segments
}

fn get_traders_count(bets: &[ManifoldBet]) -> u32 {
    bets.iter()
        .map(|bet| bet.user_id.clone())
        .collect::<std::collections::HashSet<_>>()
        .len() as u32
}

fn get_volume_usd(volume: &f32) -> f32 {
    volume / MANIFOLD_EXCHANGE_RATE
}

/// Checks and returns the resolution probability for typical binary markets.
/// Resolution values can be 1, 0, or in-between for binary markets.
fn get_resolution_value_binary(market: &ManifoldMarket) -> Result<Option<f32>> {
    match &market.resolution {
        Some(resolution) => match resolution.as_str() {
            "YES" => Ok(Some(1.0)),
            "NO" => Ok(Some(0.0)),
            "MKT" => match market.resolution_probability {
                None => {
                    // Sometimes they return MKT but don't specify a probability.
                    // Currently this is the case on 4 markets:
                    //  - ESgg78HIKX8kmbjnR0Kr
                    //  - MWzNRuVifNR8NB9WVoeC
                    //  - V288UeQ98h4j3KPbceiJ
                    //  - ooiNbYz6Adqcv7eUfLPa
                    log::error!(
                        "Market {} resolved MKT with no resolution probability.",
                        market.id
                    );
                    Ok(None)
                }
                Some(res_prob) => Ok(Some(res_prob)),
            },
            _ => Err(anyhow!(
                "Market resolution value is not one of YES/NO/MKT/CANCEL: {:?}",
                market
            )),
        },
        None => Err(anyhow!("Market lacks resolution value: {:?}", market)),
    }
}

/// Manual mapping of group slugs to our standard categories.
fn get_category(tags: &[String]) -> Option<String> {
    const CATEGORIES: [(&str, &str); 58] = [
        ("118th-congress", "Politics"),
        ("2024-us-presidential-election", "Politics"),
        ("ai", "AI"),
        ("ai-alignment", "AI"),
        ("ai-safety", "AI"),
        ("arabisraeli-conflict", "Politics"),
        ("apple", "Technology"),
        ("baseball", "Sports"),
        ("basketball", "Sports"),
        ("biotech", "Science"),
        ("bitcoin", "Crypto"),
        ("celebrities", "Culture"),
        ("chatgpt", "AI"),
        ("chess", "Sports"),
        ("climate", "Climate"),
        ("crypto-speculation", "Crypto"),
        ("culture-default", "Culture"),
        ("donald-trump", "Politics"),
        ("economics-default", "Economics"),
        ("f1", "Sports"),
        ("finance", "Economics"),
        ("football", "Sports"),
        ("formula-1", "Sports"),
        ("gaming", "Culture"),
        ("gpt4-speculation", "AI"),
        ("internet", "Technology"),
        ("israelhamas-conflict-2023", "Politics"),
        ("israeli-politics", "Politics"),
        ("medicine", "Science"),
        ("movies", "Culture"),
        ("music-f213cbf1eab5", "Culture"),
        ("nfl", "Sports"),
        ("nuclear", "Science"),
        ("nuclear-risk", "Politics"),
        ("openai", "AI"),
        ("openai-9e1c42b2bb1e", "AI"),
        ("openai-crisis", "AI"),
        ("physics", "Science"),
        ("politics-default", "Politics"),
        ("programming", "Technology"),
        ("science-default", "Science"),
        ("soccer", "Sports"),
        ("space", "Science"),
        ("speaker-of-the-house-election", "Politics"),
        ("sports-default", "Sports"),
        ("startups", "Economics"),
        ("stocks", "Economics"),
        ("technical-ai-timelines", "AI"),
        ("technology-default", "Technology"),
        ("tennis", "Sports"),
        ("time-person-of-the-year", "Culture"),
        ("tv", "Culture"),
        ("uk-politics", "Politics"),
        ("ukraine", "Politics"),
        ("ukrainerussia-war", "Politics"),
        ("us-politics", "Politics"),
        ("wars", "Politics"),
        ("world-default", "Politics"),
    ];

    let category_map: HashMap<&str, &str> = CATEGORIES.iter().cloned().collect();

    tags.iter()
        .find_map(|tag| category_map.get(tag.as_str()).map(|&cat| cat.to_string()))
}
