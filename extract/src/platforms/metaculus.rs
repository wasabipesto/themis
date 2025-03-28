//! Tools to download and process markets from the Metaculus API.
//! Metaculus API docs: https://www.metaculus.com/api/

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::{helpers, MarketAndProbs, MarketError, MarketResult, ProbSegment, StandardMarket};

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

/// Possible question types from the
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum MetaculusQuestion {
    /// Standard binary (Yes/No) market type.
    Binary {
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
        description: String,
        resolution_criteria: String,
        fine_print: String,
        aggregations: MetaculusAggregationSeries,
        /// TODO
        resolution: Option<String>,
    },
}

/// A group of questions.
#[derive(Debug, Clone, Deserialize)]
pub struct MetaculusGroupOfQuestions {
    pub questions: Vec<MetaculusQuestion>,
}

/// Info on each project associated with the question.
#[derive(Debug, Clone, Deserialize)]
pub struct MetaculusProject {
    /// The project's ID.
    pub id: u32,
    /// The project's name.
    pub name: String,
}

/// Info on each project, tag, or category applied to the question.
/// These items have additional attributes but I'm grouping them up.
#[derive(Debug, Clone, Deserialize)]
pub struct MetaculusGroup {
    /// The item's name.
    pub name: String,
    /// The item's URL slug.
    pub slug: Option<String>,
}

/// Info on the projects, tags, and categories applied to the question.
/// Used for deriving overall category.
#[derive(Debug, Clone, Deserialize)]
pub struct MetaculusGroups {
    pub default_project: MetaculusGroup,
    #[serde(default)]
    pub question_series: Vec<MetaculusGroup>,
    #[serde(default)]
    pub site_main: Vec<MetaculusGroup>,
    #[serde(default)]
    pub tournament: Vec<MetaculusGroup>,
    #[serde(default)]
    pub category: Vec<MetaculusGroup>,
    #[serde(default)]
    pub tags: Vec<MetaculusGroup>,
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
    pub projects: MetaculusGroups,

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

