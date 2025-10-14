//! Convert accuracy scores into "letter grades".
//!
//! These are intended to be an easy-to-read, intuitive representation of how
//! good a score is. For instance, Brier scores are better when they are lower,
//! which is not intuitive for many people. Logarithmic scores can have extreme
//! values without necessarily representing extreme underperformance.

use crate::scores::{brier, logarithmic, spherical, AbsoluteScoreType, RelativeScoreType};

/// Brier score cutoffs and their corresponding letter grades
pub const BRIER_ABSCORE_GRADES: [(f32, &str); 14] = [
    (0.0001, "S"),
    (0.0009, "A+"),
    (0.0064, "A"),
    (0.0100, "A-"),
    (0.0144, "B+"),
    (0.0320, "B"),
    (0.0400, "B-"),
    (0.0576, "C+"),
    (0.1296, "C"),
    (0.1600, "C-"),
    (0.1936, "D+"),
    (0.3136, "D"),
    (0.3600, "D-"),
    (1.0000, "F"),
];

/// Brier score cutoffs and their corresponding letter grades
pub const BRIER_RELSCORE_GRADES: [(f32, &str); 14] = [
    (-0.070, "S"),
    (-0.040, "A+"),
    (-0.016, "A"),
    (-0.006, "A-"),
    (-0.004, "B+"),
    (0.002, "B"),
    (0.003, "B-"),
    (0.005, "C+"),
    (0.010, "C"),
    (0.015, "C-"),
    (0.020, "D+"),
    (0.030, "D"),
    (0.040, "D-"),
    (1.000, "F"),
];

/// Calculate the letter grade for the absolute market score given the prediction
/// and outcome. The same prediction/resolution pair will have the same letter grade
/// no matter what score type is used.
///
/// I used to have separate bounds for each score type, basing each score cutoff
/// on the Nth percentile value for that score, but then the letter grades all
/// came out the same so I decided to cut out the middleman.
///
/// We convert each score type to a Brier score and then use those cutoffs to
/// determine the letter grade. Most people understand Brier scores more intuitively
/// than the others so this also makes it more audit-able.
///
pub fn absolute_letter_grade(score_type: &AbsoluteScoreType, score: f32) -> String {
    let brier_score = match score_type {
        AbsoluteScoreType::BrierAverage
        | AbsoluteScoreType::BrierMidpoint
        | AbsoluteScoreType::BrierBeforeClose7d
        | AbsoluteScoreType::BrierBeforeClose30d => score,
        AbsoluteScoreType::LogarithmicAverage
        | AbsoluteScoreType::LogarithmicMidpoint
        | AbsoluteScoreType::LogarithmicBeforeClose7d
        | AbsoluteScoreType::LogarithmicBeforeClose30d => {
            brier::brier_score(logarithmic::invert_log_score(score), 1.0)
        }
        AbsoluteScoreType::SphericalAverage
        | AbsoluteScoreType::SphericalMidpoint
        | AbsoluteScoreType::SphericalBeforeClose7d
        | AbsoluteScoreType::SphericalBeforeClose30d => {
            brier::brier_score(spherical::invert_spherical_score(score), 1.0)
        }
    };

    for &(cutoff, grade) in BRIER_ABSCORE_GRADES.iter() {
        if brier_score <= cutoff {
            return grade.to_string();
        }
    }
    "ERROR".to_string()
}

/// Calculate the letter grade for the relative market score.
///
/// The relative scoring algorithm we use results in a lot of scores very close
/// to zero with a sharp dropoff and roughly-symmetrical curve on either side.
///
/// Once again we will use the Brier scores as a reference, this time using coefficients
/// determined from sampling data.
///
#[allow(clippy::neg_multiply)]
pub fn relative_letter_grade(score_type: &RelativeScoreType, score: f32) -> String {
    let brier_rel_score = match score_type {
        RelativeScoreType::BrierRelative => score,
        RelativeScoreType::LogarithmicRelative => score * -3.0,
        RelativeScoreType::SphericalRelative => score * -1.0,
    };

    for &(cutoff, grade) in BRIER_RELSCORE_GRADES.iter() {
        if brier_rel_score <= cutoff {
            return grade.to_string();
        }
    }
    "ERROR".to_string()
}
