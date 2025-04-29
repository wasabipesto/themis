//! Convert accuracy scores into "letter grades".
//!
//! These are intended to be an easy-to-read, intuitive representation of how
//! good a score is. For instance, Brier scores are better when they are lower,
//! which is not intuitive for many people. Logarithmic scores can have extreme
//! values without necessarily representing extreme underperformance.

use crate::scores::brier;
use crate::scores::logarithmic;
use crate::scores::spherical;
use crate::scores::RelativeScoreType;

use super::AbsoluteScoreType;

/// Calculate the letter grade for the absolute market score given the prediction
/// and outcome. All absolute score types will have the same letter grade.
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
        AbsoluteScoreType::BrierAverage => score,
        AbsoluteScoreType::BrierMidpoint => score,
        AbsoluteScoreType::BrierBeforeClose30d => score,
        AbsoluteScoreType::LogarithmicAverage => {
            brier::brier_score(logarithmic::invert_log_score(score), 1.0)
        }
        AbsoluteScoreType::LogarithmicMidpoint => {
            brier::brier_score(logarithmic::invert_log_score(score), 1.0)
        }
        AbsoluteScoreType::LogarithmicBeforeClose30d => {
            brier::brier_score(logarithmic::invert_log_score(score), 1.0)
        }
        AbsoluteScoreType::SphericalAverage => {
            brier::brier_score(spherical::invert_spherical_score(score), 1.0)
        }
        AbsoluteScoreType::SphericalMidpoint => {
            brier::brier_score(spherical::invert_spherical_score(score), 1.0)
        }
        AbsoluteScoreType::SphericalBeforeClose30d => {
            brier::brier_score(spherical::invert_spherical_score(score), 1.0)
        }
    };

    match brier_score {
        x if x < 0.0001 => "S".to_string(),
        x if x < 0.0009 => "A+".to_string(),
        x if x < 0.0018 => "A".to_string(),
        x if x < 0.0022 => "A-".to_string(),
        x if x < 0.0030 => "B+".to_string(),
        x if x < 0.0045 => "B".to_string(),
        x if x < 0.0055 => "B-".to_string(),
        x if x < 0.0075 => "C+".to_string(),
        x if x < 0.015 => "C".to_string(),
        x if x < 0.025 => "C-".to_string(),
        x if x < 0.050 => "D+".to_string(),
        x if x < 0.110 => "D".to_string(),
        x if x < 0.250 => "D-".to_string(),
        x if x <= 1.0 => "F".to_string(),
        _ => "ERROR".to_string(),
    }
}

/// Calculate the letter grade for the relative market score.
///
/// The relative scoring algorithm we use results in a lot of scores very close
/// to zero with a sharp dropoff and roughly-symmetrical curve on either side.
///
/// We will calculate our grade cutoffs so that C+ is centered at zero, with a
/// reference grade "bin" width based on the score type. And of course the Brier
/// score needs to be inverted.
///
pub fn relative_letter_grade(score_type: &RelativeScoreType, score: f32) -> String {
    let (width, invert) = match score_type {
        RelativeScoreType::BrierRelative => (0.002, true),
        RelativeScoreType::LogarithmicRelative => (0.006, false),
        RelativeScoreType::SphericalRelative => (0.002, false),
    };
    match invert {
        false => match score {
            x if x > 35.0 * width => "S".to_string(),
            x if x > 20.0 * width => "A+".to_string(),
            x if x > 8.0 * width => "A".to_string(),
            x if x > 5.0 * width => "A-".to_string(),
            x if x > 4.0 * width => "B+".to_string(),
            x if x > 2.0 * width => "B".to_string(),
            x if x > 1.0 * width => "B-".to_string(),
            x if x > 0.0 * width => "C+".to_string(),
            x if x > -1.0 * width => "C".to_string(),
            x if x > -2.0 * width => "C-".to_string(),
            x if x > -4.0 * width => "D+".to_string(),
            x if x > -8.0 * width => "D".to_string(),
            x if x > -12.0 * width => "D-".to_string(),
            x if x >= -100.0 * width => "F".to_string(),
            _ => "ERROR".to_string(),
        },
        true => match score {
            x if x < -35.0 * width => "S".to_string(),
            x if x < -20.0 * width => "A+".to_string(),
            x if x < -8.0 * width => "A".to_string(),
            x if x < -5.0 * width => "A-".to_string(),
            x if x < -4.0 * width => "B+".to_string(),
            x if x < -2.0 * width => "B".to_string(),
            x if x < -1.0 * width => "B-".to_string(),
            x if x < 0.0 * width => "C+".to_string(),
            x if x < 1.0 * width => "C".to_string(),
            x if x < 2.0 * width => "C-".to_string(),
            x if x < 4.0 * width => "D+".to_string(),
            x if x < 8.0 * width => "D".to_string(),
            x if x < 12.0 * width => "D-".to_string(),
            x if x <= 100.0 * width => "F".to_string(),
            _ => "ERROR".to_string(),
        },
    }
}
