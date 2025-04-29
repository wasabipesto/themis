#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! anyhow = "1.0"
//! serde = { version = "1.0", features = ["derive"] }
//! serde_json = "1.0"
//! themis-grader = { path = "../grader" }
//! ```

use anyhow::Result;
use serde::Serialize;
use themis_grader::scores::lettergrade::BRIER_SCORE_GRADES;
use themis_grader::scores::{brier, logarithmic, spherical};

#[derive(Debug, Serialize)]
struct GradeData {
    grade: String,
    scores: Vec<ScoreData>,
}
#[derive(Debug, Serialize)]
struct ScoreData {
    key: String,
    min: f32,
    max: f32,
}

fn main() -> Result<()> {
    println!("Input map: {:?}", BRIER_SCORE_GRADES);

    let mut results: Vec<GradeData> = Vec::new();

    let mut prev_cutoff = 0f32;
    for &(cutoff, grade) in BRIER_SCORE_GRADES.iter() {
        let mut grade_data = GradeData {
            grade: grade.to_string(),
            scores: Vec::new(),
        };

        println!("Grade {grade}");
        let min_prob = brier::invert_brier_score(prev_cutoff);
        let max_prob = brier::invert_brier_score(cutoff);

        let name = "brier";
        let min_score = prev_cutoff;
        let max_score = cutoff;
        println!("  {name:12} {min_score:+.4} - {max_score:+.4}");
        grade_data.scores.push(ScoreData {
            key: name.to_string(),
            min: min_score,
            max: max_score,
        });

        let name = "logarithmic";
        let min_score = logarithmic::log_score(min_prob, 1.0);
        let max_score = logarithmic::log_score(max_prob, 1.0);
        println!("  {name:12} {min_score:+.4} - {max_score:+.4}");
        grade_data.scores.push(ScoreData {
            key: name.to_string(),
            min: min_score,
            max: max_score,
        });

        let name = "spherical";
        let min_score = spherical::spherical_score(min_prob, 1.0);
        let max_score = spherical::spherical_score(max_prob, 1.0);
        println!("  {name:12} {min_score:+.4} - {max_score:+.4}");
        grade_data.scores.push(ScoreData {
            key: name.to_string(),
            min: min_score,
            max: max_score,
        });

        let name = "probability";
        let min_score = 1.0 - min_prob;
        let max_score = 1.0 - max_prob;
        println!("  {name:12} {min_score:+.4} - {max_score:+.4}",);
        grade_data.scores.push(ScoreData {
            key: name.to_string(),
            min: min_score,
            max: max_score,
        });

        results.push(grade_data);
        prev_cutoff = cutoff;
    }

    println!("\n\n{}", serde_json::to_string(&results)?);
    Ok(())
}
