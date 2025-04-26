//! Module containing Logarithmic score calculations.

/// Calculate the Logarithmic score given the prediction and the outcome.
pub fn log_score(prediction: f32, outcome: f32) -> f32 {
    let score = if (prediction == 0.0 && outcome == 1.0) || (prediction == 1.0 && outcome == 0.0) {
        f32::NEG_INFINITY
    } else if (prediction == 1.0 && outcome == 1.0) || (prediction == 0.0 && outcome == 0.0) {
        0.0
    } else {
        outcome * prediction.ln() + (1.0 - outcome) * (1.0 - prediction).ln()
    };

    // Serde will serialize infinities as null, so we set bounds on this value.
    score.max(f32::MIN)
}

/// Convert a Logarithmic score to a letter grade.
pub fn abs_log_letter_grade(score: &f32) -> String {
    match score {
        x if *x > -0.005 => "S".to_string(),
        x if *x > -0.030 => "A+".to_string(),
        x if *x > -0.041 => "A".to_string(),
        x if *x > -0.048 => "A-".to_string(),
        x if *x > -0.056 => "B+".to_string(),
        x if *x > -0.063 => "B".to_string(),
        x if *x > -0.076 => "B-".to_string(),
        x if *x > -0.090 => "C+".to_string(),
        x if *x > -0.121 => "C".to_string(),
        x if *x > -0.167 => "C-".to_string(),
        x if *x > -0.244 => "D+".to_string(),
        x if *x > -0.391 => "D".to_string(),
        x if *x > -0.668 => "D-".to_string(),
        _ => "F".to_string(),
    }
}

/// Convert a Logarithmic score to a letter grade.
pub fn rel_log_letter_grade(score: &f32) -> String {
    match score {
        x if *x > 0.200 => "S".to_string(),
        x if *x > 0.060 => "A+".to_string(),
        x if *x > 0.030 => "A".to_string(),
        x if *x > 0.015 => "A-".to_string(),
        x if *x > 0.010 => "B+".to_string(),
        x if *x > 0.004 => "B".to_string(),
        x if *x > 0.002 => "B-".to_string(),
        x if *x > 0.000 => "C+".to_string(),
        x if *x > -0.004 => "C".to_string(),
        x if *x > -0.006 => "C-".to_string(),
        x if *x > -0.010 => "D+".to_string(),
        x if *x > -0.040 => "D".to_string(),
        x if *x > -0.085 => "D-".to_string(),
        _ => "F".to_string(),
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
    fn test_log_score_edge_cases() {
        // Test edge cases where score is defined as special values
        assert_eq!(log_score(0.0, 1.0), f32::NEG_INFINITY.max(f32::MIN));
        assert_eq!(log_score(1.0, 0.0), f32::NEG_INFINITY.max(f32::MIN));
        assert_eq!(log_score(1.0, 1.0), 0.0);
        assert_eq!(log_score(0.0, 0.0), 0.0);
    }

    #[test]
    fn test_log_score_prediction_extremes() {
        // Test with prediction values at extremes but with expected outcomes
        // Very low probability with outcome 0
        // For outcome=0, the formula is (1.0 - outcome) * (1.0 - prediction).ln() = 1.0 * 0.99999.ln()
        assert_approx_eq(log_score(0.00001, 0.0), (1.0 - 0.00001_f32).ln());

        // Very high probability with outcome 1
        // For outcome=1, the formula is outcome * prediction.ln() = 1.0 * 0.99999.ln()
        assert_approx_eq(log_score(0.99999, 1.0), 0.99999_f32.ln());
    }

    #[test]
    fn test_log_score_normal_range() {
        // Test with various normal probabilities and outcomes
        // Mid-range probabilities with outcome 1
        assert_approx_eq(log_score(0.5, 1.0), 0.5_f32.ln());
        assert_approx_eq(log_score(0.3, 1.0), 0.3_f32.ln());
        assert_approx_eq(log_score(0.7, 1.0), 0.7_f32.ln());

        // Mid-range probabilities with outcome 0
        assert_approx_eq(log_score(0.5, 0.0), (1.0 - 0.5_f32).ln());
        assert_approx_eq(log_score(0.3, 0.0), (1.0 - 0.3_f32).ln());
        assert_approx_eq(log_score(0.7, 0.0), (1.0 - 0.7_f32).ln());
    }

    #[test]
    fn test_log_score_formula() {
        // Test the complete formula with various combinations
        // P = 0.25, Outcome = 1
        assert_approx_eq(
            log_score(0.25, 1.0),
            1.0 * 0.25_f32.ln() + 0.0 * (1.0 - 0.25_f32).ln(),
        );

        // P = 0.25, Outcome = 0
        assert_approx_eq(
            log_score(0.25, 0.0),
            0.0 * 0.25_f32.ln() + 1.0 * (1.0 - 0.25_f32).ln(),
        );

        // P = 0.75, Outcome = 1
        assert_approx_eq(
            log_score(0.75, 1.0),
            1.0 * 0.75_f32.ln() + 0.0 * (1.0 - 0.75_f32).ln(),
        );

        // P = 0.75, Outcome = 0
        assert_approx_eq(
            log_score(0.75, 0.0),
            0.0 * 0.75_f32.ln() + 1.0 * (1.0 - 0.75_f32).ln(),
        );
    }

    #[test]
    fn test_log_score_with_partial_outcomes() {
        // Test with partial outcomes (neither 0 nor 1)
        // Outcome = 0.3, P = 0.3 (perfectly calibrated)
        let expected_score = 0.3 * 0.3_f32.ln() + 0.7 * 0.7_f32.ln();
        assert_approx_eq(log_score(0.3, 0.3), expected_score);

        // Outcome = 0.7, P = 0.3 (underconfident)
        let expected_score = 0.7 * 0.3_f32.ln() + 0.3 * 0.7_f32.ln();
        assert_approx_eq(log_score(0.3, 0.7), expected_score);

        // Outcome = 0.3, P = 0.7 (overconfident)
        let expected_score = 0.3 * 0.7_f32.ln() + 0.7 * 0.3_f32.ln();
        assert_approx_eq(log_score(0.7, 0.3), expected_score);
    }

    #[test]
    fn test_log_score_boundary_values() {
        // Test with values very close to boundaries
        // Very close to 0
        let epsilon = f32::EPSILON;
        assert_approx_eq(log_score(epsilon, 1.0), epsilon.ln());

        // Very close to 1
        let almost_one = 1.0 - f32::EPSILON;
        assert_approx_eq(log_score(almost_one, 1.0), almost_one.ln());
    }
}
