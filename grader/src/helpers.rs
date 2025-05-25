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

/// Get the probability in the list that matches the criterion type.
/// Assumes that the list has been filtered to the correct market.
/// Will return None if none are found. Will panic if multiple are found.
pub fn get_criterion_probability(
    criteria_probs: &[CriterionProbabilityPoint],
    criterion_type: &str,
) -> Option<CriterionProbabilityPoint> {
    let probs = criteria_probs
        .iter()
        .filter(|prob| prob.criterion_type == criterion_type)
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        probs.len() < 2,
        "Expected zero or one probabilities for criterion type '{}'",
        criterion_type
    );
    probs.first().cloned()
}

#[cfg(test)]
mod tests {
    use super::median;

    #[test]
    fn test_median_odd_length() {
        let values = &[1.0, 2.0, 3.0];
        assert_eq!(median(values), 2.0);
    }

    #[test]
    fn test_median_even_length() {
        let values = &[1.0, 2.0, 3.0, 4.0];
        assert_eq!(median(values), 2.5);
    }

    #[test]
    fn test_median_single_element() {
        let values = &[5.0];
        assert_eq!(median(values), 5.0);
    }

    #[test]
    #[should_panic]
    fn test_median_empty_slice() {
        let values: &[f32] = &[];
        median(values);
    }

    #[test]
    fn test_median_negative_numbers() {
        let values = &[-5.0, 0.0, 5.0];
        assert_eq!(median(values), 0.0);
    }

    #[test]
    fn test_median_floats() {
        let values = &[1.5, 2.5, 3.5];
        assert_eq!(median(values), 2.5);
    }

    #[test]
    fn test_median_even_floats() {
        let values = &[1.1, 2.2, 3.3, 4.4];
        assert_eq!(median(values), 2.75);
    }
}
