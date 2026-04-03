//! Each market may have many possible outcomes, each of which represents a
//! binary contract. These outcomes have probabilities, trades, etc. and may
//! resolve independently of each other.

use crate::Label;
use crate::market::{Market, MarketId};
use crate::probability::{CriterionProbability, DailyProbability, InstantProbability, Probability};
use anyhow::Result;
use chrono::NaiveDate;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique ID for the outcome
pub type OutcomeId = u32;

/// Current status of the outcome. Different outcomes may open/close/resolve
/// at different times.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum OutcomeStatus {
    PreOpen,
    Open,
    Closed,
    Resolved,
    Cancelled,
}

/// Different possible types of market outcomes
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum OutcomeType {
    /// A standard binary market with no additional per-outcome information.
    Binary,
    /// Multiple discrete outcomes but each behave like a simple binary contract.
    /// These may be dependent (sum-to-one) or be fully independent.
    Discrete,
    /// A numeric outcome with a contiguous range of possible values.
    Numeric,
    /// A date outcome with a contiguous range of possible values.
    Date,
}

/// Outcome data for all types of outcomes
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Outcome {
    pub id: OutcomeId,
    pub market_id: MarketId,
    pub label: Option<Label>,
    pub outcome_status: OutcomeStatus,
    pub outcome_type: OutcomeType,
    pub open_datetime: DateTime<Utc>,
    pub expected_close_datetime: DateTime<Utc>,
    pub actual_close_datetime: Option<DateTime<Utc>>,
    pub last_updated_datetime: DateTime<Utc>,
    pub numerical_strike_low: Option<f32>,
    pub numerical_strike_midpoint: Option<f32>,
    pub numerical_strike_high: Option<f32>,
    pub date_strike_low: Option<DateTime<Utc>>,
    pub date_strike_midpoint: Option<DateTime<Utc>>,
    pub date_strike_high: Option<DateTime<Utc>>,
    pub resolution: Option<Probability>,

    // Cache fields, not serialized
    #[serde(skip)]
    all_daily_probs: Option<Vec<DailyProbability>>,
    #[serde(skip)]
    latest_prob: Option<Option<InstantProbability>>,
}

impl Outcome {
    /// Create a new binary outcome
    pub fn new_binary(
        market_id: MarketId,
        outcome_status: OutcomeStatus,
        open_datetime: DateTime<Utc>,
        expected_close_datetime: DateTime<Utc>,
        actual_close_datetime: Option<DateTime<Utc>>,
        resolution: Option<Probability>,
    ) -> Outcome {
        Outcome {
            id: 0,
            market_id,
            outcome_status,
            outcome_type: OutcomeType::Binary,
            label: None,
            open_datetime,
            expected_close_datetime,
            actual_close_datetime,
            last_updated_datetime: Utc::now(),
            numerical_strike_low: None,
            numerical_strike_midpoint: None,
            numerical_strike_high: None,
            date_strike_low: None,
            date_strike_midpoint: None,
            date_strike_high: None,
            resolution,
            all_daily_probs: None,
            latest_prob: None,
        }
    }

    /// Check if this outcome type can be scored
    pub fn is_scorable(&self) -> bool {
        self.outcome_status == OutcomeStatus::Resolved
            && [
                OutcomeType::Binary,
                OutcomeType::Discrete,
                OutcomeType::Numeric,
                OutcomeType::Date,
            ]
            .contains(&self.outcome_type)
    }

    /// Fetch the associated market
    /// Returns Err for database errors
    pub fn market(&self) -> Result<Market> {
        todo!()
    }

    /// Get the probability for a specific day
    /// Returns Err for database errors
    /// Returns Ok(None) if there is no probability for the given date
    pub fn get_daily_probability(&self, _date: NaiveDate) -> Result<&Option<DailyProbability>> {
        todo!()
    }

    /// Get all recorded daily probabilities
    /// Returns Err for database errors
    /// Returns Ok([]) if there are no daily probabilities recorded
    pub fn get_all_daily_probabilities(&self) -> Result<&Vec<DailyProbability>> {
        // Check cache first
        if let Some(cache) = &self.all_daily_probs {
            return Ok(cache);
        }

        // Fetch from database
        let probs = todo!();

        // Store in cache & return
        self.all_daily_probs = Some(probs);
        Ok(&probs)
    }

    /// Get the probability at this criterion point
    /// Returns Err for database errors
    /// Returns Ok(None) if the criterion point is not available
    pub fn get_criterion_probability(&self) -> Result<&Option<CriterionProbability>> {
        todo!()
    }

    /// Get the latest probability observation
    /// Returns Err for database errors
    /// Returns Ok(None) if there are no recorded probability values
    pub fn get_latest_probability(&self) -> Result<&Option<InstantProbability>> {
        // Check cache first
        if let Some(cache) = &self.latest_prob {
            return Ok(cache);
        }

        // Fetch from database
        let prob = todo!();

        // Store in cache & return
        self.latest_prob = Some(prob);
        Ok(&prob)
    }

    /// Request a refresh of the latest probability
    /// Returns Err for database errors or upstream API errors
    /// Returns Ok(None) if there are no recorded probability values
    pub fn get_current_probability(&self) -> Result<&Option<InstantProbability>> {
        todo!()
    }
}
