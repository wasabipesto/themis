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

/// Given a log score, and assuming that the resolution is 1, recreate the
/// probability of the event.
///
/// When outcome is 1, s = p / sqrt(p^2 + (1 - p)^2)
/// To solve for p,    p = (s^2 +/- sqrt(s^2 - s^4) / (2s^2 - 1))
/// In order to get probabilities between zero and one we use:
///                    p = (s^2 - sqrt(s^2 - s^4) / (2s^2 - 1))
///
/// We use this to generate letter grades, since we use Brier scores as the basis.
///
pub fn invert_spherical_score(score: f32) -> f32 {
    (score.powi(2) - (score.powi(2) - score.powi(4)).sqrt()) / (2.0 * score.powi(2) - 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to compare floating point values with tolerance
    fn assert_approx_eq(actual: f32, expected: f32, epsilon: f32) {
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
        let epsilon = 1e-6;
        assert_approx_eq(spherical_score(0.5, 0.0), 1.0 / 2.0_f32.sqrt(), epsilon);
        assert_approx_eq(spherical_score(0.5, 1.0), 1.0 / 2.0_f32.sqrt(), epsilon);
        assert_approx_eq(
            spherical_score(1.0 / 3.0, 0.0),
            2.0 / 5.0_f32.sqrt(),
            epsilon,
        );
        assert_approx_eq(
            spherical_score(2.0 / 3.0, 1.0),
            2.0 / 5.0_f32.sqrt(),
            epsilon,
        );
        assert_approx_eq(
            spherical_score(2.0 / 3.0, 0.0),
            1.0 / 5.0_f32.sqrt(),
            epsilon,
        );
        assert_approx_eq(
            spherical_score(1.0 / 3.0, 1.0),
            1.0 / 5.0_f32.sqrt(),
            epsilon,
        );
    }

    #[test]
    /// Test with prediction values near extremes
    fn test_prediction_extremes() {
        let epsilon = 1e-6;
        assert_approx_eq(spherical_score(0.00001, 1.0), 0.00001, epsilon);
        assert_approx_eq(spherical_score(0.99999, 0.0), 0.00001, epsilon);
        assert_approx_eq(spherical_score(0.1, 0.0), 9.0 / 82.0_f32.sqrt(), epsilon);
        assert_approx_eq(spherical_score(0.9, 1.0), 9.0 / 82.0_f32.sqrt(), epsilon);
        // Values past this point essentially round directly to one.
        assert_approx_eq(
            spherical_score(0.01, 0.0),
            99.0 / (13.0 * 58.0_f32.sqrt()),
            epsilon,
        );
        assert_approx_eq(
            spherical_score(0.99, 1.0),
            99.0 / (13.0 * 58.0_f32.sqrt()),
            epsilon,
        );
        assert_approx_eq(
            spherical_score(0.001, 0.0),
            999.0 / 998_002.0_f32.sqrt(),
            epsilon,
        );
        assert_approx_eq(
            spherical_score(0.999, 1.0),
            999.0 / 998_002.0_f32.sqrt(),
            epsilon,
        );
        assert_approx_eq(
            spherical_score(0.0001, 0.0),
            9999.0 / 99_980_002.0_f32.sqrt(),
            epsilon,
        );
        assert_approx_eq(
            spherical_score(0.9999, 1.0),
            9999.0 / 99_980_002.0_f32.sqrt(),
            epsilon,
        );
        assert_approx_eq(spherical_score(0.00001, 0.0), 1.0, epsilon);
        assert_approx_eq(spherical_score(0.99999, 1.0), 1.0, epsilon);
    }

    #[test]
    /// Test best and worst possible predictions
    fn test_edge_cases() {
        let epsilon = 1e-6;
        assert_approx_eq(spherical_score(1.0, 1.0), 1.0, epsilon);
        assert_approx_eq(spherical_score(0.0, 0.0), 1.0, epsilon);
        assert_approx_eq(spherical_score(0.0, 1.0), 0.0, epsilon);
        assert_approx_eq(spherical_score(1.0, 0.0), 0.0, epsilon);
    }

    #[test]
    /// Test with partial outcomes (neither 0 nor 1)
    fn test_partial_outcomes() {
        let epsilon = 1e-6;
        assert_approx_eq(
            spherical_score(1.0 / 3.0, 1.0 / 3.0),
            5.0_f32.sqrt() / 3.0,
            epsilon,
        );
        assert_approx_eq(
            spherical_score(1.0 / 3.0, 2.0 / 3.0),
            4.0 / (3.0 * 5.0_f32.sqrt()),
            epsilon,
        );
        assert_approx_eq(
            spherical_score(2.0 / 3.0, 1.0 / 3.0),
            4.0 / (3.0 * 5.0_f32.sqrt()),
            epsilon,
        );
        assert_approx_eq(spherical_score(0.5, 0.5), 1.0 / 2.0_f32.sqrt(), epsilon);
    }

    #[test]
    /// Test that the score inverts back to the original probability
    fn test_inversion() {
        // Epsilon here is larger due to poor precision around the extremes
        let epsilon = 1e-4;
        for i in 0..1000 {
            let prediction = i as f32 / 1000.0;
            let outcome = 1.0;
            let score = spherical_score(prediction, outcome);
            assert_approx_eq(invert_spherical_score(score), prediction, epsilon);
        }
    }
}
