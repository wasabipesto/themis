//! Module containing Brier score calculations.

/// Calculate the Brier score given the prediction and the outcome.
/// Also known as the Quadratic score, and is equivalent to mean squared error.
/// For any resolution (r) this is simply:
///   (p - r) ^ 2
///
pub fn brier_score(prediction: f32, outcome: f32) -> f32 {
    (prediction - outcome).powi(2)
}

/// Given a Brier score, and assuming that the resolution is 1, recreate the
/// probability of the event.
///
/// Originally: score = (p - r) ^ 2
/// Inverted:   p = 1 - sqrt(s)
///
pub fn invert_brier_score(score: f32) -> f32 {
    1.0 - score.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to compare floating point values with tolerance
    fn assert_approx_eq(actual: f32, expected: f32) {
        let epsilon = 1e-6;
        let diff = (actual - expected).abs();
        assert!(
            diff < epsilon,
            "Expected approximately {}, got {}",
            expected,
            actual
        );
    }

    #[test]
    /// Test with various normal probabilities and outcomes
    fn test_normal_range() {
        assert_approx_eq(brier_score(0.5, 0.0), 0.25);
        assert_approx_eq(brier_score(0.5, 1.0), 0.25);
        assert_approx_eq(brier_score(0.3, 0.0), 0.09);
        assert_approx_eq(brier_score(0.7, 1.0), 0.09);
        assert_approx_eq(brier_score(0.7, 0.0), 0.49);
        assert_approx_eq(brier_score(0.3, 1.0), 0.49);
    }

    #[test]
    /// Test with prediction values near extremes
    fn test_prediction_extremes() {
        assert_approx_eq(brier_score(0.00001, 0.0), 0.00001_f32.powi(2));
        assert_approx_eq(brier_score(0.99999, 1.0), 0.00001_f32.powi(2));
        assert_approx_eq(brier_score(0.00001, 1.0), 0.99999_f32.powi(2));
        assert_approx_eq(brier_score(0.99999, 0.0), 0.99999_f32.powi(2));
    }

    #[test]
    /// Test best and worst possible predictions
    fn test_edge_cases() {
        assert_eq!(brier_score(1.0, 1.0), 0.0);
        assert_eq!(brier_score(0.0, 0.0), 0.0);
        assert_eq!(brier_score(0.0, 1.0), 1.0);
        assert_eq!(brier_score(1.0, 0.0), 1.0);
    }

    #[test]
    /// Test with partial outcomes (neither 0 nor 1)
    fn test_partial_outcomes() {
        assert_approx_eq(brier_score(0.3, 0.3), 0.0);
        assert_approx_eq(brier_score(0.3, 0.7), 0.16);
        assert_approx_eq(brier_score(0.7, 0.3), 0.16);
    }

    #[test]
    /// Test that scores are the same when both prediction and outcome are inverted
    fn test_symmetry() {
        // The Brier score should be symmetric: score(p, o) = score(1-p, 1-o)
        // For example, predicting 0.7 when outcome is 0.3 should give same score as predicting 0.3 when outcome is 0.7
        let p1 = 0.7;
        let o1 = 0.3;
        let p2 = 1.0 - p1;
        let o2 = 1.0 - o1;
        assert_approx_eq(brier_score(p1, o1), brier_score(p2, o2));

        // Try with more values
        for i in 0..10 {
            let p = i as f32 * 0.1;
            let o = 0.5;
            let p_complement = 1.0 - p;
            let o_complement = 1.0 - o;
            assert_approx_eq(brier_score(p, o), brier_score(p_complement, o_complement));
        }
    }

    #[test]
    /// Test that the score inverts back to the original probability
    fn test_inversion() {
        for i in 0..1000 {
            let prediction = i as f32 / 1000.0;
            let outcome = 1.0;
            let score = brier_score(prediction, outcome);
            assert_approx_eq(invert_brier_score(score), prediction);
        }
    }
}
