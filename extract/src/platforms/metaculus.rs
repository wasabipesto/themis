use chrono::{DateTime, Utc};
use serde::Deserialize;

/// This is the container format we used to save items to disk earlier.
#[derive(Deserialize)]
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
#[derive(Deserialize)]
pub struct MetaculusAggregationHistoryPoint {
    /// Start time of history bucket.
    /// Time is in milliseconds since epoch but formatted as foating-point.
    pub start_time: Option<f32>,
    /// End time of history bucket.
    /// Time is in milliseconds since epoch but formatted as foating-point.
    pub end_time: Option<f32>,
    /// Prediction point mean.
    /// If we have to use just one, this is the point we use.
    pub means: Option<Vec<f32>>,
    /// Cofidence interval lower bound.
    pub interval_lower_bounds: Option<Vec<f32>>,
    /// Cofidence interval upper bound.
    pub interval_upper_bounds: Option<Vec<f32>>,
    /// Cofidence interval center.
    pub centers: Option<Vec<f32>>,
    /// The number of forecasters who have logged predictions up to this point.
    /// This number can only increase over the history of a market.
    pub forecaster_count: u32,
}

/// Within each aggregation series, get the history as a series of buckets or
/// just the latest snapshot. Also includes score data which we don't care about.
#[derive(Deserialize)]
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
#[derive(Deserialize)]
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

/// Info on each tag applied to the question.
#[derive(Deserialize)]
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
#[derive(Deserialize)]
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

/// What kind of market this is.
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetaculusType {
    /// Typical binary market. Predictions are single points or distributions.
    Binary,
    /// Potential future market type.
    Continuous,
}

/// Info on each project associated with the question.
#[derive(Deserialize)]
pub struct MetaculusProjects {
    /// The project's ID.
    pub id: u32,
    /// The project's name.
    pub name: String,
}

/// Info on each project associated with the question.
#[derive(Deserialize)]
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

/// What stage of the market lifecycle this is in.
#[derive(Deserialize)]
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
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetaculusResolution {
    /// Resolved positively.
    Yes,
    /// Resolved negtively.
    No,
    /// Cancelled due to issues with the question interpretation.
    Ambiguous,
    /// Cancelled due to other issues or because the premise is no longer valid.
    Annulled,
}

/// Values returned from the `/questions/{id}` endpoint.
/// https://www.metaculus.com/api/
#[derive(Deserialize)]
pub struct MetaculusInfo {
    /// The unique ID of this question.
    /// Used to built the site URL: https://www.metaculus.com/questions/{id}
    pub id: u32,
    /// Question text, also used as title.
    pub title: String,
    /// The URL slug for this question.
    pub slug: String,
    /// What type of question this is. Always `binary`.
    #[serde(rename = "type")]
    pub mkt_type: Option<MetaculusType>,

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
    /// If resolved, the value of the resolution.
    pub resolution: Option<MetaculusResolution>,
}
