//! Module containing score types and their implementations.

use anyhow::{anyhow, Result};
use log::{error, warn};
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::fmt::{self, Display};

use crate::{
    helpers, Category, CriterionProbabilityPoint, DailyProbabilityPoint, Market, Platform, Question,
};

pub mod brier;
pub mod lettergrade;
pub mod logarithmic;
pub mod relative;
pub mod spherical;

/// Possible absolute score types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScoreType {
    Absolute(AbsoluteScoreType),
    Relative(RelativeScoreType),
}
impl Display for ScoreType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScoreType::Absolute(abs_type) => write!(f, "{}", abs_type),
            ScoreType::Relative(rel_type) => write!(f, "{}", rel_type),
        }
    }
}
impl Serialize for ScoreType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
impl ScoreType {
    /// List of all possible score types.
    pub fn all() -> Vec<ScoreType> {
        let mut score_types = AbsoluteScoreType::all()
            .into_iter()
            .map(ScoreType::Absolute)
            .collect::<Vec<_>>();

        score_types.extend(
            RelativeScoreType::all()
                .into_iter()
                .map(ScoreType::Relative),
        );

        score_types
    }
    /// Get the grade for a market using this score type.
    pub fn get_grade(&self, score: f32) -> String {
        match self {
            ScoreType::Absolute(absolute_score_type) => absolute_score_type.get_grade(score),
            ScoreType::Relative(relative_score_type) => relative_score_type.get_grade(score),
        }
    }
}

