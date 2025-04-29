//! Helper functions for the grader.

use crate::CriterionProbabilityPoint;

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

/// Get the first probability in the list that matches the criterion type.
/// Assumes that the list has been filtered to the correct market.
pub fn get_first_probability(
    criteria_probs: &[CriterionProbabilityPoint],
    criterion_type: &str,
) -> Option<CriterionProbabilityPoint> {
    criteria_probs
        .iter()
        .filter(|prob| prob.criterion_type == criterion_type)
        .cloned()
        .collect::<Vec<_>>()
        .first()
        .cloned()
}
