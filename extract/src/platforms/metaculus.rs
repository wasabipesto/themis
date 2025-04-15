//! Tools to download and process markets from the Metaculus API.
//! Metaculus API docs: https://www.metaculus.com/api/

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::criteria::{calculate_all_criteria, CriterionProbability};
use crate::platforms::{MarketAndProbs, MarketResult};
use crate::{helpers, MarketError, ProbSegment, StandardMarket};

/// This is the container format we used to save items to disk earlier.
#[derive(Debug, Clone, Deserialize)]
pub struct MetaculusData {
    /// Market ID used for lookups.
    pub id: String,
    /// Timestamp the market was downloaded from the API.
    pub last_updated: DateTime<Utc>,
    // Values returned from the `/posts` endpoint.
    // Ignoring this because everything is also in `extended_data`.
    // pub post: MetaculusQuestionBasic,
    /// Values returned from the `/posts/{id}/` endpoint.
    pub details: MetaculusInfo,
}

/// A point within the aggregation history.
/// Aggregation methods are internal, we don't get detailed data.
#[derive(Debug, Clone, Deserialize)]
pub struct MetaculusAggregationHistoryPoint {
    /// Start time of history bucket.
    /// Time is in milliseconds since epoch but formatted as floating-point.
    pub start_time: f32,
    /// End time of history bucket.
    /// Time is in milliseconds since epoch but formatted as floating-point.
    pub end_time: Option<f32>,
    /// Prediction point mean.
    /// If we have to use just one, this is the point we use. (The first one?)
    pub means: Option<Vec<f32>>,
    /// Confidence interval lower bound.
    pub interval_lower_bounds: Option<Vec<f32>>,
    /// Confidence interval upper bound.
    pub interval_upper_bounds: Option<Vec<f32>>,
    /// Confidence interval center.
    pub centers: Option<Vec<f32>>,
    /// The number of forecasters who have logged predictions up to this point.
    /// This number can only increase over the history of a market.
    pub forecaster_count: u32,
}

/// Within each aggregation series, get the history as a series of buckets or
/// just the latest snapshot. Also includes score data which we don't care about.
#[derive(Debug, Clone, Deserialize)]
pub struct MetaculusAggregationTypes {
    /// Aggregation of forecast data over time.
    #[serde(default)] // default to empty vec
    pub history: Vec<MetaculusAggregationHistoryPoint>,
    // Latest aggregation of forecast data.
    pub latest: Option<MetaculusAggregationHistoryPoint>,
    // pub score_data: MetaculusAggregationScoreData,
}

/// The different aggregation types that Metaculus uses.
/// https://www.metaculus.com/notebooks/28595/104-update-updates-to-metaculus-api/
#[derive(Debug, Clone, Deserialize)]
pub struct MetaculusAggregationSeries {
    /// The official Metaculus prediction.
    pub metaculus_prediction: MetaculusAggregationTypes,
    /// The community prediction.
    pub recency_weighted: MetaculusAggregationTypes,
    /// TODO: Unknown.
    pub single_aggregation: MetaculusAggregationTypes,
    /// TODO: Unknown.
    pub unweighted: MetaculusAggregationTypes,
}