/// Absolute score types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbsoluteScoreType {
    BrierAverage,
    BrierMidpoint,
    BrierBeforeClose7d,
    BrierBeforeClose30d,
    LogarithmicAverage,
    LogarithmicMidpoint,
    LogarithmicBeforeClose7d,
    LogarithmicBeforeClose30d,
    SphericalAverage,
    SphericalMidpoint,
    SphericalBeforeClose7d,
    SphericalBeforeClose30d,
}
impl Display for AbsoluteScoreType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AbsoluteScoreType::BrierAverage => "brier-average",
            AbsoluteScoreType::BrierMidpoint => "brier-midpoint",
            AbsoluteScoreType::BrierBeforeClose7d => "brier-before-close-days-7",
            AbsoluteScoreType::BrierBeforeClose30d => "brier-before-close-days-30",
            AbsoluteScoreType::LogarithmicAverage => "logarithmic-average",
            AbsoluteScoreType::LogarithmicMidpoint => "logarithmic-midpoint",
            AbsoluteScoreType::LogarithmicBeforeClose7d => "logarithmic-before-close-days-7",
            AbsoluteScoreType::LogarithmicBeforeClose30d => "logarithmic-before-close-days-30",
            AbsoluteScoreType::SphericalAverage => "spherical-average",
            AbsoluteScoreType::SphericalMidpoint => "spherical-midpoint",
            AbsoluteScoreType::SphericalBeforeClose7d => "spherical-before-close-days-7",
            AbsoluteScoreType::SphericalBeforeClose30d => "spherical-before-close-days-30",
        };
        write!(f, "{}", s)
    }
}
impl Serialize for AbsoluteScoreType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
impl AbsoluteScoreType {
    /// List of all possible absolute score types.
    pub fn all() -> Vec<AbsoluteScoreType> {
        vec![
            AbsoluteScoreType::BrierAverage,
            AbsoluteScoreType::BrierMidpoint,
            AbsoluteScoreType::BrierBeforeClose7d,
            AbsoluteScoreType::BrierBeforeClose30d,
            AbsoluteScoreType::LogarithmicAverage,
            AbsoluteScoreType::LogarithmicMidpoint,
            AbsoluteScoreType::LogarithmicBeforeClose7d,
            AbsoluteScoreType::LogarithmicBeforeClose30d,
            AbsoluteScoreType::SphericalAverage,
            AbsoluteScoreType::SphericalMidpoint,
            AbsoluteScoreType::SphericalBeforeClose7d,
            AbsoluteScoreType::SphericalBeforeClose30d,
        ]
    }
    /// Whether this score type is optional or not.
    fn is_optional(&self) -> bool {
        matches!(
            self,
            AbsoluteScoreType::BrierBeforeClose7d
                | AbsoluteScoreType::BrierBeforeClose30d
                | AbsoluteScoreType::LogarithmicBeforeClose7d
                | AbsoluteScoreType::LogarithmicBeforeClose30d
                | AbsoluteScoreType::SphericalBeforeClose7d
                | AbsoluteScoreType::SphericalBeforeClose30d
        )
    }
    /// Get the criterion probability (prediction metric) to use for this score type.
    /// Returns None if the score is not found.
    pub fn get_criterion_prob(&self, criteron_probs: &[CriterionProbabilityPoint]) -> Option<f32> {
        match self {
            AbsoluteScoreType::BrierAverage
            | AbsoluteScoreType::LogarithmicAverage
            | AbsoluteScoreType::SphericalAverage => {
                helpers::get_first_probability(criteron_probs, "time-average").map(|p| p.prob)
            }
            AbsoluteScoreType::BrierMidpoint
            | AbsoluteScoreType::LogarithmicMidpoint
            | AbsoluteScoreType::SphericalMidpoint => {
                helpers::get_first_probability(criteron_probs, "midpoint").map(|p| p.prob)
            }
            AbsoluteScoreType::BrierBeforeClose7d
            | AbsoluteScoreType::LogarithmicBeforeClose7d
            | AbsoluteScoreType::SphericalBeforeClose7d => {
                helpers::get_first_probability(criteron_probs, "before-close-days-7")
                    .map(|p| p.prob)
            }
            AbsoluteScoreType::BrierBeforeClose30d
            | AbsoluteScoreType::LogarithmicBeforeClose30d
            | AbsoluteScoreType::SphericalBeforeClose30d => {
                helpers::get_first_probability(criteron_probs, "before-close-days-30")
                    .map(|p| p.prob)
            }
        }
    }
    /// Get the score for a market using this absolute score type.
    /// Returns None if the score is not found.
    pub fn get_score(
        &self,
        market: &Market,
        criteron_probs: &[CriterionProbabilityPoint],
    ) -> Option<f32> {
        match self {
            AbsoluteScoreType::BrierAverage
            | AbsoluteScoreType::BrierMidpoint
            | AbsoluteScoreType::BrierBeforeClose7d
            | AbsoluteScoreType::BrierBeforeClose30d => Some(brier::brier_score(
                self.get_criterion_prob(criteron_probs)?,
                market.resolution,
            )),
            AbsoluteScoreType::LogarithmicAverage
            | AbsoluteScoreType::LogarithmicMidpoint
            | AbsoluteScoreType::LogarithmicBeforeClose7d
            | AbsoluteScoreType::LogarithmicBeforeClose30d => Some(logarithmic::log_score(
                self.get_criterion_prob(criteron_probs)?,
                market.resolution,
            )),
            AbsoluteScoreType::SphericalAverage
            | AbsoluteScoreType::SphericalMidpoint
            | AbsoluteScoreType::SphericalBeforeClose7d
            | AbsoluteScoreType::SphericalBeforeClose30d => Some(spherical::spherical_score(
                self.get_criterion_prob(criteron_probs)?,
                market.resolution,
            )),
        }
    }
    /// Score a market using this absolute score type.
    pub fn score_market(
        &self,
        market: &Market,
        criteron_probs: &[CriterionProbabilityPoint],
    ) -> Result<Option<MarketScore>> {
        match (self.get_score(market, criteron_probs), self.is_optional()) {
            (Some(score), _) => {
                let grade = self.get_grade(score);
                Ok(Some(MarketScore {
                    market_id: market.id.clone(),
                    score_type: ScoreType::Absolute(self.clone()),
                    score,
                    grade,
                }))
            }
            (None, true) => Ok(None),
            (None, false) => Err(anyhow!("criterion probability {self} missing")),
        }
    }
    /// Get the grade for a market using this absolute score type.
    pub fn get_grade(&self, score: f32) -> String {
        lettergrade::absolute_letter_grade(self, score)
    }
}

/// Relative score types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelativeScoreType {
    BrierRelative,
    LogarithmicRelative,
    SphericalRelative,
}
impl Display for RelativeScoreType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            RelativeScoreType::BrierRelative => "brier-relative",
            RelativeScoreType::LogarithmicRelative => "logarithmic-relative",
            RelativeScoreType::SphericalRelative => "spherical-relative",
        };
        write!(f, "{}", s)
    }
}
impl Serialize for RelativeScoreType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
impl RelativeScoreType {
    /// List of all relative score types.
    pub fn all() -> Vec<RelativeScoreType> {
        vec![
            RelativeScoreType::BrierRelative,
            RelativeScoreType::LogarithmicRelative,
            RelativeScoreType::SphericalRelative,
        ]
    }
    /// Score a market using this relative score type.
    pub fn score_market(
        &self,
        question: &Question,
        markets: &[Market],
        probs: &[DailyProbabilityPoint],
    ) -> Result<Vec<MarketScore>> {
        relative::score_market(self, question, markets, probs)
    }
    /// Get the score for a market using this relative score type.
    pub fn get_score(&self, prediction: f32, outcome: f32) -> f32 {
        match self {
            RelativeScoreType::BrierRelative => brier::brier_score(prediction, outcome),
            RelativeScoreType::LogarithmicRelative => logarithmic::log_score(prediction, outcome),
            RelativeScoreType::SphericalRelative => spherical::spherical_score(prediction, outcome),
        }
    }
    /// Get the grade for a market using this relative score type.
    pub fn get_grade(&self, score: f32) -> String {
        lettergrade::relative_letter_grade(self, score)
    }
}

