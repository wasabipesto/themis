//! Module containing score types and their implementations.

use std::collections::{HashMap, HashSet};

use crate::{helpers, DailyProbabilityPoint, Market, MarketWithProbability, Question};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use log::error;
use serde::{Deserialize, Serialize};

pub mod brier;

/// Possible absolute score types.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ScoreType {
    Absolute(AbsoluteScoreType),
    Relative(RelativeScoreType),
}

/// Absolute score types.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AbsoluteScoreType {
    BrierAverage,
    BrierMidpoint,
}
impl AbsoluteScoreType {
    /// List of all possible absolute score types.
    pub fn all() -> Vec<AbsoluteScoreType> {
        vec![
            AbsoluteScoreType::BrierAverage,
            AbsoluteScoreType::BrierMidpoint,
        ]
    }
    /// Score a market using this absolute score type.
    pub fn score_market(&self, market: &Market) -> Result<MarketScore> {
        let score = self.get_score(market);
        let grade = self.get_grade(&score);
        Ok(MarketScore {
            market_id: market.id.clone(),
            score_type: ScoreType::Absolute(self.clone()),
            score,
            grade,
        })
    }
    /// Get the score for a market using this absolute score type.
    pub fn get_score(&self, market: &Market) -> f32 {
        match self {
            AbsoluteScoreType::BrierAverage => {
                brier::brier_score(&market.prob_time_avg, &market.resolution)
            }
            AbsoluteScoreType::BrierMidpoint => {
                brier::brier_score(&market.prob_at_midpoint, &market.resolution)
            }
        }
    }
    /// Get the grade for a market using this absolute score type.
    pub fn get_grade(&self, score: &f32) -> String {
        match self {
            AbsoluteScoreType::BrierAverage => brier::abs_brier_letter_grade(score),
            AbsoluteScoreType::BrierMidpoint => brier::abs_brier_letter_grade(score),
        }
    }
}

