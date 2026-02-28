//! Capabilities for standardized markets.

use crate::Url;
use crate::outcome::Outcome;
use crate::platform::{PlatformName, PlatformSlug};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Disambiguated market identifier
#[derive(
    Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Default, derive_more::Display,
)]
pub struct MarketId(pub String);

/// Market title/question text
#[derive(
    Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Default, derive_more::Display,
)]
pub struct MarketTitle(pub String);

/// Tags applied to the market
#[derive(
    Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Default, derive_more::Display,
)]
pub struct Tag(pub String);

/// Type of prediction market structure
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum MarketType {
    Binary,
    DiscreteOne,
    DiscreteMulti,
    Continuous,
    Numeric,
    Date,
}

/// Complete market data including metadata and outcomes
#[derive(Debug, Serialize, Deserialize)]
pub struct Market {
    pub platform_slug: PlatformSlug,
    pub platform_name: PlatformName,
    pub title: MarketTitle,
    pub url: Url,
    pub description: String,
    pub market_status: MarketStatus,
    pub market_type: MarketType,
    pub volume_usd: Option<f32>,
    pub unique_traders: Option<u32>,
    pub open_datetime: DateTime<Utc>,
    pub expected_close_datetime: DateTime<Utc>,
    pub actual_close_datetime: Option<DateTime<Utc>>,
    pub last_updated_datetime: DateTime<Utc>,
    pub tags: Vec<Tag>,
    outcomes: Vec<Outcome>,
}

/// Current status of a prediction market
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum MarketStatus {
    PreOpen,
    Open,
    Closed,
    Resolved,
    Cancelled,
}

impl Market {
    /// Check if the market is ready to be scored
    pub fn is_scorable(&self) -> bool {
        self.market_status == MarketStatus::Resolved
            && [
                MarketType::Binary,
                MarketType::DiscreteOne,
                MarketType::DiscreteMulti,
                MarketType::Numeric,
                MarketType::Date,
            ]
            .contains(&self.market_type)
    }

    /// Get the associated outcomes for the market
    pub fn outcomes(&self) -> Vec<Outcome> {
        todo!()
    }
}

impl fmt::Display for Market {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.platform_name, self.title)
    }
}
