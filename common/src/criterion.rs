//! Criterion points are specific instantaneous probabilities or
//! aggregated probabilities that can be used as the outcome prediction value.

use crate::market::Market;
use crate::probability::Probability;
use serde::{Deserialize, Serialize};

/// All possible criterion types.
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum CriterionType {
    Midpoint,
    TimeAverage,
    DurationPercent25,
    DurationPercent75,
    BeforeCloseHours12,
    BeforeCloseHours24,
    BeforeCloseDays7,
    BeforeCloseDays30,
    BeforeCloseDays60,
    BeforeCloseDays90,
    BeforeCloseDays180,
    BeforeCloseDays365,
    AfterStartHours12,
    AfterStartHours24,
    AfterStartDays7,
    AfterStartDays30,
}

impl CriterionType {
    /// Get all criterion types.
    pub fn all() -> Vec<CriterionType> {
        vec![
            CriterionType::Midpoint,
            CriterionType::TimeAverage,
            CriterionType::DurationPercent25,
            CriterionType::DurationPercent75,
            CriterionType::BeforeCloseHours12,
            CriterionType::BeforeCloseHours24,
            CriterionType::BeforeCloseDays7,
            CriterionType::BeforeCloseDays30,
            CriterionType::BeforeCloseDays60,
            CriterionType::BeforeCloseDays90,
            CriterionType::BeforeCloseDays180,
            CriterionType::BeforeCloseDays365,
            CriterionType::AfterStartHours12,
            CriterionType::AfterStartHours24,
            CriterionType::AfterStartDays7,
            CriterionType::AfterStartDays30,
        ]
    }

    /// Given a market, get the criterion probability.
    pub fn get_criterion_probability(_market: Market) -> Probability {
        todo!()
    }
}