    // Convert based on market type
    match &input.details.question {
        Some(MetaculusQuestion::Binary {
            description,
            resolution_criteria,
            fine_print,
            aggregations,
            resolution,
        }) => {
            // Get probability segments. If there are none then skip.
            // Using recency_weighted (community prediction) here, may change in the future.
            // Since this is binary, get the first (and only) prob in the set.
            let probs = build_prob_segments(
                &aggregations.recency_weighted.history,
                0,
                &input.details.actual_close_time,
            )
            .with_context(|| format!("Error building probability segments. ID: {market_id}"))
            .map_err(|e| MarketError::ProcessingError(market_id.to_owned(), e.to_string()))?;
            if probs.is_empty() {
                return Err(MarketError::NoMarketTrades(market_id.to_owned()));
            }

            // Validate probability segments and collate into daily prob segments.
            if let Err(e) = helpers::validate_prob_segments(&probs) {
                return Err(MarketError::InvalidMarketTrades(
                    market_id.to_owned(),
                    e.to_string(),
                ));
            }
            let daily_probabilities = helpers::get_daily_probabilities(&probs, &market_id)
                .map_err(|e| MarketError::ProcessingError(market_id.to_owned(), e.to_string()))?;

            // We only consider the market to be open while there are actual probabilities.
            let start = probs.first().unwrap().start;
            let end = probs.last().unwrap().end;

            // Get resolution value.
            let resolution = match resolution {
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
            let market = StandardMarket {
                id: market_id.clone(),
                title: input.details.title.clone(),
                url: format!("https://www.metaculus.com/questions/{}", input.details.id),
                description: format!(
                    "{}\n\n{}\n\n{}",
                    description.clone(),
                    resolution_criteria.clone(),
                    fine_print.clone(),
                ),
                platform_slug,
                category_slug: None, // TODO
                open_datetime: start,
                close_datetime: end,
                traders_count: Some(input.details.nr_forecasters),
                volume_usd: None, // Metaculus does not use volume.
                duration_days: helpers::get_market_duration(start, end).map_err(|e| {
                    MarketError::ProcessingError(market_id.to_owned(), e.to_string())
                })?,
                prob_at_midpoint: helpers::get_prob_at_midpoint(&probs, start, end).map_err(
                    |e| MarketError::ProcessingError(market_id.to_owned(), e.to_string()),
                )?,
                prob_time_avg: helpers::get_prob_time_avg(&probs, start, end).map_err(|e| {
                    MarketError::ProcessingError(market_id.to_owned(), e.to_string())
                })?,
                resolution,
            };
            Ok(vec![MarketAndProbs {
                market,
                daily_probabilities,
            }])
        }
        // TODO: Implement other types
        Some(MetaculusQuestion::Numeric {
            description: _,
            resolution_criteria: _,
            fine_print: _,
            aggregations: _,
            resolution: _,
        }) => Err(MarketError::MarketTypeNotImplemented(
            market_id.to_owned(),
            "Metaculus::Numeric".to_string(),
        )),
        Some(MetaculusQuestion::Date {
            description: _,
            resolution_criteria: _,
            fine_print: _,
            aggregations: _,
            resolution: _,
        }) => Err(MarketError::MarketTypeNotImplemented(
            market_id.to_owned(),
            "Metaculus::Date".to_string(),
        )),
        Some(MetaculusQuestion::MultipleChoice {
            description,
            resolution_criteria,
            fine_print,
            aggregations,
            options,
            resolution,
        }) => {
            // Get market ID. Construct from platform slug and ID within platform.
            let market_id = format!("{}:{}", platform_slug, input.details.id);

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
            let title = format!("{} | {}", input.details.title, resolved_option);

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
            let probs = build_prob_segments(
                &aggregations.recency_weighted.history,
                index,
                &input.details.actual_close_time,
            )
            .map_err(|e| MarketError::ProcessingError(market_id.to_owned(), e.to_string()))?;
            if probs.is_empty() {
                return Err(MarketError::NoMarketTrades(market_id.to_owned()));
            }

            // Validate probability segments and collate into daily prob segments.
            if let Err(e) = helpers::validate_prob_segments(&probs) {
                return Err(MarketError::InvalidMarketTrades(
                    market_id.to_owned(),
                    e.to_string(),
                ));
            }
            let daily_probabilities = helpers::get_daily_probabilities(&probs, &market_id)
                .map_err(|e| MarketError::ProcessingError(market_id.to_owned(), e.to_string()))?;

            // We only consider the market to be open while there are actual probabilities.
            let start = probs.first().unwrap().start;
            let end = probs.last().unwrap().end;

            // Build standard market item.
            let market = StandardMarket {
                id: market_id.clone(),
                title,
                url: format!("https://www.metaculus.com/questions/{}", input.details.id),
                description: format!(
                    "{}\n\n{}\n\n{}",
                    description.clone(),
                    resolution_criteria.clone(),
                    fine_print.clone(),
                ),
                platform_slug,
                category_slug: None, // TODO
                open_datetime: start,
                close_datetime: end,
                traders_count: Some(input.details.nr_forecasters),
                volume_usd: None, // Metaculus does not use volume.
                duration_days: helpers::get_market_duration(start, end).map_err(|e| {
                    MarketError::ProcessingError(market_id.to_owned(), e.to_string())
                })?,
                prob_at_midpoint: helpers::get_prob_at_midpoint(&probs, start, end).map_err(
                    |e| MarketError::ProcessingError(market_id.to_owned(), e.to_string()),
                )?,
                prob_time_avg: helpers::get_prob_time_avg(&probs, start, end).map_err(|e| {
                    MarketError::ProcessingError(market_id.to_owned(), e.to_string())
                })?,
                resolution: resolution_value,
            };
            Ok(vec![MarketAndProbs {
                market,
                daily_probabilities,
            }])
        }
        Some(MetaculusQuestion::Conditional {
            description: _,
            resolution_criteria: _,
            fine_print: _,
            resolution: _,
            aggregations: _,
        }) => Err(MarketError::MarketTypeNotImplemented(
            market_id.to_owned(),
            "Metaculus::Conditional".to_string(),
        )),
        None => Err(MarketError::MarketTypeNotImplemented(
            market_id.to_owned(),
            "Metaculus::GroupOfQuestions".to_string(),
        )),
    }
}

/// Converts Metaculus aggregated history points into standard probability segments.
pub fn build_prob_segments(
    raw_history: &[MetaculusAggregationHistoryPoint],
    index: usize,
    actual_close_time: &Option<DateTime<Utc>>,
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
                // If end_time is null, then the segment extends to the end of the market.
                // This should only happen for the very last item.
                let end = actual_close_time
                    .with_context(|| "Market actual_close_time not present for resolved market.")?;

                // The actual_close_time may be before the final history point if the market was closed retroactively.
                // We could properly redact those but I'd rather keep those predictions for comparison.
                // For question analysis we set our own end dates anyways so more data will only be more helpful.
                if end < start {
                    continue;
                }
                end
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
