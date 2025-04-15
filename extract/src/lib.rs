//! Tools to extract information from each platform's unique API responses.

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fmt;

use criterions::CriterionProbability;

pub mod criterions;
pub mod helpers;
pub mod platforms;

/// Standardized market. It has everything we need.
#[derive(Debug, Serialize, Clone)]
pub struct StandardMarket {
    pub id: String,
    pub title: String,
    pub url: String,
    pub description: String,
    pub platform_slug: String,
    pub category_slug: Option<String>,
    pub open_datetime: DateTime<Utc>,
    pub close_datetime: DateTime<Utc>,
    pub traders_count: Option<u32>,
    pub volume_usd: Option<f32>,
    pub duration_days: u32,
    pub prob_at_midpoint: f32,
    pub prob_time_avg: f32,
    pub resolution: f32,
}

/// Standardized market with history data.
#[derive(Debug, Serialize, Clone)]
pub struct MarketAndProbs {
    pub market: StandardMarket,
    pub daily_probabilities: Vec<DailyProbability>,
    pub other_probabilities: Vec<CriterionProbability>,
}

/// A fully-constructed probability data point.
#[derive(Debug, Serialize, Clone)]
pub struct DailyProbability {
    pub market_id: String,
    pub date: DateTime<Utc>,
    pub prob: f32,
}

/// A segment of time and the market probability during that period.
#[derive(Debug, Serialize, Clone)]
pub struct ProbSegment {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub prob: f32,
}

/// Standardization Errors
#[derive(Debug)]
pub enum MarketError {
    NotAMarket(String),
    MarketNotResolved(String),
    MarketCancelled(String),
    NoMarketTrades(String),
    InvalidMarketTrades(String, String),
    DataInvalid(String, String),
    ProcessingError(String, String),
    MarketTypeNotImplemented(String, String),
}
pub type MarketResult<T> = Result<T, MarketError>;
impl std::error::Error for MarketError {}
impl fmt::Display for MarketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MarketError::NotAMarket(id) => write!(f, "{}: Item is not a market.", id),
            MarketError::MarketNotResolved(id) => write!(f, "{}: Market is not resolved.", id),
            MarketError::MarketCancelled(id) => {
                write!(f, "{}: Market has been cancelled.", id)
            }
            MarketError::NoMarketTrades(id) => write!(f, "Market has no trades (ID: {}).", id),
            MarketError::InvalidMarketTrades(id, msg) => {
                write!(f, "{}: Error processing market trades: {}", id, msg)
            }
            MarketError::DataInvalid(id, msg) => {
                write!(f, "{}: Platform data invalid: {}", id, msg)
            }
            MarketError::ProcessingError(id, msg) => {
                write!(f, "{}: Error processing market data: {}", id, msg)
            }
            MarketError::MarketTypeNotImplemented(id, market_type) => {
                write!(f, "{}: Market type not implemented: {}", id, market_type)
            }
        }
    }
}