/// Possible question types from the Metaculus API.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum MetaculusQuestion {
    /// Standard binary (Yes/No) market type.
    Binary {
        /// Question ID.
        id: u64,
        /// Question title.
        /// Contains the parent question if this is in a group.
        title: String,
        /// Question description.
        /// Can be multiple lines, separated with "\n\n".
        description: String,
        /// Question resolution criteria.
        /// Can be multiple lines, separated with "\n\n".
        resolution_criteria: String,
        /// Question fine print.
        /// Can be multiple lines, separated with "\n\n".
        fine_print: String,
        /// A list of community aggregation points for this question.
        /// We will use this for our probability history.
        aggregations: MetaculusAggregationSeries,
        /// If resolved, the value of the resolution.
        resolution: Option<MetaculusResolution>,
    },
    /// Resolves to a number in a specified range.
    Numeric {
        /// Typical attributes.
        id: u64,
        title: String,
        description: String,
        resolution_criteria: String,
        fine_print: String,
        aggregations: MetaculusAggregationSeries,
        /// Can be a number (stringified) or "annulled"
        resolution: Option<String>,
    },
    /// Resolves to a date in a specified range(?).
    Date {
        /// Typical attributes.
        id: u64,
        title: String,
        description: String,
        resolution_criteria: String,
        fine_print: String,
        aggregations: MetaculusAggregationSeries,
        /// This is the resolved DateTime or "ambiguous"
        resolution: Option<String>,
    },
    /// Resolves to one of the specified options.
    MultipleChoice {
        /// Typical attributes.
        id: u64,
        title: String,
        description: String,
        resolution_criteria: String,
        fine_print: String,
        aggregations: MetaculusAggregationSeries,
        /// Possible resolution options.
        options: Vec<String>,
        /// The resolved option. Must be one of `options` or "annulled"
        resolution: Option<String>,
    },
    /// TODO
    Conditional {
        /// Typical attributes.
        id: u64,
        title: String,
        description: String,
        resolution_criteria: String,
        fine_print: String,
        aggregations: MetaculusAggregationSeries,
        /// TODO
        resolution: Option<String>,
    },
}
impl MetaculusQuestion {
    /// Get the ID from any question type
    pub fn id(&self) -> u64 {
        match self {
            Self::Binary { id, .. } => *id,
            Self::Numeric { id, .. } => *id,
            Self::Date { id, .. } => *id,
            Self::MultipleChoice { id, .. } => *id,
            Self::Conditional { id, .. } => *id,
        }
    }
}

/// Struct for a group of questions.
#[derive(Debug, Clone, Deserialize)]
pub struct MetaculusGroupOfQuestions {
    pub questions: Vec<MetaculusQuestion>,
}

/// Info on each project, tag, or category applied to the question.
/// These items have additional attributes but I'm grouping them up.
#[derive(Debug, Clone, Deserialize)]
pub struct MetaculusProjectInfo {
    /// The item's name.
    pub name: String,
    /// The item's URL slug.
    pub slug: Option<String>,
}

/// Info on the projects, tags, and categories applied to the question.
/// Used for deriving overall category.
#[derive(Debug, Clone, Deserialize)]
pub struct MetaculusProjects {
    pub default_project: MetaculusProjectInfo,
    #[serde(default)]
    pub question_series: Vec<MetaculusProjectInfo>,
    #[serde(default)]
    pub site_main: Vec<MetaculusProjectInfo>,
    #[serde(default)]
    pub tournament: Vec<MetaculusProjectInfo>,
    #[serde(default)]
    pub category: Vec<MetaculusProjectInfo>,
    #[serde(default)]
    pub tags: Vec<MetaculusProjectInfo>,
}

/// What stage of the market life-cycle this is in.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MetaculusStatus {
    Draft,
    Pending,
    Approved,
    Rejected,
    Upcoming,
    Open,
    Closed,
    /// Resolved is the status used after everything is complete.
    /// We will filter to only finalized markets for the database.
    Resolved,
}

/// Resolution states for a question.
/// Essentially Yes, No, or Cancel.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetaculusResolution {
    /// Resolved positively.
    Yes,
    /// Resolved negatively.
    No,
    /// Canceled due to issues with the question interpretation.
    Ambiguous,
    /// Canceled due to other issues or because the premise is no longer valid.
    Annulled,
}

/// Values returned from the `/questions/{id}` endpoint.
/// https://www.metaculus.com/api/
#[derive(Debug, Clone, Deserialize)]
pub struct MetaculusInfo {
    /// The unique ID of this question.
    /// Used to built the site URL: https://www.metaculus.com/questions/{id}
    pub id: u32,
    /// Question text, also used as title.
    pub title: String,
    /// Shortened version of the question title.
    pub short_title: String,
    /// The URL slug for this question.
    /// Can optionally be added to the end of the URL (see ID above).
    pub slug: String,

