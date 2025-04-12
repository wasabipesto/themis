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
    let score_clamped = score.max(f32::MIN);
    if &serde_json::to_string(&score_clamped).unwrap() == "null" {
        log::error!("Invalid score: P:{prediction} & O:{outcome} = {score}");
    }
    assert!(score_clamped <= 0.0);
    assert!(!score_clamped.is_nan());
    assert!(score_clamped.is_finite());
    assert!(&serde_json::to_string(&score_clamped).unwrap() != "null");
    score_clamped
}

/// Convert a Logarithmic score to a letter grade.
pub fn abs_log_letter_grade(_score: &f32) -> String {
    "TODO".to_string()
}

/// Convert a Logarithmic score to a letter grade.
pub fn rel_log_letter_grade(_score: &f32) -> String {
    "TODO".to_string()
}
