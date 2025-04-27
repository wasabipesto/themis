//! Module containing Spherical score calculations.

/// Calculate the Spherical score given the prediction and the outcome.
///
/// The scores are defined as such when the outcome equal 0 or 1:
///   When outcome is 1 =>  p / sqrt(p^2 + (1 - p)^2)
///   When outcome is 0 =>  (1 - p) / sqrt(p^2 + (1 - p)^2)
/// Note that the denominator is the same in both cases.
///
/// The below is a generalization of these equations into something capable of
/// handling resolutions (r) that are inbetween 0 and 1:
///   Generalization    =>  (r * p + (1 - r) * (1 - p)) / sqrt((p^2 + (1 - p)^2))
///   When outcome is 1 =>  (1 * p + (1 - 1) * (1 - p)) / sqrt((p^2 + (1 - p)^2)) =
///                         p / sqrt(p^2 + (1 - p)^2)
///   When outcome is 0 =>  (0 * p + (1 - 0) * (1 - p)) / sqrt((p^2 + (1 - p)^2)) =
///                         (1 - p) / sqrt(p^2 + (1 - p)^2)
///
pub fn spherical_score(prediction: f32, outcome: f32) -> f32 {
    let numerator = outcome * prediction + (1.0 - outcome) * (1.0 - prediction);
    let denominator = (prediction.powi(2) + (1.0 - prediction).powi(2)).sqrt();
    numerator / denominator
}

/// Convert a Spherical score to a letter grade.
pub fn abs_spherical_letter_grade(_score: &f32) -> String {
    "TODO".to_string()
}

/// Convert a Spherical score to a letter grade.
pub fn rel_spherical_letter_grade(_score: &f32) -> String {
    "TODO".to_string()
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
        assert_approx_eq(spherical_score(0.5, 0.0), 1.0 / 2.0_f32.sqrt());
        assert_approx_eq(spherical_score(0.5, 1.0), 1.0 / 2.0_f32.sqrt());
        assert_approx_eq(spherical_score(1.0 / 3.0, 0.0), 2.0 / 5.0_f32.sqrt());
        assert_approx_eq(spherical_score(2.0 / 3.0, 1.0), 2.0 / 5.0_f32.sqrt());
        assert_approx_eq(spherical_score(2.0 / 3.0, 0.0), 1.0 / 5.0_f32.sqrt());
        assert_approx_eq(spherical_score(1.0 / 3.0, 1.0), 1.0 / 5.0_f32.sqrt());
    }

    #[test]
    /// Test with prediction values near extremes
    fn test_prediction_extremes() {
        assert_approx_eq(spherical_score(0.00001, 1.0), 0.00001);
        assert_approx_eq(spherical_score(0.99999, 0.0), 0.00001);
        assert_approx_eq(spherical_score(0.1, 0.0), 9.0 / 82.0_f32.sqrt());
        assert_approx_eq(spherical_score(0.9, 1.0), 9.0 / 82.0_f32.sqrt());
        // Values past this point essentially round directly to one.
        assert_approx_eq(spherical_score(0.01, 0.0), 99.0 / (13.0 * 58.0_f32.sqrt()));
        assert_approx_eq(spherical_score(0.99, 1.0), 99.0 / (13.0 * 58.0_f32.sqrt()));
        assert_approx_eq(spherical_score(0.001, 0.0), 999.0 / 998_002.0_f32.sqrt());
        assert_approx_eq(spherical_score(0.999, 1.0), 999.0 / 998_002.0_f32.sqrt());
        assert_approx_eq(
            spherical_score(0.0001, 0.0),
            9999.0 / 99_980_002.0_f32.sqrt(),
        );
        assert_approx_eq(
            spherical_score(0.9999, 1.0),
            9999.0 / 99_980_002.0_f32.sqrt(),
        );
        assert_approx_eq(spherical_score(0.00001, 0.0), 1.0);
        assert_approx_eq(spherical_score(0.99999, 1.0), 1.0);
    }

    #[test]
    /// Test best and worst possible predictions
    fn test_edge_cases() {
        assert_approx_eq(spherical_score(1.0, 1.0), 1.0);
        assert_approx_eq(spherical_score(0.0, 0.0), 1.0);
        assert_approx_eq(spherical_score(0.0, 1.0), 0.0);
        assert_approx_eq(spherical_score(1.0, 0.0), 0.0);
    }

    #[test]
    /// Test with partial outcomes (neither 0 nor 1)
    fn test_partial_outcomes() {
        assert_approx_eq(spherical_score(1.0 / 3.0, 1.0 / 3.0), 5.0_f32.sqrt() / 3.0);
        assert_approx_eq(
            spherical_score(1.0 / 3.0, 2.0 / 3.0),
            4.0 / (3.0 * 5.0_f32.sqrt()),
        );
        assert_approx_eq(
            spherical_score(2.0 / 3.0, 1.0 / 3.0),
            4.0 / (3.0 * 5.0_f32.sqrt()),
        );
        assert_approx_eq(spherical_score(0.5, 0.5), 1.0 / 2.0_f32.sqrt());
    }
}