    /// Information about the specific question.
    pub question: Option<MetaculusQuestion>,
    /// Information about a group of questions. Only for GroupOfMarkets type.
    pub group_of_questions: Option<MetaculusGroupOfQuestions>,
    /// Info on the projects, tags, and categories applied to the question.
    pub projects: MetaculusProjects,

    /// The question trading status.
    pub status: MetaculusStatus,
    /// Whether the question is resolved yet.
    /// Redundant with `status`.
    pub resolved: bool,

    /// The question author's user ID.
    pub author_id: u32,
    /// The question author's username.
    pub author_username: String,

    /// Number of comments.
    pub comment_count: u32,
    /// Number of forecasts.
    pub forecasts_count: u32,
    /// Number of forecasters.
    pub nr_forecasters: u32,

    /// Moment the question was created. Usually set in a draft or review state.
    pub created_at: DateTime<Utc>,
    /// If published, the moment the question was published.
    /// Usually questions are not open for trading at this point.
    pub published_at: Option<DateTime<Utc>>,
    /// If open, the moment the question was opened for trading.
    pub open_time: Option<DateTime<Utc>>,
    /// Moment of most recent edit to the question.
    pub edited_at: Option<DateTime<Utc>>,
    /// If closed, the most recent close time.
    pub actual_close_time: Option<DateTime<Utc>>,
    /// If resolved, the resolution time.
    pub resolution_set_time: Option<DateTime<Utc>>,
}

/// Convert data pulled from the API into a standardized market item or an error.
/// Note: This is not a 1:1 conversion because some inputs contain multiple
/// discrete markets, and each of those have their own histories.
pub fn standardize(input: &MetaculusData) -> MarketResult<Vec<MarketAndProbs>> {
    // Get market ID. Construct from platform slug and ID within platform.
    let platform_slug = "metaculus".to_string();
    let market_id = format!("{}:{}", platform_slug, input.details.id);

    // Skip markets that have not resolved
    if !input.details.resolved || input.details.status != MetaculusStatus::Resolved {
        return Err(MarketError::MarketNotResolved(market_id.to_owned()));
    }

    if let Some(question) = &input.details.question {
        // If there is a single question, process it
        let standard_market = standardize_single(question, &input.details, market_id)?;
        Ok(vec![standard_market])
    } else if let Some(questions) = &input.details.group_of_questions {
        // If there is a group of questions, process them
        let mut markets = Vec::new();
        for question in &questions.questions {
            // Append question ID to market ID
            let individual_market_id = format!("{}:{}", market_id, question.id());
            let standard_market =
                standardize_single(question, &input.details, individual_market_id)?;
            markets.push(standard_market);
        }
        Ok(markets)
    } else {
        // If there are no questions or groups, exit
        Err(MarketError::NotAMarket(market_id.to_owned()))
    }
}

