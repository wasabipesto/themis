use super::{DailyProbability, ProbSegment};
use chrono::{DateTime, Utc};

/// Converts
pub fn get_market_duration(start: DateTime<Utc>, end: DateTime<Utc>) -> u32 {
    todo!();
}

/// Find the average probability during the specified time window.
pub fn get_prob_time_avg(probs: &[ProbSegment], start: DateTime<Utc>, end: DateTime<Utc>) -> f32 {
    todo!();
}

/// Find the probability at the specified datetime.
pub fn get_prob_at_time(probs: &[ProbSegment], time: DateTime<Utc>) -> f32 {
    todo!();
}

/// Find the probability at the midpoint of the specified time window.
pub fn get_prob_at_midpoint(
    probs: &[ProbSegment],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> f32 {
    todo!();
}

/// Convert probability segments of varying width into daily segments.
/// The day starts and ends at midnight UTC.
/// Probabilities are time-averaged over that window.
pub fn get_daily_probabilities(probs: &[ProbSegment]) -> Vec<DailyProbability> {
    todo!();
}