/// Market-question scores.
#[derive(Debug, Serialize, Clone)]
pub struct MarketScore {
    pub market_id: String,
    pub score_type: ScoreType,
    pub score: f32,
    pub grade: String,
}

/// Platform-category scores.
#[derive(Debug, Serialize, Clone)]
pub struct PlatformCategoryScore {
    pub platform_slug: String,
    pub category_slug: String,
    pub score_type: ScoreType,
    pub num_markets: usize,
    pub score: Option<f32>,
    pub grade: Option<String>,
}

/// Other scores.
#[derive(Debug, Serialize, Clone)]
pub struct OtherScore {
    pub item_type: String,
    pub item_id: String,
    pub score_type: ScoreType,
    pub num_markets: usize,
    pub score: Option<f32>,
    pub grade: Option<String>,
}

/// Calculate and return all absolute scores for a market.
pub fn calculate_absolute_scores(
    markets: &[Market],
    criterion_probs: Vec<CriterionProbabilityPoint>,
) -> Result<Vec<MarketScore>> {
    // Index the criterion probabilities by market ID to optimize lookup times.
    let mut crit_prob_map = HashMap::with_capacity(criterion_probs.len());
    for prob in criterion_probs {
        crit_prob_map
            .entry(prob.market_id.to_owned())
            .or_insert_with(Vec::new)
            .push(prob);
    }

    let score_types = AbsoluteScoreType::all();
    let mut scores = Vec::with_capacity(markets.len() * score_types.len());

    for market in markets {
        log::trace!("Calculating absolute scores for market {}", market.id);

        // Retrieve criterion probabilities associated with the current market ID.
        if let Some(market_criterion_probs) = crit_prob_map.get(&market.id) {
            for score_type in &score_types {
                match score_type.score_market(market, market_criterion_probs) {
                    Ok(Some(market_score)) => scores.push(market_score),
                    Ok(None) => continue,
                    Err(e) => error!(
                        "Error calculating absolute scores for market {}: {}",
                        market.id, e
                    ),
                }
            }
        } else {
            warn!("No criterion probabilities found for market {}", market.id);
        }
    }

    Ok(scores)
}

/// Calculate and return all absolute scores for a market.
pub fn calculate_relative_scores(
    questions: &[Question],
    markets: &[Market],
    probs: &[DailyProbabilityPoint],
) -> Result<Vec<MarketScore>> {
    let score_types = RelativeScoreType::all();
    let mut scores = Vec::with_capacity(markets.len() * score_types.len());

    for question in questions {
        log::trace!("Calculating relative scores for question {}", question.id);

        // Filter to markets for the current question
        let question_markets: Vec<Market> = markets
            .iter()
            .filter(|m| m.question_id == Some(question.id))
            .cloned()
            .collect();

        // Get the market IDs for this question
        let question_markets_ids: Vec<String> =
            question_markets.iter().map(|m| m.id.clone()).collect();

        // Filter to probs for the current question
        let question_probs: Vec<DailyProbabilityPoint> = probs
            .iter()
            .filter(|p| question_markets_ids.contains(&p.market_id))
            .cloned()
            .collect();

        for score_type in &score_types {
            match score_type.score_market(question, &question_markets, &question_probs) {
                Ok(mut market_scores) => scores.append(&mut market_scores),
                Err(e) => error!(
                    "Error calculating relative scores for question {}: {e}",
                    question.id
                ),
            }
        }
    }
    Ok(scores)
}