/// Standardize a single question
fn standardize_single(
    question: &MetaculusQuestion,
    details: &MetaculusInfo,
    market_id: String,
) -> MarketResult<MarketAndProbs> {
    let platform_slug = "metaculus".to_string();

    match question {
        MetaculusQuestion::Binary {
            title,
            description,
            resolution_criteria,
            fine_print,
            aggregations,
            resolution,
            ..
        } => {
            // Get probability segments. If there are none then skip.
            // Using recency_weighted (community prediction) here, may change in the future.
            // Since this is binary, get the first (and only) prob in the set.
            let probs = build_prob_segments(&aggregations.recency_weighted.history, 0)
                .map_err(|e| MarketError::ProcessingError(market_id.to_owned(), e.to_string()))?;
            if probs.is_empty() {
                return Err(MarketError::NoMarketTrades(market_id.to_owned()));
            }

            // Validate probability segments and collate into daily prob segments.
            helpers::validate_prob_segments(&probs).map_err(|e| {
                MarketError::InvalidMarketTrades(market_id.to_owned(), e.to_string())
            })?;
            let daily_probabilities = helpers::get_daily_probabilities(&probs, &market_id)
                .map_err(|e| MarketError::ProcessingError(market_id.to_owned(), e.to_string()))?;
            let criterion_probabilities: Vec<CriterionProbability> =
                calculate_all_criteria(&market_id, &probs).map_err(|e| {
                    MarketError::ProcessingError(market_id.to_owned(), e.to_string())
                })?;

            // Get resolution value.
            let resolution_value = match resolution {
                Some(MetaculusResolution::Yes) => 1.0,
                Some(MetaculusResolution::No) => 0.0,
                Some(MetaculusResolution::Ambiguous) => {
                    return Err(MarketError::MarketCancelled(market_id.to_owned()))
                }
                Some(MetaculusResolution::Annulled) => {
                    return Err(MarketError::MarketCancelled(market_id.to_owned()))
                }
                None => {
                    return Err(MarketError::DataInvalid(
                        market_id.to_owned(),
                        "Market is resolved but missing resolution value.".to_string(),
                    ))
                }
            };

            // Build standard market item.
            let market = create_standard_market(
                market_id,
                title,
                format_market_url(details.id),
                format_market_description(description, resolution_criteria, fine_print),
                platform_slug,
                &probs,
                details.nr_forecasters,
                resolution_value,
            )?;
            Ok(MarketAndProbs {
                market,
                daily_probabilities,
                criterion_probabilities,
            })
        }
        MetaculusQuestion::Numeric { .. } => Err(MarketError::MarketTypeNotImplemented(
            market_id.to_owned(),
            "Metaculus::Numeric".to_string(),
        )),
        MetaculusQuestion::Date { .. } => Err(MarketError::MarketTypeNotImplemented(
            market_id.to_owned(),
            "Metaculus::Date".to_string(),
        )),
        MetaculusQuestion::MultipleChoice {
            title,
            description,
            resolution_criteria,
            fine_print,
            aggregations,
            options,
            resolution,
            ..
        } => {
            // Since we only allow markets where one answer is selected and only refer to the
            // winning answer, the resolution will always be YES.
            let resolution_value = 1.0;

            // Get the resolution value.
            let resolved_option = match resolution {
                Some(res) => res,
                None => {
                    return Err(MarketError::DataInvalid(
                        market_id.to_owned(),
                        "Multiple choice question lacks resolution value".to_string(),
                    ))
                }
            };

            // Skip if resolution is annulled.
            if resolved_option == "annulled" {
                return Err(MarketError::MarketCancelled(market_id.to_owned()));
            }

            // Append the tracked outcome to the market title so we know which side we're tracking.
            let title = format!("{} | {}", title, resolved_option);

            // Get index of resolved option for prob lookup.
            let index = options
                .iter()
                .position(|option| option == resolved_option)
                .ok_or_else(|| {
                    MarketError::DataInvalid(
                        market_id.to_owned(),
                        "Multiple choice resolution {resolved_option} not found in options."
                            .to_string(),
                    )
                })?;

            // Get probability segments. If there are none then skip.
            // Using recency_weighted (community prediction) here, may change in the future.
            // Since this is multiple choice, we need to use the index of the resolved option.
            let probs = build_prob_segments(&aggregations.recency_weighted.history, index)
                .map_err(|e| MarketError::ProcessingError(market_id.to_owned(), e.to_string()))?;
            if probs.is_empty() {
                return Err(MarketError::NoMarketTrades(market_id.to_owned()));
            }

            // Validate probability segments and collate into daily prob segments.
            helpers::validate_prob_segments(&probs).map_err(|e| {
                MarketError::InvalidMarketTrades(market_id.to_owned(), e.to_string())
            })?;
            let daily_probabilities = helpers::get_daily_probabilities(&probs, &market_id)
                .map_err(|e| MarketError::ProcessingError(market_id.to_owned(), e.to_string()))?;
            let criterion_probabilities: Vec<CriterionProbability> =
                calculate_all_criteria(&market_id, &probs).map_err(|e| {
                    MarketError::ProcessingError(market_id.to_owned(), e.to_string())
                })?;

            // Build standard market item.
            let market = create_standard_market(
                market_id,
                &title,
                format_market_url(details.id),
                format_market_description(description, resolution_criteria, fine_print),
                platform_slug,
                &probs,
                details.nr_forecasters,
                resolution_value,
            )?;
            Ok(MarketAndProbs {
                market,
                daily_probabilities,
                criterion_probabilities,
            })
        }
        MetaculusQuestion::Conditional { .. } => Err(MarketError::MarketTypeNotImplemented(
            market_id.to_owned(),
            "Metaculus::Conditional".to_string(),
        )),
    }
}

