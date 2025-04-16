//! Library for the grader.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

pub mod api;
pub mod helpers;
pub mod scores;

/// Parameters that we need in order to upload items to the database.
pub struct PostgrestParams {
    pub postgrest_url: String,
    pub postgrest_api_key: String,
}

/// Standard platform information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Platform {
    pub slug: String,
    pub name: String,
}

/// Standard category information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Category {
    pub slug: String,
    pub name: String,
}

/// Standard market information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Market {
    pub id: String,
    pub platform_slug: String,
    pub category_slug: Option<String>,
    pub question_id: Option<u32>,
    pub question_invert: Option<bool>,
    pub open_datetime: DateTime<Utc>,
    pub close_datetime: DateTime<Utc>,
    pub traders_count: Option<u32>,
    pub volume_usd: Option<f32>,
    pub duration_days: u32,
    pub resolution: f32,
}

/// Daily probability point.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DailyProbabilityPoint {
    pub market_id: String,
    pub date: DateTime<Utc>,
    pub prob: f32,
}

/// Daily probability point.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CriterionProbabilityPoint {
    pub market_id: String,
    pub criterion_type: String,
    pub prob: f32,
}

/// Market with probability.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarketWithProbs {
    pub market: Market,
    pub daily_probs: Vec<DailyProbabilityPoint>,
    pub criterion_probs: Vec<CriterionProbabilityPoint>,
}

/// Question information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Question {
    pub id: u32,
    pub category_slug: String,
    pub start_date_override: Option<NaiveDate>,
    pub end_date_override: Option<NaiveDate>,
}
