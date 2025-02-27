use super::{DailyProbabilityPartial, ProbSegment};
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use log::{debug, error};

/// Gets number of days covered (in UTC) by start and end.
pub fn get_market_duration(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<u32> {
    if end <= start {
        return Err(anyhow::anyhow!("End time must be after start time"));
    }

    let duration = end - start;
    let days = (duration.num_seconds() as f64 / 86400.0).ceil() as u32;
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
            let duration = (overlap_end - overlap_start).num_seconds() as f32;
            weighted_sum += segment.prob * duration;
            total_weight += duration;
        }
    }

    if total_weight > 0.0 {
        Ok(weighted_sum / total_weight)
    } else {
        error!("No overlapping time segments found between start and end times");
        Err(anyhow::anyhow!(
            "No valid time segments found for probability calculation"
        ))
    }
}

/// Find the probability at the exact specified datetime.
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
pub fn get_daily_probabilities(probs: &[ProbSegment]) -> Result<Vec<DailyProbabilityPartial>> {
    if probs.is_empty() {
        debug!("No probability segments provided for daily probability calculation");
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

        daily_probs.push(DailyProbabilityPartial {
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
    if probs.len() <= 1 {
        return Ok(());
    }

    let mut prev = &probs[0];

    // check first item before entering loop
    if prev.start >= prev.end {
        return Err(anyhow::anyhow!(
            "Invalid segment: start time ({}) >= end time ({})",
            prev.start,
            prev.end
        ));
    }

    // Single pass through the array checking both ordering and continuity
    for segment in &probs[1..] {
        if segment.start >= segment.end {
            return Err(anyhow::anyhow!(
                "Invalid segment: start time ({}) >= end time ({})",
                segment.start,
                segment.end
            ));
        }

        if segment.start < prev.end {
            return Err(anyhow::anyhow!(
                "Overlapping segments detected: previous segment ends at {}, next segment starts at {}",
                prev.end,
                segment.start
            ));
        }

        if segment.start > prev.end {
            return Err(anyhow::anyhow!(
                "Gap between segments detected: previous segment ends at {}, next segment starts at {}",
                prev.end,
                segment.start
            ));
        }

        prev = segment;
    }

    Ok(())
}