fn average_platform_category_scores(
    platform_slug: &str,
    category_slug: &str,
    score_type: &ScoreType,
    market_scores: &[MarketScore],
) -> PlatformCategoryScore {
    if !market_scores.is_empty() {
        // Average the scores
        let average_score =
            market_scores.iter().map(|s| s.score).sum::<f32>() / market_scores.len() as f32;
        PlatformCategoryScore {
            platform_slug: platform_slug.to_string(),
            category_slug: category_slug.to_string(),
            score_type: score_type.clone(),
            num_markets: market_scores.len(),
            score: Some(average_score),
            grade: Some(score_type.get_grade(average_score)),
        }
    } else {
        PlatformCategoryScore {
            platform_slug: platform_slug.to_string(),
            category_slug: category_slug.to_string(),
            score_type: score_type.clone(),
            num_markets: 0,
            score: None,
            grade: None,
        }
    }
}

fn average_other_scores(
    item_type: &str,
    item_id: &str,
    score_type: &ScoreType,
    market_scores: &[MarketScore],
) -> OtherScore {
    if !market_scores.is_empty() {
        // Average the scores
        let average_score =
            market_scores.iter().map(|s| s.score).sum::<f32>() / market_scores.len() as f32;
        OtherScore {
            item_type: item_type.to_string(),
            item_id: item_id.to_string(),
            score_type: score_type.clone(),
            num_markets: market_scores.len(),
            score: Some(average_score),
            grade: Some(score_type.get_grade(average_score)),
        }
    } else {
        OtherScore {
            item_type: item_type.to_string(),
            item_id: item_id.to_string(),
            score_type: score_type.clone(),
            num_markets: 0,
            score: None,
            grade: None,
        }
    }
}

pub fn aggregate_platform_category_scores(
    platforms: &[Platform],
    categories: &[Category],
    questions: &[Question],
    markets: &[Market],
    market_scores: &[MarketScore],
) -> (Vec<PlatformCategoryScore>, Vec<OtherScore>) {
    // Link questions, markets, and scores together for filtering
    struct QuestionsMarketsAndScores<'a> {
        question: &'a Question,
        market: &'a Market,
        score: &'a MarketScore,
    }
    let mut markets_and_scores = Vec::new();
    for market in markets {
        let question = questions
            .iter()
            .find(|q| market.question_id == Some(q.id))
            .unwrap();
        for score in market_scores {
            if score.market_id == market.id {
                markets_and_scores.push(QuestionsMarketsAndScores {
                    question,
                    market,
                    score,
                });
            }
        }
    }

    let mut platform_category_scores = Vec::new();
    let mut other_overall_scores = Vec::new();

    // Average scores by platform x category
    for platform in platforms {
        for category in categories {
            for score_type in ScoreType::all() {
                // Collect scores that match the platform, category, and score type
                let filtered_market_scores: Vec<MarketScore> = markets_and_scores
                    .iter()
                    .filter(|item| {
                        item.market.platform_slug == platform.slug
                            && item.question.category_slug == category.slug
                            && item.score.score_type == score_type
                    })
                    .map(|item| item.score.clone())
                    .collect();

                // Average the scores and push
                platform_category_scores.push(average_platform_category_scores(
                    &platform.slug,
                    &category.slug,
                    &score_type,
                    &filtered_market_scores,
                ));
            }
        }
    }

    // Average overall per platform
    for platform in platforms {
        for score_type in ScoreType::all() {
            let filtered_market_scores: Vec<MarketScore> = markets_and_scores
                .iter()
                .filter(|item| {
                    item.market.platform_slug == platform.slug
                        && item.score.score_type == score_type
                })
                .map(|item| item.score.clone())
                .collect();
            other_overall_scores.push(average_other_scores(
                "platform",
                &platform.slug,
                &score_type,
                &filtered_market_scores,
            ));
        }
    }
    // Average overall per category
    for category in categories {
        for score_type in ScoreType::all() {
            let filtered_market_scores: Vec<MarketScore> = markets_and_scores
                .iter()
                .filter(|item| {
                    item.question.category_slug == category.slug
                        && item.score.score_type == score_type
                })
                .map(|item| item.score.clone())
                .collect();
            other_overall_scores.push(average_other_scores(
                "category",
                &category.slug,
                &score_type,
                &filtered_market_scores,
            ));
        }
    }
    // Average overall per question
    for question in questions {
        for score_type in ScoreType::all() {
            let filtered_market_scores: Vec<MarketScore> = markets_and_scores
                .iter()
                .filter(|item| {
                    item.question.id == question.id && item.score.score_type == score_type
                })
                .map(|item| item.score.clone())
                .collect();
            other_overall_scores.push(average_other_scores(
                "question",
                &question.id.to_string(),
                &score_type,
                &filtered_market_scores,
            ));
        }
    }

    (platform_category_scores, other_overall_scores)
}
