//!

use crate::criterion::CriterionType;
use crate::outcome::OutcomeId;
use anyhow::{Result, bail};
use chrono::NaiveDate;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

/// The average probability over this day
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct DailyProbability {
    pub outcome_id: OutcomeId,
    pub date: NaiveDate,
    pub probability: Probability,
}

/// The probability at this criterion point
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct CriterionProbability {
    pub outcome_id: OutcomeId,
    pub criterion: CriterionType,
    pub probability: Probability,
}

/// A single probability observation
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct InstantProbability {
    pub outcome_id: OutcomeId,
    pub datetime: DateTime<Utc>,
    pub probability: Probability,
}

/*
/// The value that this outcome was resolved to
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct ResolutionProbability {
    pub outcome_id: OutcomeId,
    pub datetime: DateTime<Utc>,
    pub probability: Probability,
}
*/
