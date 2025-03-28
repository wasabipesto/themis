//! Helper functions for dealing with probabilities over time

use super::{DailyProbability, ProbSegment};
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, TimeZone, Utc};
use log::{debug, error, warn};

/// Gets the number of calendar days covered (in UTC) by start and end,
/// NOT the number of days from the start time to the end time.
pub fn get_market_duration(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<u32> {
    if end <= start {
        return Err(anyhow::anyhow!("End time must be after start time"));
    }

    let days = (end.date_naive() - start.date_naive()).num_days() as u32 + 1;
    Ok(days)
}

/// Find the time-weighted average probability during the specified time window.
/// Assumes the prob segments are sorted and non-overlapping.
pub fn get_prob_time_avg(
    probs: &[ProbSegment],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<f32> {
    if probs.is_empty() {
        debug!("No probability segments provided for time average calculation, returning default");
        return Ok(0.5);
    }

    if end <= start {
        return Err(anyhow::anyhow!("End time must be after start time"));
    }

    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;

    for segment in probs {
        let overlap_start = start.max(segment.start);
        let overlap_end = end.min(segment.end);

        if overlap_start < overlap_end {
            let duration = (overlap_end - overlap_start).num_milliseconds() as f32;
            weighted_sum += segment.prob * duration;
            total_weight += duration;
        }
    }

    if total_weight > 0.0 {
        Ok(weighted_sum / total_weight)
    } else {
        error!(
            "No prob segments found in window ({start} to {end}): {:?}",
            probs
        );
        Err(anyhow::anyhow!(
            "No valid time segments found for probability calculation"
        ))
    }
}

/// Find the probability at the exact specified DateTime.
/// Assumes the prob segments are sorted and non-overlapping.
pub fn get_prob_at_time(probs: &[ProbSegment], time: DateTime<Utc>) -> Result<f32> {
    if probs.is_empty() {
        debug!("No probability segments provided for time lookup, returning default");
        return Ok(0.5);
    }

    for segment in probs {
        if time >= segment.start && time < segment.end {
            return Ok(segment.prob);
        }
    }

    error!("No probability segment found for specified time: {}", time);
    Err(anyhow::anyhow!(
        "No probability segment found for specified time"
    ))
}

/// Find the probability at the midpoint of the specified time window.
/// Assumes the prob segments are sorted and non-overlapping.
pub fn get_prob_at_midpoint(
    probs: &[ProbSegment],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<f32> {
    if end <= start {
        return Err(anyhow::anyhow!("End time must be after start time"));
    }

    let duration = end - start;
    let midpoint = start + (duration / 2);
    get_prob_at_time(probs, midpoint)
}

/// Sets the hour on the given DateTime<Utc>.
fn dt_set_hour(dt: DateTime<Utc>, hour: u32) -> Result<DateTime<Utc>> {
    if hour >= 24 {
        return Err(anyhow::anyhow!("Hour must be between 0 and 23"));
    }

    let naive_dt = dt
        .date_naive()
        .and_hms_opt(hour, 0, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid hour value: {}", hour))?;

    naive_dt
        .and_local_timezone(Utc)
        .single()
        .ok_or_else(|| anyhow::anyhow!("Failed to convert naive datetime to UTC"))
}

/// Convert probability segments of varying width into daily segments.
/// The day starts and ends at midnight UTC. Date point is at noon UTC.
/// Probabilities are time-averaged over that window.
pub fn get_daily_probabilities(
    probs: &[ProbSegment],
    market_id: &str,
) -> Result<Vec<DailyProbability>> {
    if probs.is_empty() {
        warn!("No probability segments provided for daily probability calculation");
        return Ok(vec![]);
    }

    let range_start = probs
        .first()
        .context("Failed to get first probability segment")?
        .start;
    let range_end = probs
        .last()
        .context("Failed to get last probability segment")?
        .end;

    let mut daily_probs = Vec::new();
    let mut day_start = dt_set_hour(range_start, 0)?;

    while day_start < range_end {
        let day_end = day_start + Duration::days(1);
        let day_midpoint = day_start + Duration::hours(12);
        let prob = get_prob_time_avg(probs, day_start, day_end)
            .context("Failed to calculate daily probability")?;

        daily_probs.push(DailyProbability {
            market_id: market_id.to_owned(),
            date: day_midpoint,
            prob,
        });

        day_start = day_end;
    }

    Ok(daily_probs)
}

/// Validates that probability segments are properly ordered and continuous.
/// Returns Ok(()) if segments are valid, or an Error with details about the issue found.
pub fn validate_prob_segments(probs: &[ProbSegment]) -> Result<()> {
    let mut previous: Option<&ProbSegment> = None;

    for segment in probs {
        // Segment start and end date sanity checks
        let too_early = Utc::with_ymd_and_hms(&Utc, 2000, 1, 1, 0, 0, 0).unwrap();
        let too_late = Utc::now();
        if segment.start < too_early || segment.start > too_late {
            return Err(anyhow::anyhow!(
                "Segment end date ({}) is out of bounds [{} - {}]",
                segment.end,
                too_early,
                too_late,
            ));
        }
        if segment.end < too_early || segment.end > too_late {
            return Err(anyhow::anyhow!(
                "Segment end date ({}) is out of bounds [{} - {}]",
                segment.end,
                too_early,
                too_late,
            ));
        }

        // Check that the segment has positive width
        if segment.end <= segment.start {
            return Err(anyhow::anyhow!(
                "Invalid segment: end time ({}) <= start time ({})",
                segment.end,
                segment.start,
            ));
        }

        if let Some(prev) = &previous {
            // Check that the previous segment does not intrude
            if segment.start < prev.end {
                return Err(anyhow::anyhow!(
                "Overlapping segments detected: previous segment ends at {}, this segment starts at {}",
                prev.end,
                segment.start
            ));
            }

            // Check that the previous segment meets this one
            if segment.start > prev.end {
                return Err(anyhow::anyhow!(
                "Gap between segments detected: previous segment ends at {}, this segment starts at {}",
                prev.end,
                segment.start
            ));
            }
        }

        previous = Some(segment);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn create_dt(year: i32, month: u32, day: u32, hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, hour, 0, 0)
            .single()
            .unwrap()
    }

    #[test]
    fn test_get_market_duration() {
        // Normal case
        let start = create_dt(2023, 1, 1, 0);
        let end = create_dt(2023, 1, 3, 0);
        assert_eq!(get_market_duration(start, end).unwrap(), 3);

        // Long duration
        let end = create_dt(2024, 1, 1, 12);
        assert_eq!(get_market_duration(start, end).unwrap(), 366);

        // Partial day
        let end = create_dt(2023, 1, 1, 12);
        assert_eq!(get_market_duration(start, end).unwrap(), 1);

        // Short duration over multiple days
        let start = create_dt(2023, 1, 1, 20);
        let end = create_dt(2023, 1, 2, 2);
        assert_eq!(get_market_duration(start, end).unwrap(), 2);

        // Invalid case: end before start
        let start = create_dt(2023, 1, 2, 0);
        let end = create_dt(2023, 1, 1, 0);
        assert!(get_market_duration(start, end).is_err());

        // Same time
        let start = create_dt(2023, 1, 1, 0);
        let end = create_dt(2023, 1, 1, 0);
        assert!(get_market_duration(start, end).is_err());
    }

    #[test]
    fn test_get_prob_time_avg() {
        let start = create_dt(2023, 1, 1, 0);
        let mid = create_dt(2023, 1, 1, 12);
        let end = create_dt(2023, 1, 2, 0);

        // Empty segments
        let probs: Vec<ProbSegment> = vec![];
        assert_eq!(get_prob_time_avg(&probs, start, end).unwrap(), 0.5);

        // Single segment covering entire period
        let probs = vec![ProbSegment {
            start,
            end,
            prob: 0.7,
        }];
        assert_eq!(get_prob_time_avg(&probs, start, end).unwrap(), 0.7);

        // Multiple segments with different probabilities
        let probs = vec![
            ProbSegment {
                start,
                end: mid,
                prob: 0.6,
            },
            ProbSegment {
                start: mid,
                end,
                prob: 0.8,
            },
        ];
        assert_eq!(get_prob_time_avg(&probs, start, end).unwrap(), 0.7);

        // No overlap with time window
        let later_start = create_dt(2023, 1, 3, 0);
        let later_end = create_dt(2023, 1, 4, 0);
        assert!(get_prob_time_avg(&probs, later_start, later_end).is_err());

        // Invalid time window
        assert!(get_prob_time_avg(&probs, end, start).is_err());
    }

    #[test]
    fn test_subsecond_time_avg() {
        // Test handling of very small time differences (less than 1 second)
        let base = create_dt(2023, 1, 1, 0);
        let plus_500ms = base + Duration::milliseconds(500);
        let plus_750ms = base + Duration::milliseconds(750);
        let plus_1sec = base + Duration::seconds(1);

        let probs = vec![
            ProbSegment {
                start: base,
                end: plus_500ms,
                prob: 0.3,
            },
            ProbSegment {
                start: plus_500ms,
                end: plus_750ms,
                prob: 0.6,
            },
            ProbSegment {
                start: plus_750ms,
                end: plus_1sec,
                prob: 0.9,
            },
        ];

        // Calculate time-weighted average over the full second
        let avg = get_prob_time_avg(&probs, base, plus_1sec).unwrap();

        // Expected average: (0.3 * 0.5 + 0.6 * 0.25 + 0.9 * 0.25) = 0.525
        let expected = 0.3 * 0.5 + 0.6 * 0.25 + 0.9 * 0.25;
        assert!((avg - expected).abs() < 0.0001);

        // Test with even smaller window
        let tiny_start = plus_500ms;
        let tiny_end = tiny_start + Duration::milliseconds(1);
        let tiny_avg = get_prob_time_avg(&probs, tiny_start, tiny_end).unwrap();
        assert_eq!(tiny_avg, 0.6);
    }

    #[test]
    fn test_get_prob_at_time() {
        let start = create_dt(2023, 1, 1, 0);
        let mid = create_dt(2023, 1, 1, 12);
        let end = create_dt(2023, 1, 2, 0);

        // Empty segments
        let probs: Vec<ProbSegment> = vec![];
        assert_eq!(get_prob_at_time(&probs, mid).unwrap(), 0.5);

        // Time exactly at segment start
        let probs = vec![ProbSegment {
            start,
            end,
            prob: 0.7,
        }];
        assert_eq!(get_prob_at_time(&probs, start).unwrap(), 0.7);

        // Time in middle of segment
        assert_eq!(get_prob_at_time(&probs, mid).unwrap(), 0.7);

        // Time outside any segment
        let outside_time = create_dt(2023, 1, 3, 0);
        assert!(get_prob_at_time(&probs, outside_time).is_err());
    }

    #[test]
    fn test_validate_prob_segments() {
        let start = create_dt(2023, 1, 1, 0);
        let mid = create_dt(2023, 1, 1, 12);
        let end = create_dt(2023, 1, 2, 0);

        // Valid segments
        let valid_probs = vec![
            ProbSegment {
                start,
                end: mid,
                prob: 0.6,
            },
            ProbSegment {
                start: mid,
                end,
                prob: 0.8,
            },
        ];
        assert!(validate_prob_segments(&valid_probs).is_ok());

        // Empty or single segment
        let empty_probs: Vec<ProbSegment> = vec![];
        assert!(validate_prob_segments(&empty_probs).is_ok());
        let single_prob = vec![ProbSegment {
            start,
            end,
            prob: 0.7,
        }];
        assert!(validate_prob_segments(&single_prob).is_ok());

        // Invalid: start >= end
        let invalid_probs = vec![ProbSegment {
            start: end,
            end: start,
            prob: 0.7,
        }];
        assert!(validate_prob_segments(&invalid_probs).is_err());

        // Overlapping segments
        let overlapping_probs = vec![
            ProbSegment {
                start,
                end,
                prob: 0.6,
            },
            ProbSegment {
                start: mid,
                end,
                prob: 0.8,
            },
        ];
        assert!(validate_prob_segments(&overlapping_probs).is_err());

        // Gap between segments
        let gap_probs = vec![
            ProbSegment {
                start,
                end: mid,
                prob: 0.6,
            },
            ProbSegment {
                start: create_dt(2023, 1, 1, 13),
                end,
                prob: 0.8,
            },
        ];
        assert!(validate_prob_segments(&gap_probs).is_err());
    }

    #[test]
    fn test_get_daily_probabilities() {
        let start = create_dt(2023, 1, 1, 0);
        let mid = create_dt(2023, 1, 1, 12);
        let end = create_dt(2023, 1, 3, 0);

        // Normal case
        let probs = vec![
            ProbSegment {
                start,
                end: mid,
                prob: 0.6,
            },
            ProbSegment {
                start: mid,
                end,
                prob: 0.8,
            },
        ];
        let daily_probs = get_daily_probabilities(&probs, "").unwrap();
        assert_eq!(daily_probs.len(), 2);
        assert_eq!(daily_probs[0].prob, 0.7); // Average for first day

        // Empty input
        let empty_probs: Vec<ProbSegment> = vec![];
        assert!(get_daily_probabilities(&empty_probs, "")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn test_get_prob_at_midpoint() {
        let start = create_dt(2023, 1, 1, 0);
        let mid = create_dt(2023, 1, 1, 12);
        let end = create_dt(2023, 1, 2, 0);

        let probs = vec![
            ProbSegment {
                start,
                end: mid,
                prob: 0.6,
            },
            ProbSegment {
                start: mid,
                end,
                prob: 0.8,
            },
        ];

        // Normal case
        assert_eq!(get_prob_at_midpoint(&probs, start, end).unwrap(), 0.8);

        // Invalid time window
        assert!(get_prob_at_midpoint(&probs, end, start).is_err());

        // Empty segments
        let empty_probs: Vec<ProbSegment> = vec![];
        assert_eq!(get_prob_at_midpoint(&empty_probs, start, end).unwrap(), 0.5);
    }

    #[test]
    fn test_microsecond_precision() {
        // Test handling of very small time differences
        let base = create_dt(2023, 1, 1, 0);
        let almost_same = base + Duration::microseconds(1);
        let tiny_later = base + Duration::microseconds(2);

        let probs = vec![
            ProbSegment {
                start: base,
                end: almost_same,
                prob: 0.3,
            },
            ProbSegment {
                start: almost_same,
                end: tiny_later,
                prob: 0.7,
            },
        ];

        assert!(validate_prob_segments(&probs).is_ok());
        assert_eq!(get_prob_at_time(&probs, almost_same).unwrap(), 0.7);
    }

    #[test]
    fn test_long_duration_accuracy() {
        // Test handling of very long time periods
        let start = create_dt(2000, 1, 1, 0);
        let far_future = create_dt(3000, 1, 1, 0);

        let long_prob = vec![ProbSegment {
            start,
            end: far_future,
            prob: 0.5,
        }];

        let duration = get_market_duration(start, far_future).unwrap();
        assert!(duration > 365000);

        let daily_probs = get_daily_probabilities(&long_prob, "").unwrap();
        assert!(daily_probs.len() > 365000);
        assert_eq!(daily_probs.first().unwrap().prob, 0.5);
        assert_eq!(daily_probs.last().unwrap().prob, 0.5);
    }
}
