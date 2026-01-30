//! Themis common utilities, definitions, and processes

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod db_util;
pub mod download_util;
pub mod platforms;
pub mod xray;

/// URL wrapper type
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Default)]
pub struct Url(String);

/// Disambiguated market identifier
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Default)]
pub struct MarketId(String);

/// Market title/question text
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Default)]
pub struct MarketTitle(String);

/// Generic label text
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Default)]
pub struct Label(String);

/// Probability value constrained to [0.0, 1.0]
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Probability(f32);

impl Probability {
    /// Creates a new probability, validating it's between 0.0 and 1.0
    pub fn new(value: f32) -> Result<Self> {
        if (0.0..=1.0).contains(&value) {
            Ok(Probability(value))
        } else {
            bail!(
                "Probability must be between 0.0 and 1.0. Was given: {}",
                value
            )
        }
    }
}

/// Type of prediction market structure
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum MarketType {
    #[default]
    Binary,
    DiscreteOne,
    DiscreteMulti,
    Continuous,
    Numeric,
    Date,
}

/// Represents different types of market outcomes
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum Outcomes {
    #[default]
    None,
    Binary(OutcomeBinary),
    Discrete(Vec<OutcomeDiscrete>),
    Continuous(OutcomeContinuous),
    Numeric(Vec<OutcomeNumeric>),
    Date(Vec<OutcomeDate>),
}

/// Binary outcome with a single probability
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct OutcomeBinary {
    pub probability: Probability,
}

/// Discrete outcome with label and probability, usually in sets
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct OutcomeDiscrete {
    pub label: Label,
    pub probability: Probability,
}

/// Continuous outcome with a probability distribution
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct OutcomeContinuous {
    pub label: Label,
    pub continuous_min: f32,
    pub continuous_peak: f32,
    pub continuous_max: f32,
    pub probability: Probability,
}

/// Numeric outcome with strike range
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct OutcomeNumeric {
    pub label: Label,
    pub numerical_strike_low: f32,
    pub numerical_strike_midpoint: f32,
    pub numerical_strike_high: f32,
    pub probability: Probability,
}

/// Date-based outcome with time range
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct OutcomeDate {
    pub label: Label,
    pub date_strike_low: DateTime<Utc>,
    pub date_strike_midpoint: DateTime<Utc>,
    pub date_strike_high: DateTime<Utc>,
    pub probability: Probability,
}

/// Complete market data including metadata and outcomes
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MarketData {
    pub platform_name: platforms::PlatformName,
    pub title: MarketTitle,
    pub url: Url,
    pub description: String,
    pub market_status: MarketStatus,
    pub market_type: MarketType,
    pub volume_usd: Option<f32>,
    pub unique_traders: Option<u32>,
    pub open_datetime: DateTime<Utc>,
    pub expected_close_datetime: DateTime<Utc>,
    pub last_updated_datetime: DateTime<Utc>,
    pub tags: Vec<String>,
    pub outcomes: Outcomes,
}

/// Current status of a prediction market
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum MarketStatus {
    PreOpen,
    #[default]
    Open,
    Closed,
    Resolved,
    Cancelled,
}

/// Overall assessment summary for X-Ray analysis
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct XRayAssessment {
    pub title: String,
    pub description: String,
    pub confidence_percent: f32,
}

/// Type of confidence factor impact
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum ImpactType {
    Positive,
    Negative,
    #[default]
    Neutral,
}

/// Individual factor affecting confidence in market analysis
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct XRayConfidenceAspect {
    pub icon: String,
    pub title: String,
    pub aspect_name: String,
    pub impact_type: ImpactType,
    pub impact_amount: f32,
    pub description: String,
    pub links: Vec<Link>,
}

/// Reference link with context
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Link {
    pub preface: Option<String>,
    pub text: String,
    pub url: Url,
}

/// Single data point in a time series chart
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HistoryChartPoint {
    pub series_label: Label,
    pub series_color: String,
    pub point_datetime: DateTime<Utc>,
    pub point_value: f32,
}

/// Historical data chart with multiple series
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HistoryChart {
    pub title: String,
    pub points: Vec<HistoryChartPoint>,
}

/// Complete X-Ray analysis response including market data and insights
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct XRayAnalysis {
    pub market: MarketData,
    pub assessment: XRayAssessment,
    pub confidence_aspects: Vec<XRayConfidenceAspect>,
    pub history_chart: HistoryChart,
    pub similar_markets: Vec<MarketData>,
}
