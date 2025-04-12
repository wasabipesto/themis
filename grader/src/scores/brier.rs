//! Module containing Brier score calculations.

/// Calculate the Brier score given the prediction and the outcome.
pub fn brier_score(prediction: f32, outcome: f32) -> f32 {
    (prediction - outcome).powi(2)
}

/// Convert a Brier score to a letter grade.
pub fn abs_brier_letter_grade(score: &f32) -> String {
    match score {
        x if *x < 0.05 => "S".to_string(),
        x if *x < 0.06 => "A+".to_string(),
        x if *x < 0.09 => "A".to_string(),
        x if *x < 0.10 => "A-".to_string(),
        x if *x < 0.11 => "B+".to_string(),
        x if *x < 0.14 => "B".to_string(),
        x if *x < 0.15 => "B-".to_string(),
        x if *x < 0.16 => "C+".to_string(),
        x if *x < 0.19 => "C".to_string(),
        x if *x < 0.20 => "C-".to_string(),
        x if *x < 0.21 => "D+".to_string(),
        x if *x < 0.25 => "D".to_string(),
        x if *x < 0.26 => "D-".to_string(),
        x if *x <= 1.0 => "F".to_string(),
        _ => "ERROR".to_string(),
    }
}

/// Convert a Brier score to a letter grade.
pub fn rel_brier_letter_grade(score: &f32) -> String {
    match score {
        x if *x < -0.95 => "S".to_string(),
        x if *x < -0.75 => "A+".to_string(),
        x if *x < -0.55 => "A".to_string(),
        x if *x < -0.35 => "A-".to_string(),
        x if *x < -0.25 => "B+".to_string(),
        x if *x < -0.15 => "B".to_string(),
        x if *x < -0.05 => "B-".to_string(),
        x if *x < 0.0 => "C+".to_string(),
        x if *x < 0.05 => "C".to_string(),
        x if *x < 0.15 => "C-".to_string(),
        x if *x < 0.25 => "D+".to_string(),
        x if *x < 0.35 => "D".to_string(),
        x if *x < 0.55 => "D-".to_string(),
        x if *x <= 1.0 => "F".to_string(),
        _ => "ERROR".to_string(),
    }
}
