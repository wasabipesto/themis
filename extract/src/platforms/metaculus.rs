//! Tools to download and process markets from the Metaculus API.
//! Metaculus API docs: https://www.metaculus.com/api/

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use log::error;
use serde::Deserialize;

use super::{helpers, MarketAndProbs, ProbSegment, StandardMarket};

/// This is the container format we used to save items to disk earlier.
#[derive(Debug, Clone, Deserialize)]
pub struct MetaculusData {
    /// Market ID used for lookups.
    pub id: String,
    /// Timestamp the market was downloaded from the API.
    pub last_updated: DateTime<Utc>,
    // Values returned from the `/questions` endpoint.
    // Ignoring this because everything is also in `extended_data`.
    // pub question: MetaculusQuestionBasic,
    /// Values returned from the `/questions/{id}` endpoint.
    pub extended_data: MetaculusInfo,
}

/// A point within the aggregation history.
/// Aggregation methods are internal, we don't get detailed data.
/// TODO: Look into ForecastDataCSV in the future
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

/// What kind of market this is.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetaculusType {
    /// Typical binary market. Predictions are single points or distributions.
    Binary,
    /// Potential future market type. Currently unused.
    Continuous,
}

/// Info on each tag applied to the question.
#[derive(Debug, Clone, Deserialize)]
pub struct MetaculusTag {
    /// The tag's ID.
    pub id: u32,
    /// The tag's name.
    pub name: String,
    /// The tag's URL slug.
    pub slug: String,
}

/// Some additional information.
/// This object has a lot of redundant information.
#[derive(Debug, Clone, Deserialize)]
pub struct MetaculusQuestion {
    /// Question description.
    /// Can be multiple lines, separated with "\n\n".
    pub description: String,
    /// Question resolution criteria.
    /// Can be multiple lines, separated with "\n\n".
    pub resolution_criteria: String,
    /// Question fine print.
    /// Can be multiple lines, separated with "\n\n".
    pub fine_print: String,
    /// What type of question this is.
    #[serde(rename = "type")]
    pub market_type: MetaculusType,

    /// If resolved, the value of the resolution.
    pub resolution: Option<MetaculusResolution>,

    /// How much this question is weighted (for competitions?)
    /// Always between 0 and 1 so far.
    pub question_weight: f32,
    /// Whether bots are included in aggregates.
    /// Only true around 80% of the time.
    pub include_bots_in_aggregates: bool,

    /// A list of community aggregation points for this question.
    /// We will use this for our probability history.
    pub aggregations: MetaculusAggregationSeries,

    /// Tags applied to this question.
    /// Unsure if we should use this or projects for categorization.
    #[serde(default)] // default to empty vec
    pub tag: Vec<MetaculusTag>,
}

/// Info on each project associated with the question.
#[derive(Debug, Clone, Deserialize)]
pub struct MetaculusProjects {
    /// The project's ID.
    pub id: u32,
    /// The project's name.
    pub name: String,
}

/// Info on each project associated with the question.
#[derive(Debug, Clone, Deserialize)]
pub struct MetaculusProjectSeries {
    /// TODO: Unknown.
    pub default_project: MetaculusProjects,
    /// TODO: Unknown.
    #[serde(default)] // default to empty vec
    pub question_series: Vec<MetaculusProjects>,
    /// TODO: Unknown.
    #[serde(default)] // default to empty vec
    pub site_main: Vec<MetaculusProjects>,
}

/// What stage of the market life-cycle this is in.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetaculusStatus {
    Approved,
    Pending,
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
    /// The URL slug for this question.
    /// Can optionally be added to the end of the URL.
    pub slug: String,

    /// More information about the question.
    /// This object has a lot of redundant information.
    pub question: MetaculusQuestion,
    /// Some data about the projects associated with the question.
    pub projects: MetaculusProjectSeries,

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
    pub edited_at: DateTime<Utc>,
    /// If closed, the most recent close time.
    pub actual_close_time: Option<DateTime<Utc>>,
    /// If resolved, the resolution time.
    pub resolution_set_time: Option<DateTime<Utc>>,
}

/// Convert data pulled from the API into a standardized market item.
/// Returns Error if there were any actual problems with the processing.
/// Returns None if the market was invalid in an expected way.
/// Otherwise, returns a list of markets with probabilities.
pub fn standardize(input: &MetaculusData) -> Result<Option<Vec<MarketAndProbs>>> {
    // Only process resolved markets
    match input.extended_data.status {
        MetaculusStatus::Resolved => {}
        _ => return Ok(None),
    }
    // Skip markets that have not resolved
    if !input.extended_data.resolved {
        return Ok(None);
    }

    // Convert based on market type
    match input.extended_data.question.market_type {
        // Currently only binary markets exist
        MetaculusType::Binary => {
            // Get market ID. Construct from platform slug and ID within platform.
            let platform_slug = "metaculus".to_string();
            let market_id = format!("{}:{}", platform_slug, input.extended_data.id);

            // Get probability segments. If there are none then skip.
            // Using recency_weighted (community prediction) here, may change in the future.
            let probs = build_prob_segments(
                &input
                    .extended_data
                    .question
                    .aggregations
                    .recency_weighted
                    .history,
                &input.extended_data.actual_close_time,
            )
            .with_context(|| format!("Error building probability segments. ID: {market_id}"))?;
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
                id: market_id.clone(),
                title: input.extended_data.title.clone(),
                platform_slug,
                platform_name: "Metaculus".to_string(),
                description: format!(
                    "{}\n\n{}\n\n{}",
                    input.extended_data.question.description.clone(),
                    input.extended_data.question.resolution_criteria.clone(),
                    input.extended_data.question.fine_print.clone(),
                ),
                url: format!(
                    "https://www.metaculus.com/questions/{}",
                    input.extended_data.id
                ),
                open_datetime: start,
                close_datetime: end,
                traders_count: Some(input.extended_data.nr_forecasters),
                volume_usd: None, // Not available in API
                duration_days: helpers::get_market_duration(start, end)?,
                category: None, // TODO
                prob_at_midpoint: helpers::get_prob_at_midpoint(&probs, start, end)?,
                prob_time_avg: helpers::get_prob_time_avg(&probs, start, end)?,
                resolution: match input.extended_data.question.resolution {
                    Some(MetaculusResolution::Yes) => 1.0,
                    Some(MetaculusResolution::No) => 0.0,
                    Some(MetaculusResolution::Ambiguous) => return Ok(None),
                    Some(MetaculusResolution::Annulled) => return Ok(None),
                    None => {
                        error!("Resolved market {market_id} had no resolution value.");
                        return Ok(None);
                    }
                },
            };
            Ok(Some(vec![MarketAndProbs {
                market,
                daily_probabilities,
            }]))
        }
        MetaculusType::Continuous => {
            log::error!("Metaculus `continuous` type detected. We don't have enough information on that type to properly standardize it. Market ID: {}", input.extended_data.id);
            Ok(None)
        }
    }
}

/// Converts Metaculus aggregated history points into standard probability segments.
pub fn build_prob_segments(
    raw_history: &[MetaculusAggregationHistoryPoint],
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
        // There is (so far) only ever one per aggregation but there could be more in the future.
        let means = item.means.unwrap_or_default();
        let prob = match means.first() {
            None => return Err(anyhow!("Aggregation series has no mean.")),
            Some(prob) => prob.to_owned(),
        };
        if means.len() > 1 {
            log::warn!("Aggregation series has multiple means. Using the first one.");
        }

        segments.push(ProbSegment { start, end, prob });
    }
    Ok(segments)
}
