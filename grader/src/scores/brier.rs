//! Module containing Brier score calculations.

/// Calculate the Brier score given the prediction and the outcome.
pub fn brier_score(prediction: f32, outcome: f32) -> f32 {
    (prediction - outcome).powi(2)
}

/// Convert a Brier score to a letter grade.
pub fn abs_brier_letter_grade(score: &f32) -> String {
    match score {
        x if *x < 0.00005 => "S".to_string(),
        x if *x < 0.0009 => "A+".to_string(),
        x if *x < 0.0018 => "A".to_string(),
        x if *x < 0.0022 => "A-".to_string(),
        x if *x < 0.0030 => "B+".to_string(),
        x if *x < 0.0040 => "B".to_string(),
        x if *x < 0.0055 => "B-".to_string(),
        x if *x < 0.0075 => "C+".to_string(),
        x if *x < 0.013 => "C".to_string(),
        x if *x < 0.024 => "C-".to_string(),
        x if *x < 0.047 => "D+".to_string(),
        x if *x < 0.106 => "D".to_string(),
        x if *x < 0.237 => "D-".to_string(),
        x if *x <= 1.0 => "F".to_string(),
        _ => "ERROR".to_string(),
    }
}

/// Convert a Brier score to a letter grade.
pub fn rel_brier_letter_grade(score: &f32) -> String {
    match score {
        x if *x < -0.075 => "S".to_string(),
        x if *x < -0.040 => "A+".to_string(),
        x if *x < -0.015 => "A".to_string(),
        x if *x < -0.010 => "A-".to_string(),
        x if *x < -0.008 => "B+".to_string(),
        x if *x < -0.004 => "B".to_string(),
        x if *x < -0.002 => "B-".to_string(),
        x if *x < 0.000 => "C+".to_string(),
        x if *x < 0.002 => "C".to_string(),
        x if *x < 0.004 => "C-".to_string(),
        x if *x < 0.008 => "D+".to_string(),
        x if *x < 0.015 => "D".to_string(),
        x if *x < 0.025 => "D-".to_string(),
        x if *x <= 1.0 => "F".to_string(),
        _ => "ERROR".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to compare floating point values with tolerance
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
    fn test_brier_score_edge_cases() {
        // Perfect predictions
        assert_eq!(brier_score(1.0, 1.0), 0.0);
        assert_eq!(brier_score(0.0, 0.0), 0.0);

        // Worst possible predictions
        assert_eq!(brier_score(0.0, 1.0), 1.0);
        assert_eq!(brier_score(1.0, 0.0), 1.0);
    }

    #[test]
    fn test_brier_score_prediction_extremes() {
        // Test with prediction values at extremes
        // Very low probability with outcome 0
        assert_approx_eq(brier_score(0.00001, 0.0), 0.00001_f32.powi(2));

        // Very high probability with outcome 1
        assert_approx_eq(brier_score(0.99999, 1.0), (0.99999_f32 - 1.0_f32).powi(2));

        // Very low probability with outcome 1 (bad prediction)
        assert_approx_eq(brier_score(0.00001, 1.0), (0.00001_f32 - 1.0_f32).powi(2));

        // Very high probability with outcome 0 (bad prediction)
        assert_approx_eq(brier_score(0.99999, 0.0), 0.99999_f32.powi(2));
    }

    #[test]
    fn test_brier_score_normal_range() {
        // Test with various normal probabilities and outcomes
        // Mid-range probabilities with outcome 1
        assert_approx_eq(brier_score(0.5, 1.0), 0.25); // (0.5 - 1.0)^2 = 0.25
        assert_approx_eq(brier_score(0.3, 1.0), 0.49); // (0.3 - 1.0)^2 = 0.49
        assert_approx_eq(brier_score(0.7, 1.0), 0.09); // (0.7 - 1.0)^2 = 0.09

        // Mid-range probabilities with outcome 0
        assert_approx_eq(brier_score(0.5, 0.0), 0.25); // (0.5 - 0.0)^2 = 0.25
        assert_approx_eq(brier_score(0.3, 0.0), 0.09); // (0.3 - 0.0)^2 = 0.09
        assert_approx_eq(brier_score(0.7, 0.0), 0.49); // (0.7 - 0.0)^2 = 0.49
    }

    #[test]
    fn test_brier_score_formula() {
        // Test the formula with various combinations
        // P = 0.25, Outcome = 1
        assert_approx_eq(brier_score(0.25, 1.0), (0.25_f32 - 1.0_f32).powi(2));

        // P = 0.25, Outcome = 0
        assert_approx_eq(brier_score(0.25, 0.0), (0.25_f32 - 0.0_f32).powi(2));

        // P = 0.75, Outcome = 1
        assert_approx_eq(brier_score(0.75, 1.0), (0.75_f32 - 1.0_f32).powi(2));

        // P = 0.75, Outcome = 0
        assert_approx_eq(brier_score(0.75, 0.0), (0.75_f32 - 0.0_f32).powi(2));
    }

    #[test]
    fn test_brier_score_with_partial_outcomes() {
        // Test with partial outcomes (neither 0 nor 1)
        // Outcome = 0.3, P = 0.3 (perfectly calibrated)
        assert_approx_eq(brier_score(0.3, 0.3), 0.0);

        // Outcome = 0.7, P = 0.3 (underconfident)
        assert_approx_eq(brier_score(0.3, 0.7), 0.16); // (0.3 - 0.7)^2 = 0.16

        // Outcome = 0.3, P = 0.7 (overconfident)
        assert_approx_eq(brier_score(0.7, 0.3), 0.16); // (0.7 - 0.3)^2 = 0.16
    }

    #[test]
    fn test_brier_score_boundary_values() {
        // Test with values very close to boundaries
        // Very close to 0
        let epsilon = f32::EPSILON;
        assert_approx_eq(brier_score(epsilon, 0.0), epsilon.powi(2));

        // Very close to 1
        let almost_one = 1.0 - f32::EPSILON;
        assert_approx_eq(brier_score(almost_one, 1.0), (almost_one - 1.0).powi(2));

        // Other boundary tests
        assert_approx_eq(brier_score(0.0, epsilon), epsilon.powi(2));
        assert_approx_eq(brier_score(1.0, almost_one), (1.0 - almost_one).powi(2));
    }

    #[test]
    fn test_brier_score_symmetry() {
        // The Brier score should be symmetric: score(p, o) = score(1-p, 1-o)
        // For example, predicting 0.7 when outcome is 0.3 should give same score as
        // predicting 0.3 when outcome is 0.7
        let p1 = 0.7;
        let o1 = 0.3;
        let p2 = 1.0 - p1; // 0.3
        let o2 = 1.0 - o1; // 0.7

        assert_approx_eq(brier_score(p1, o1), brier_score(p2, o2));

        // Try with more values
        for i in 0..10 {
            let p = i as f32 * 0.1;
            let o = 0.5; // fixed outcome
            let p_complement = 1.0 - p;
            let o_complement = 1.0 - o;

            assert_approx_eq(brier_score(p, o), brier_score(p_complement, o_complement));
        }
    }
}
