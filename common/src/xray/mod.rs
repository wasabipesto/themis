//! Utilities for the X-Ray module.

use crate::market::Market;
use crate::{Label, Url};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod sample;

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
    pub market: Market,
    pub assessment: XRayAssessment,
    pub confidence_aspects: Vec<XRayConfidenceAspect>,
    pub history_chart: HistoryChart,
    pub similar_markets: Vec<Market>,
}