/// Creates a standardized market description from Metaculus question fields
fn format_market_description(
    description: &str,
    resolution_criteria: &str,
    fine_print: &str,
) -> String {
    format!(
        "{}\n\n{}\n\n{}",
        description, resolution_criteria, fine_print
    )
}

/// Creates a standardized market URL from a Metaculus question ID
fn format_market_url(id: u32) -> String {
    format!("https://www.metaculus.com/questions/{}", id)
}

#[allow(clippy::too_many_arguments)]
fn create_standard_market(
    market_id: String,
    title: &str,
    url: String,
    description: String,
    platform_slug: String,
    probs: &[ProbSegment],
    nr_forecasters: u32,
    resolution: f32,
) -> Result<StandardMarket, MarketError> {
    // We only consider the market to be open while there are actual probabilities.
    let start = probs.first().unwrap().start;
    let end = probs.last().unwrap().end;

    Ok(StandardMarket {
        id: market_id.to_owned(),
        title: title.to_owned(),
        url,
        description,
        platform_slug,
        category_slug: None, // TODO
        open_datetime: start,
        close_datetime: end,
        traders_count: Some(nr_forecasters),
        volume_usd: None, // Metaculus does not use volume.
        duration_days: helpers::get_market_duration(start, end)
            .map_err(|e| MarketError::ProcessingError(market_id.to_owned(), e.to_string()))?,
        resolution,
    })
}

/// Converts Metaculus aggregated history points into standard probability segments.
pub fn build_prob_segments(
    raw_history: &[MetaculusAggregationHistoryPoint],
    index: usize,
) -> Result<Vec<ProbSegment>> {
    // Sort the history by time.
    let mut history = raw_history.to_vec();
    history.sort_by_key(|item| item.start_time as u32);

    let mut segments: Vec<ProbSegment> = Vec::new();
    for item in history {
        // Get the start and end dates
        let start = DateTime::from_timestamp(item.start_time as i64, 0)
            .with_context(|| "Could not create datetime from history start point")?;
        let end = match item.end_time {
            Some(end_time) => DateTime::from_timestamp(end_time as i64, 0)
                .with_context(|| "Could not create datetime from history end point")?,
            None => {
                // We used to extend the last segment to the end of the market, but with question groups that is more difficult.
                // We would have to dig into each question to find that question's end time to do this right.
                // Now we just ignore any segments that don't have an end time.
                continue;
            }
        };

        // If the duration is exactly 0, skip.
        if end == start {
            continue;
        }

        // If the start of this segment is prior to the end of the previous one, skip it.
        // There are some overlapping items but if we filter them out everything seems to work out.
        // Relevant Question IDs:
        //   1640, 2599, 2616, 2788, 3238, 3682, 5174, 11274, 11528, 18177, 20533, 20694,
        //   20747, 20748, 20751, 20762, 20766, 20768, 20771, 20774, 20775, 20783, 20789,
        //   24020, 30251, 30297
        if let Some(previous_segment) = segments.last() {
            if previous_segment.end > start {
                continue;
            }
        }

        // Get the means list and check it. We'll use the first listed probability.
        // For binary markets there is only ever one per aggregation.
        // For multiple-choice there is one per option, indexed the same as the options.
        let means = item.means.unwrap_or_default();
        let prob = match means.get(index) {
            None => {
                return Err(anyhow!(
                    "Could not get index {index} in means list {:?}.",
                    means
                ))
            }
            Some(prob) => prob.to_owned(),
        };

        segments.push(ProbSegment { start, end, prob });
    }
    Ok(segments)
}
