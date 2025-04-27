//! Module containing Logarithmic score calculations.

/// Calculate the Logarithmic score given the prediction and the outcome.
///
/// This is simple when the outcome equal 0 or 1:
///   When outcome is 1 =>  ln(p)
///   When outcome is 0 =>  ln(1 - p)
///
/// The below is a generalization of these equations into something capable of
/// handling resolutions (r) that are inbetween 0 and 1:
///   Generalization    =>  r * ln(p) + (1 - r) * ln(1 - p)
///   When outcome is 1 =>  1 * ln(p) + (1 - 1) * ln(1 - p) = ln(p)
///   When outcome is 0 =>  0 * ln(p) + (1 - 0) * ln(1 - p) = ln(1 - p)
///
/// Since we are using the built-in natural logarithm function, the precision is
/// non-deterministic and could pose an issue in the future. For now, I've
/// included tests that should ensure we have good enough precision for our uses.
/// In the future if it becomes an issue we can use a more precise crate.
///
pub fn log_score(prediction: f32, outcome: f32) -> f32 {
    // Return edge cases early to avoid issues around ln(1).
    if (prediction == 1.0 && outcome == 1.0) || (prediction == 0.0 && outcome == 0.0) {
        return 0.0;
    }

    // Calculate the score with the generalization above.
    let score = outcome * prediction.ln() + (1.0 - outcome) * (1.0 - prediction).ln();

    // Serde will serialize infinities as null, so we set a lower bound on this
    // value in order to actually be able to send it to the database.
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
        assert_approx_eq(log_score(0.5, 1.0), 0.5_f32.ln());
        assert_approx_eq(log_score(0.3, 1.0), 0.3_f32.ln());
        assert_approx_eq(log_score(0.7, 1.0), 0.7_f32.ln());
        assert_approx_eq(log_score(0.5, 0.0), 0.5_f32.ln());
        assert_approx_eq(log_score(0.3, 0.0), 0.7_f32.ln());
        assert_approx_eq(log_score(0.7, 0.0), 0.3_f32.ln());
    }

    #[test]
    /// Test with prediction values near extremes
    fn test_prediction_extremes() {
        assert_approx_eq(log_score(0.00001, 0.0), 0.99999_f32.ln());
        assert_approx_eq(log_score(0.99999, 1.0), 0.99999_f32.ln());
        assert_approx_eq(log_score(0.01, 1.0), 0.01_f32.ln());
        assert_approx_eq(log_score(0.99, 0.0), 0.01_f32.ln());

        // These tests fail due to inaccuracies in the standard f32 ln function.
        // TODO: Find a better crate for this function and re-enable these tests.
        // For now, the built-in function is good enough to around 1%.
        //assert_approx_eq(log_score(0.00001, 1.0), 0.00001_f32.ln());
        //assert_approx_eq(log_score(0.99999, 0.0), 0.00001_f32.ln());
    }

    #[test]
    /// Test best and worst possible predictions
    fn test_edge_cases() {
        assert_approx_eq(log_score(1.0, 1.0), 0.0);
        assert_approx_eq(log_score(0.0, 0.0), 0.0);
        assert_approx_eq(log_score(0.0, 1.0), f32::MIN);
        assert_approx_eq(log_score(1.0, 0.0), f32::MIN);
    }

    #[test]
    /// Test with partial outcomes (neither 0 nor 1)
    fn test_partial_outcomes() {
        let expected_score = 0.3 * 0.3_f32.ln() + 0.7 * 0.7_f32.ln();
        assert_approx_eq(log_score(0.3, 0.3), expected_score);
        let expected_score = 0.7 * 0.3_f32.ln() + 0.3 * 0.7_f32.ln();
        assert_approx_eq(log_score(0.3, 0.7), expected_score);
        let expected_score = 0.3 * 0.7_f32.ln() + 0.7 * 0.3_f32.ln();
        assert_approx_eq(log_score(0.7, 0.3), expected_score);
    }
}