/// Relative score types.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RelativeScoreType {
    BrierRelative,
}
impl RelativeScoreType {
    /// List of all relative score types.
    pub fn all() -> Vec<RelativeScoreType> {
        vec![RelativeScoreType::BrierRelative]
    }
    /// Score a market using this relative score type.
    /// This uses the methodology described here:
    /// https://www.cultivatelabs.com/crowdsourced-forecasting-guide/what-are-relative-brier-scores-and-how-are-they-calculated
    pub fn score_market(
        &self,
        question: &Question,
        markets: &[Market],
        probs: &[DailyProbabilityPoint],
    ) -> Result<Vec<MarketScore>> {
        // Check that >=2 markets are provided
        if markets.len() < 2 {
            return Err(anyhow!(
                "At least two markets are required for relative scoring, {} markets provided",
                markets.len()
            ));
        }

        // Check that all markets resolved the same direction
        let resolution = if let Some(first_market) = markets.first() {
            match first_market.question_invert {
                Some(true) => 1.0 - first_market.resolution,
                Some(false) => first_market.resolution,
                None => {
                    return Err(anyhow!(
                        "Market {} has no question invert attribute provided",
                        first_market.id
                    ))
                }
            }
        } else {
            return Err(anyhow!("No markets provided"));
        };
        for market in markets {
            let market_resolution = match market.question_invert {
                Some(true) => 1.0 - market.resolution,
                Some(false) => market.resolution,
                None => {
                    return Err(anyhow!(
                        "Market {} has no question invert attribute provided",
                        market.id
                    ))
                }
            };
            if market_resolution != resolution {
                return Err(anyhow!(
                    "Market {} resolved differently than consensus",
                    market.id
                ));
            }
        }

        // Get override bounds as DateTime<Utc>
        let start_date_override = question
            .start_date_override
            .map(|date| date.and_hms_opt(0, 0, 0).unwrap().and_utc());
        let end_date_override = question
            .end_date_override
            .map(|date| date.and_hms_opt(0, 0, 0).unwrap().and_utc() + Duration::days(1));

        // Connect probs to markets and sort by date
        let markets_with_probs: Vec<MarketWithProbability> = markets
            .iter()
            .map(|market| {
                // Filter out probabilities for this market
                let mut market_probs: Vec<DailyProbabilityPoint> = probs
                    .iter()
                    .filter(|p| p.market_id == market.id)
                    .cloned()
                    .collect();
                let prob_count_unfiltered = market_probs.len();

                // Filter out probability points outside of override bounds
                if let Some(start_date) = start_date_override {
                    market_probs.retain(|p| p.date >= start_date);
                }
                if let Some(end_date) = end_date_override {
                    market_probs.retain(|p| p.date <= end_date);
                }

                // Confirm that there are probabilities for this market
                if market_probs.is_empty() {
                    if prob_count_unfiltered == 0 {
                        return Err(anyhow!("No probabilities found for market {}", market.id));
                    } else {
                        return Err(anyhow!(
                            "All probabilities for market {} are outside of the override bounds",
                            market.id
                        ));
                    }
                }

                // Sort probabilities by date
                market_probs.sort_by(|a, b| a.date.cmp(&b.date));

                Ok(MarketWithProbability {
                    market: market.clone(),
                    probs: market_probs,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        // Collect start & end dates from each market
        let mut start_dates: Vec<DateTime<Utc>> = markets_with_probs
            .iter()
            .map(|m| m.probs.first().unwrap().date)
            .collect();
        let mut end_dates: Vec<DateTime<Utc>> = markets_with_probs
            .iter()
            .map(|m| m.probs.last().unwrap().date)
            .collect();

        // Start scoring after the second market starts
        start_dates.sort_by_key(|date| *date);
        let start_date = start_dates.get(1).unwrap().to_owned();

        // End scoring after the second-to-last market ends
        end_dates.sort_by_key(|date| *date);
        end_dates.reverse();
        let end_date = end_dates.get(1).unwrap().to_owned();

        // Get list of days to evaluate
        let days: HashSet<DateTime<Utc>> = probs
            .iter()
            .filter(|p| p.date >= start_date && p.date <= end_date)
            .map(|p| p.date)
            .collect();

        // Set up relative score map
        let mut relative_scores: HashMap<String, Vec<f64>> =
            markets.iter().map(|m| (m.id.clone(), Vec::new())).collect();

        // For each day:
        //  Get each market's score.
        //  Get the median score across all markets for that day.
        //  Get each market's relative score (score - median).
        for day in days {
            let mut daily_market_absolute_scores = HashMap::with_capacity(markets.len());
            for market in markets {
                // Get the market's probability point for the current day
                let market_prob_point =
                    if let Some(market_prob_point) = probs.iter().find(|p| p.date == day) {
                        market_prob_point
                    } else {
                        // If no probability point is found, skip this market
                        continue;
                    };

                // Invert the predicted probability if necessary
                let prediction = match market.question_invert {
                    Some(true) => 1.0 - market_prob_point.prob,
                    Some(false) => market_prob_point.prob,
                    None => {
                        return Err(anyhow!(
                            "Market {} has no question invert attribute provided",
                            market.id
                        ))
                    }
                };

                // Get the score for the market on this day
                let score = self.get_score(&prediction, &resolution);
                daily_market_absolute_scores.insert(market.id.clone(), score);
            }

            // Get median score for the current day
            let scores = daily_market_absolute_scores
                .values()
                .cloned()
                .collect::<Vec<f32>>();
            let median = helpers::median(&scores);

            // Subtract the median from each score to get the relative scores for each market
            let daily_market_relative_scores: HashMap<String, f32> = daily_market_absolute_scores
                .iter()
                .map(|(market_id, abs_score)| (market_id.clone(), abs_score - median))
                .collect();

            // Add the relative scores to the overall relative scores
            for (market_id, relative_score) in daily_market_relative_scores {
                if let Some(scores) = relative_scores.get_mut(&market_id) {
                    scores.push(relative_score as f64);
                }
            }
        }

        // Then, sum the relative scores and divide by the total number of days.
        let overall_market_relative_score: HashMap<String, f32> = relative_scores
            .iter()
            .filter_map(|(market_id, scores)| {
                if scores.is_empty() {
                    None
                } else {
                    let score = scores.iter().sum::<f64>() as f32 / scores.len() as f32;
                    Some((market_id.clone(), score))
                }
            })
            .collect();

        // Set up the result scores
        let result_scores = overall_market_relative_score
            .iter()
            .map(|(market_id, score)| MarketScore {
                market_id: market_id.clone(),
                score_type: ScoreType::Relative(self.clone()),
                score: *score,
                grade: self.get_grade(score),
            })
            .collect();

        Ok(result_scores)
    }
    /// Get the score for a market using this relative score type.
    pub fn get_score(&self, prediction: &f32, outcome: &f32) -> f32 {
        match self {
            RelativeScoreType::BrierRelative => brier::brier_score(prediction, outcome),
        }
    }
    /// Get the grade for a market using this relative score type.
    pub fn get_grade(&self, score: &f32) -> String {
        match self {
            RelativeScoreType::BrierRelative => brier::rel_brier_letter_grade(score),
        }
    }
}

/// Market-question scores.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarketScore {
    pub market_id: String,
    pub score_type: ScoreType,
    pub score: f32,
    pub grade: String,
}

/// Platform-category scores.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlatformScore {
    pub platform_slug: String,
    pub category_slug: String,
    pub score_type: ScoreType,
    pub num_markets: usize,
    pub score: f32,
    pub grade: String,
}

/// Calculate and return all absolute scores for a market.
pub fn calculate_absolute_scores(markets: &[Market]) -> Result<Vec<MarketScore>> {
    let score_types = AbsoluteScoreType::all();
    let mut scores = Vec::with_capacity(markets.len() * score_types.len());

    for market in markets {
        for score_type in &score_types {
            match score_type.score_market(market) {
                Ok(market_score) => scores.push(market_score),
                Err(e) => error!(
                    "Error calculating absolute scores for market {}: {e}",
                    market.id
                ),
            }
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
