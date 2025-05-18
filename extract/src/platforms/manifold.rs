//! Tools to download and process markets from the Manifold API.
//! Manifold API docs: https://docs.manifold.markets/api
//! Source code: https://github.com/manifoldmarkets/manifold/tree/main/backend/api/src

use anyhow::{anyhow, Result};
use chrono::serde::{ts_milliseconds, ts_milliseconds_option};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::criteria::{calculate_all_criteria, CriterionProbability};
use crate::platforms::{MarketAndProbs, MarketResult};
use crate::{helpers, MarketError, ProbSegment, StandardMarket};

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
    /// Date type, also like a re-skinned multiple choice market.
    Date,
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
    /// How the option resolved
    pub resolution: Option<String>,
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
    /// Public URL to the ai-generated or user-uploaded hero image.
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
    /// This may be more than `amount` if the entire order wasn't filled.
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

/// Convert data pulled from the API into a standardized market item or an error.
/// Note: This is not a 1:1 conversion because some inputs contain multiple
/// discrete markets, and each of those have their own histories.
pub fn standardize(input: &ManifoldData) -> MarketResult<Vec<MarketAndProbs>> {
    // Get market ID. Construct from platform slug and ID within platform.
    let platform_slug = "manifold".to_string();
    let market_id = format!("{}:{}", platform_slug, input.full_market.id);

    // Skip markets that have not resolved
    if !input.full_market.is_resolved {
        return Err(MarketError::MarketNotResolved(market_id));
    }
    // Skip markets that were canceled
    if input.full_market.resolution == Some("CANCEL".to_string()) {
        return Err(MarketError::MarketCancelled(market_id.to_owned()));
    }

    // Reference full market response
    let market = &input.full_market;

    // Convert based on market type
    match market.outcome_type {
        // Typical single binary market
        ManifoldOutcomeType::Binary => {
            // Get probability segments. If there are none then skip.
            let probs = build_prob_segments(&input.bets);
            if probs.is_empty() {
                return Err(MarketError::NoMarketTrades(market_id.to_owned()));
            }

            // Validate probability segments and collate into prob segments.
            helpers::validate_prob_segments(&probs).map_err(|e| {
                MarketError::InvalidMarketTrades(market_id.to_owned(), e.to_string())
            })?;
            let daily_probabilities = helpers::get_daily_probabilities(&probs, &market_id)
                .map_err(|e| MarketError::ProcessingError(market_id.to_owned(), e.to_string()))?;
            let criterion_probabilities: Vec<CriterionProbability> =
                calculate_all_criteria(&market_id, &probs).map_err(|e| {
                    MarketError::ProcessingError(market_id.to_owned(), e.to_string())
                })?;

            // We only consider the market to be open while there are actual probabilities.
            let start = probs.first().unwrap().start;
            let end = probs.last().unwrap().end;

            // Build standard market item.
            let market = StandardMarket {
                id: market_id.to_owned(),
                title: market.question.clone(),
                url: market.url.to_owned(),
                description: market.text_description.clone(),
                platform_slug,
                category_slug: get_category(&market.group_slugs),
                open_datetime: start,
                close_datetime: end,
                traders_count: Some(get_traders_count(&input.bets)),
                volume_usd: Some(market.volume / MANIFOLD_EXCHANGE_RATE),
                duration_days: helpers::get_market_duration(start, end).map_err(|e| {
                    MarketError::ProcessingError(market_id.to_owned(), e.to_string())
                })?,
                resolution: match get_resolution_value_binary(
                    &market.resolution,
                    &market.resolution_probability,
                    true,
                ) {
                    Ok(Some(res)) => res,
                    Ok(None) => return Err(MarketError::MarketCancelled(market_id.to_owned())),
                    Err(e) => {
                        return Err(MarketError::ProcessingError(
                            market_id.to_owned(),
                            e.to_string(),
                        ));
                    }
                },
            };
            Ok(vec![MarketAndProbs {
                market,
                daily_probabilities,
                criterion_probabilities,
            }])
        }
        // Multiple choice markets
        ManifoldOutcomeType::MultipleChoice => match market.should_answers_sum_to_one {
            Some(true) => {
                // Market has many potential outcomes but prices are automatically arbitraged
                // in order to keep everything summed to 100%.

                // Make sure answers are present
                let answers = market.answers.to_owned().ok_or(MarketError::DataInvalid(
                    market_id.to_owned(),
                    "Multiple choice market does not have answers.".to_string(),
                ))?;

                // Either one resolution is picked (most common), or multiple are picked and their
                // winnings are split proportionally.
                let resolved_answer = match &market.resolution {
                    Some(resolution) => {
                        if resolution == "CHOOSE_MULTIPLE" || resolution == "MKT" {
                            // This is currently not implemented. I'm not exactly sure how we would do this.
                            return Err(MarketError::MarketTypeNotImplemented(
                                market_id.to_owned(),
                                "Manifold::MultipleChoice::ResolvedToMultiple".to_string(),
                            ));
                        } else {
                            answers
                                .iter()
                                .find(|answer| &answer.id == resolution)
                                .ok_or(MarketError::DataInvalid(market_id.to_owned(),
                                "Market {market_id}: No answer found matching the resolution ID".to_string()))?
                                .to_owned()
                        }
                    }
                    None => {
                        return Err(MarketError::DataInvalid(
                            market_id.to_owned(),
                            "Market lacks resolution value.".to_string(),
                        ))
                    }
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
                    return Err(MarketError::NoMarketTrades(market_id.to_owned()));
                }

                // Validate probability segments and collate into daily prob segments.
                helpers::validate_prob_segments(&probs).map_err(|e| {
                    MarketError::InvalidMarketTrades(market_id.to_owned(), e.to_string())
                })?;
                let daily_probabilities = helpers::get_daily_probabilities(&probs, &market_id)
                    .map_err(|e| {
                        MarketError::ProcessingError(market_id.to_owned(), e.to_string())
                    })?;
                let criterion_probabilities: Vec<CriterionProbability> =
                    calculate_all_criteria(&market_id, &probs).map_err(|e| {
                        MarketError::ProcessingError(market_id.to_owned(), e.to_string())
                    })?;

                // We only consider the market to be open while there are actual probabilities.
                let start = probs.first().unwrap().start;
                let end = probs.last().unwrap().end;

                // Build standard market item.
                let market = StandardMarket {
                    id: market_id.to_owned(),
                    title,
                    url: market.url.to_owned(),
                    description: market.text_description.clone(),
                    platform_slug,
                    category_slug: get_category(&market.group_slugs),
                    open_datetime: start,
                    close_datetime: end,
                    traders_count: Some(get_traders_count(&bets)),
                    volume_usd: Some(market.volume / MANIFOLD_EXCHANGE_RATE),
                    duration_days: helpers::get_market_duration(start, end).map_err(|e| {
                        MarketError::ProcessingError(market_id.to_owned(), e.to_string())
                    })?,
                    resolution,
                };
                Ok(vec![MarketAndProbs {
                    market,
                    daily_probabilities,
                    criterion_probabilities,
                }])
            }
            Some(false) => {
                // Collection of independent markets grouped in the user interface.
                // We will treat each one as an independent binary market.

                // Make sure answers are present
                let answers = market.answers.to_owned().ok_or(MarketError::DataInvalid(
                    market_id.to_owned(),
                    "Multiple choice market does not have answers".to_string(),
                ))?;

                let mut result = Vec::new();
                for answer in answers {
                    // Override market ID. Construct from platform slug, market ID within platform, and answer ID within market.
                    let market_id = format!("{}:{}:{}", &platform_slug, market.id, answer.id);

                    // Determine the resolution
                    let resolution = match get_resolution_value_binary(
                        &answer.resolution,
                        &answer.resolution_probability,
                        false,
                    ) {
                        Ok(Some(res)) => res,
                        Ok(None) => return Err(MarketError::MarketCancelled(market_id.to_owned())),
                        Err(e) => {
                            return Err(MarketError::ProcessingError(
                                market_id.to_owned(),
                                e.to_string(),
                            ));
                        }
                    };

                    // Append the tracked outcome to the market title so we know which side we're tracking.
                    let title = format!("{} | {}", market.question, answer.text);

                    // Filter bets for the resolved answer
                    let bets: Vec<ManifoldBet> = input
                        .bets
                        .iter()
                        .filter(|bet| bet.answer_id.as_ref() == Some(&answer.id))
                        .cloned()
                        .collect();

                    // Get probability segments. If there are none then skip.
                    let probs = build_prob_segments(&bets);
                    if probs.is_empty() {
                        return Err(MarketError::NoMarketTrades(market_id.to_owned()));
                    }

                    // Validate probability segments and collate into daily prob segments.
                    helpers::validate_prob_segments(&probs).map_err(|e| {
                        MarketError::InvalidMarketTrades(market_id.to_owned(), e.to_string())
                    })?;
                    let daily_probabilities = helpers::get_daily_probabilities(&probs, &market_id)
                        .map_err(|e| {
                            MarketError::ProcessingError(market_id.to_owned(), e.to_string())
                        })?;
                    let criterion_probabilities: Vec<CriterionProbability> =
                        calculate_all_criteria(&market_id, &probs).map_err(|e| {
                            MarketError::ProcessingError(market_id.to_owned(), e.to_string())
                        })?;

                    // We only consider the market to be open while there are actual probabilities.
                    let start = probs.first().unwrap().start;
                    let end = probs.last().unwrap().end;

                    // The market volume counts bets for all answers, so we need to calculate based
                    // on the ones for this answer.
                    let volume_usd = get_volume_from_bets(&bets);

                    // Build standard market item.
                    let market = StandardMarket {
                        id: market_id.to_owned(),
                        title,
                        url: market.url.to_owned(),
                        description: market.text_description.clone(),
                        platform_slug: platform_slug.to_owned(),
                        category_slug: get_category(&market.group_slugs),
                        open_datetime: start,
                        close_datetime: end,
                        traders_count: Some(get_traders_count(&bets)),
                        volume_usd: Some(volume_usd),
                        duration_days: helpers::get_market_duration(start, end).map_err(|e| {
                            MarketError::ProcessingError(market_id.to_owned(), e.to_string())
                        })?,
                        resolution,
                    };
                    result.push(MarketAndProbs {
                        market,
                        daily_probabilities,
                        criterion_probabilities,
                    });
                }
                Ok(result)
            }
            None => Err(MarketError::DataInvalid(
                market_id.to_owned(),
                "Market is multiple choice but should_answers_sum_to_one is not present."
                    .to_string(),
            )),
        },
        // Various ways of implementing numeric markets
        // Not urgent to implement but would like to have for the future
        ManifoldOutcomeType::PseudoNumeric => Err(MarketError::MarketTypeNotImplemented(
            market_id.to_owned(),
            "Manifold::PseudoNumeric".to_string(),
        )),
        ManifoldOutcomeType::Number => Err(MarketError::MarketTypeNotImplemented(
            market_id.to_owned(),
            "Manifold::Number".to_string(),
        )),
        ManifoldOutcomeType::MultiNumeric => Err(MarketError::MarketTypeNotImplemented(
            market_id.to_owned(),
            "Manifold::MultiNumeric".to_string(),
        )),
        ManifoldOutcomeType::Date => Err(MarketError::MarketTypeNotImplemented(
            market_id.to_owned(),
            "Manifold::Date".to_string(),
        )),
        // The remaining types are not actual markets - skip them
        ManifoldOutcomeType::Stonk => Err(MarketError::NotAMarket(market_id.to_owned())),
        ManifoldOutcomeType::BountiedQuestion => Err(MarketError::NotAMarket(market_id.to_owned())),
        ManifoldOutcomeType::QuadraticFunding => Err(MarketError::NotAMarket(market_id.to_owned())),
        ManifoldOutcomeType::Poll => Err(MarketError::NotAMarket(market_id.to_owned())),
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

/// Get the number of unique traders from the bet log.
fn get_traders_count(bets: &[ManifoldBet]) -> u32 {
    bets.iter()
        .map(|bet| bet.user_id.clone())
        .collect::<std::collections::HashSet<_>>()
        .len() as u32
}

/// Get the total trade volume from the bet log.
fn get_volume_from_bets(bets: &[ManifoldBet]) -> f32 {
    bets.iter().map(|bet| bet.amount.abs()).sum()
}

/// Checks and returns the resolution probability for typical binary markets.
/// Resolution values can be 1, 0, or in-between for binary markets.
/// If fail_on_missing is set false, then don't error on missing resolution values.
/// That may happen for e.g. multiple-choice markets where some items are not resolved.
fn get_resolution_value_binary(
    resolution: &Option<String>,
    resolution_probability: &Option<f32>,
    fail_on_missing: bool,
) -> Result<Option<f32>> {
    match &resolution {
        Some(res) => match res.as_str() {
            "YES" => Ok(Some(1.0)),
            "NO" => Ok(Some(0.0)),
            "MKT" => match resolution_probability {
                None => {
                    // Sometimes they return MKT but don't specify a probability.
                    // Currently this is the case on 4 markets:
                    //  - ESgg78HIKX8kmbjnR0Kr
                    //  - MWzNRuVifNR8NB9WVoeC
                    //  - V288UeQ98h4j3KPbceiJ
                    //  - ooiNbYz6Adqcv7eUfLPa
                    Err(anyhow!("Resolution is MKT but probability is missing.",))
                }
                Some(res_prob) => Ok(Some(res_prob.to_owned())),
            },
            "CANCEL" => Ok(None),
            _ => Err(anyhow!("Resolution value is not one of YES/NO/MKT/CANCEL",)),
        },
        None => match fail_on_missing {
            true => Err(anyhow!("Resolution value is missing.")),
            false => Ok(None),
        },
    }
}

/// Manual mapping of group slugs to our standard categories.
fn get_category(tags: &[String]) -> Option<String> {
    const CATEGORIES: [(&str, &str); 58] = [
        ("118th-congress", "politics"),
        ("2024-us-presidential-election", "politics"),
        ("ai", "technology"),
        ("ai-alignment", "technology"),
        ("ai-safety", "technology"),
        ("arabisraeli-conflict", "politics"),
        ("apple", "technology"),
        ("baseball", "sports"),
        ("basketball", "sports"),
        ("biotech", "science"),
        ("bitcoin", "economics"),
        ("celebrities", "culture"),
        ("chatgpt", "technology"),
        ("chess", "sports"),
        ("climate", "science"),
        ("crypto-speculation", "economics"),
        ("culture-default", "culture"),
        ("donald-trump", "politics"),
        ("economics-default", "economics"),
        ("f1", "sports"),
        ("finance", "economics"),
        ("football", "sports"),
        ("formula-1", "sports"),
        ("gaming", "culture"),
        ("gpt4-speculation", "technology"),
        ("internet", "technology"),
        ("israelhamas-conflict-2023", "politics"),
        ("israeli-politics", "politics"),
        ("medicine", "science"),
        ("movies", "culture"),
        ("music-f213cbf1eab5", "culture"),
        ("nfl", "sports"),
        ("nuclear", "science"),
        ("nuclear-risk", "politics"),
        ("openai", "technology"),
        ("openai-9e1c42b2bb1e", "technology"),
        ("openai-crisis", "technology"),
        ("physics", "science"),
        ("politics-default", "politics"),
        ("programming", "technology"),
        ("science-default", "science"),
        ("soccer", "sports"),
        ("space", "science"),
        ("speaker-of-the-house-election", "politics"),
        ("sports-default", "sports"),
        ("startups", "economics"),
        ("stocks", "economics"),
        ("technical-ai-timelines", "technology"),
        ("technology-default", "technology"),
        ("tennis", "sports"),
        ("time-person-of-the-year", "culture"),
        ("tv", "culture"),
        ("uk-politics", "politics"),
        ("ukraine", "politics"),
        ("ukrainerussia-war", "politics"),
        ("us-politics", "politics"),
        ("wars", "politics"),
        ("world-default", "politics"),
    ];

    let category_map: HashMap<&str, &str> = CATEGORIES.iter().cloned().collect();

    tags.iter()
        .find_map(|tag| category_map.get(tag.as_str()).map(|&cat| cat.to_string()))
}
