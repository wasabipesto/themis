//! Questions are groups of markets from

use crate::outcome::Outcome;
use anyhow::Result;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Unique ID for the question
pub type QuestionId = u32;

/// Question information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Question {
    pub id: QuestionId,
    pub category_slug: String,
    pub start_date_override: Option<NaiveDate>,
    pub end_date_override: Option<NaiveDate>,
}

/// Details on the linked market.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OutcomeQuestionLink {
    pub outcome: Outcome,
    pub question_invert: bool,
}

impl Question {
    /// Get outcomes associated with this question.
    pub fn get_outcomes(&self) -> Result<Vec<OutcomeQuestionLink>> {
        todo!()
    }
}
