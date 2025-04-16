//! Helper functions for the grader.

use crate::CriterionProbabilityPoint;
use anyhow::Result;

/// Simple function to get median from a list.
pub fn median(values: &[f32]) -> f32 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

pub fn get_first_probability(
    criteria_probs: &[&CriterionProbabilityPoint],
    criterion_type: &str,
) -> Result<f32> {
    criteria_probs
        .iter()
        .filter(|prob| prob.criterion_type == criterion_type)
        .cloned()
        .collect::<Vec<_>>()
        .first()
        .ok_or_else(|| {
            anyhow::anyhow!("No matching probability found for type: {}", criterion_type)
        })
        .map(|p| p.prob)
}
