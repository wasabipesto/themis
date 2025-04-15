//! Generating the probability points (criterions) for use in calibration charts.

use anyhow::Result;
use chrono::TimeDelta;
use serde::{Serialize, Serializer};
use std::fmt::{self, Display};

use crate::{helpers, ProbSegment};

/// A probability data point used for calibration plots.
#[derive(Debug, Serialize, Clone)]
pub struct CriterionProbability {
    pub market_id: String,
    pub criterion_type: CriterionType,
    pub prob: f32,
}

/// Different methods of generating a market prediction.
/// These are used to build calibration plots.
#[derive(Debug, Clone)]
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
}
impl Display for CriterionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CriterionType::Midpoint => write!(f, "midpoint"),
            CriterionType::TimeAverage => write!(f, "time-average"),
            CriterionType::DurationPercent25 => write!(f, "duration-percent-25"),
            CriterionType::DurationPercent75 => write!(f, "duration-percent-75"),
            CriterionType::BeforeCloseHours12 => write!(f, "before-close-hours-12"),
            CriterionType::BeforeCloseHours24 => write!(f, "before-close-hours-24"),
            CriterionType::BeforeCloseDays7 => write!(f, "before-close-days-7"),
            CriterionType::BeforeCloseDays30 => write!(f, "before-close-days-30"),
            CriterionType::BeforeCloseDays60 => write!(f, "before-close-days-60"),
            CriterionType::BeforeCloseDays90 => write!(f, "before-close-days-90"),
            CriterionType::BeforeCloseDays180 => write!(f, "before-close-days-180"),
            CriterionType::BeforeCloseDays365 => write!(f, "before-close-days-365"),
        }
    }
}
impl Serialize for CriterionType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
impl CriterionType {
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
        ]
    }
    pub fn calc(
        &self,
        market_id: &String,
        probs: &[ProbSegment],
    ) -> Result<Option<CriterionProbability>> {
        let start = probs.first().unwrap().start;
        let end = probs.last().unwrap().end;
        let prob_opt = match self {
            CriterionType::Midpoint => Some(helpers::get_prob_at_percent(probs, start, end, 0.5)?),
            CriterionType::TimeAverage => Some(helpers::get_prob_time_avg(probs, start, end)?),
            CriterionType::DurationPercent25 => {
                Some(helpers::get_prob_at_percent(probs, start, end, 0.25)?)
            }
            CriterionType::DurationPercent75 => {
                Some(helpers::get_prob_at_percent(probs, start, end, 0.75)?)
            }
            CriterionType::BeforeCloseHours12 => {
                let time = end - TimeDelta::hours(12);
                if time > start {
                    Some(helpers::get_prob_at_time(probs, time)?)
                } else {
                    None
                }
            }
            CriterionType::BeforeCloseHours24 => {
                let time = end - TimeDelta::hours(24);
                if time > start {
                    Some(helpers::get_prob_at_time(probs, time)?)
                } else {
                    None
                }
            }
            CriterionType::BeforeCloseDays7 => {
                let time = end - TimeDelta::days(7);
                if time > start {
                    Some(helpers::get_prob_at_time(probs, time)?)
                } else {
                    None
                }
            }
            CriterionType::BeforeCloseDays30 => {
                let time = end - TimeDelta::days(30);
                if time > start {
                    Some(helpers::get_prob_at_time(probs, time)?)
                } else {
                    None
                }
            }
            CriterionType::BeforeCloseDays60 => {
                let time = end - TimeDelta::days(60);
                if time > start {
                    Some(helpers::get_prob_at_time(probs, time)?)
                } else {
                    None
                }
            }
            CriterionType::BeforeCloseDays90 => {
                let time = end - TimeDelta::days(90);
                if time > start {
                    Some(helpers::get_prob_at_time(probs, time)?)
                } else {
                    None
                }
            }
            CriterionType::BeforeCloseDays180 => {
                let time = end - TimeDelta::days(180);
                if time > start {
                    Some(helpers::get_prob_at_time(probs, time)?)
                } else {
                    None
                }
            }
            CriterionType::BeforeCloseDays365 => {
                let time = end - TimeDelta::days(365);
                if time > start {
                    Some(helpers::get_prob_at_time(probs, time)?)
                } else {
                    None
                }
            }
        };
        if let Some(prob) = prob_opt {
            Ok(Some(CriterionProbability {
                market_id: market_id.to_owned(),
                criterion_type: self.to_owned(),
                prob,
            }))
        } else {
            Ok(None)
        }
    }
}
