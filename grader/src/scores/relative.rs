//! Module with relative score calculations.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use log::error;
use std::collections::{HashMap, HashSet};

use crate::helpers;
use crate::scores::{MarketScore, RelativeScoreType, ScoreType};
use crate::{DailyProbabilityPoint, Market, MarketWithProbs, Question};

/// Score a market using the specified relative score type.
///
/// This uses the methodology described here:
/// https://www.cultivatelabs.com/crowdsourced-forecasting-guide/what-are-relative-brier-scores-and-how-are-they-calculated
///
/// For each day in the scoring period, we calculate each market's score, then
/// the baseline score, then the difference between the two (relative daily score).
/// The relative score is each market's sum of relative daily scores divided by
/// the total number of days in the scoring period.
///
pub fn score_market(
    score_type: &RelativeScoreType,
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
    let markets_with_probs: Vec<MarketWithProbs> = markets
        .iter()
        .map(|market| {
            // Filter out probabilities for this market
            let mut daily_probs: Vec<DailyProbabilityPoint> = probs
                .iter()
                .filter(|p| p.market_id == market.id)
                .cloned()
                .collect();
            let prob_count_unfiltered = daily_probs.len();

            // Filter out probability points outside of override bounds
            if let Some(start_date) = start_date_override {
                daily_probs.retain(|p| p.date >= start_date);
            }
            if let Some(end_date) = end_date_override {
                daily_probs.retain(|p| p.date <= end_date);
            }

            // Confirm that there are probabilities for this market
            if daily_probs.is_empty() {
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
            daily_probs.sort_by(|a, b| a.date.cmp(&b.date));

            Ok(MarketWithProbs {
                market: market.clone(),
                daily_probs,
                criterion_probs: Vec::new(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // Collect start & end dates from each market
    let mut start_dates: Vec<DateTime<Utc>> = markets_with_probs
        .iter()
        .map(|m| m.daily_probs.first().unwrap().date)
        .collect();
    let mut end_dates: Vec<DateTime<Utc>> = markets_with_probs
        .iter()
        .map(|m| m.daily_probs.last().unwrap().date)
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
    //  Get the baseline score across all markets for that day.
    //  Get each market's relative score (score - baseline).
    for day in days {
        let mut daily_market_absolute_scores = HashMap::with_capacity(markets.len());
        for market in markets {
            // Get the market's probability point for the current day
            let market_prob_point = if let Some(market_prob_point) = probs
                .iter()
                .find(|p| p.date == day && p.market_id == market.id)
            {
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
            let score = score_type.get_score(prediction, resolution);
            daily_market_absolute_scores.insert(market.id.clone(), score);
        }

        // Get baseline score for the current day
        let scores = daily_market_absolute_scores
            .values()
            .cloned()
            .collect::<Vec<f32>>();
        let baseline = helpers::median(&scores);

        // Subtract the baseline from each score to get the relative scores for each market
        let daily_market_relative_scores: HashMap<String, f32> = daily_market_absolute_scores
            .iter()
            .map(|(market_id, abs_score)| (market_id.clone(), abs_score - baseline))
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
                if &serde_json::to_string(&score).unwrap() == "null" {
                    error!(
                        "{market_id} {score_type} score ({score}) serializes to null: {:?}",
                        scores
                    );
                    return None;
                }
                Some((market_id.clone(), score))
            }
        })
        .collect();

    // Set up the result scores
    let result_scores = overall_market_relative_score
        .iter()
        .map(|(market_id, score)| MarketScore {
            market_id: market_id.clone(),
            score_type: ScoreType::Relative(score_type.clone()),
            score: *score,
            grade: score_type.get_grade(*score),
        })
        .collect();

    Ok(result_scores)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn create_test_question(
        start_date_override: Option<NaiveDate>,
        end_date_override: Option<NaiveDate>,
    ) -> Question {
        Question {
            id: 1234,
            category_slug: "test_question".to_string(),
            start_date_override,
            end_date_override,
        }
    }

    fn create_test_market(id: &str, resolution: f32, invert: bool) -> Market {
        Market {
            id: id.to_string(),
            platform_slug: "test_platform".to_string(),
            category_slug: Some("test_category".to_string()),
            question_id: Some(1234),
            question_invert: Some(invert),
            open_datetime: Utc::now(),
            close_datetime: Utc::now(),
            traders_count: None,
            volume_usd: None,
            duration_days: 1234,
            resolution,
        }
    }

    fn create_test_prob(market_id: &str, date: &str, prob: f32) -> DailyProbabilityPoint {
        DailyProbabilityPoint {
            market_id: market_id.to_string(),
            date: NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc(),
            prob,
        }
    }

    #[test]
    fn test_basic_relative_scoring() {
        let markets = vec![
            create_test_market("market1", 1.0, false),
            create_test_market("market2", 1.0, false),
            create_test_market("market3", 1.0, false),
        ];

        let probs = vec![
            // Market 1 probabilities
            create_test_prob("market1", "2023-01-01", 0.8),
            create_test_prob("market1", "2023-01-02", 0.9),
            // Market 2 probabilities
            create_test_prob("market2", "2023-01-01", 0.6),
            create_test_prob("market2", "2023-01-02", 0.7),
            // Market 3 probabilities
            create_test_prob("market3", "2023-01-01", 0.4),
            create_test_prob("market3", "2023-01-02", 0.5),
        ];

        let question = create_test_question(None, None);

        let result = score_market(
            &RelativeScoreType::BrierRelative,
            &question,
            &markets,
            &probs,
        )
        .unwrap();

        assert_eq!(result.len(), 3);

        // Market1 should have the best (lowest) relative Brier score since it was closest to resolution
        let market1_score = result.iter().find(|s| s.market_id == "market1").unwrap();
        let market2_score = result.iter().find(|s| s.market_id == "market2").unwrap();
        let market3_score = result.iter().find(|s| s.market_id == "market3").unwrap();

        assert!(market1_score.score < market2_score.score);
        assert!(market2_score.score < market3_score.score);
    }

    #[test]
    fn test_inverted_markets() {
        let markets = vec![
            create_test_market("market1", 0.0, true), // Inverted, actual resolution 1.0
            create_test_market("market2", 1.0, false), // Not inverted
        ];

        let probs = vec![
            create_test_prob("market1", "2023-01-01", 0.2), // Will be inverted to 0.8
            create_test_prob("market2", "2023-01-01", 0.8),
        ];

        let question = create_test_question(None, None);

        let result = score_market(
            &RelativeScoreType::BrierRelative,
            &question,
            &markets,
            &probs,
        )
        .unwrap();

        assert_eq!(result.len(), 2);
        // Both markets should have similar scores since their effective predictions were the same
        let score_diff = (result[0].score - result[1].score).abs();
        assert!(score_diff < 0.0001);
    }

    #[test]
    fn test_date_override_bounds() {
        let markets = vec![
            create_test_market("market1", 1.0, false),
            create_test_market("market2", 1.0, false),
        ];

        let probs = vec![
            create_test_prob("market1", "2023-01-01", 0.8),
            create_test_prob("market1", "2023-01-02", 0.9),
            create_test_prob("market2", "2023-01-01", 0.7),
            create_test_prob("market2", "2023-01-02", 0.8),
        ];

        let question = create_test_question(
            Some(NaiveDate::parse_from_str("2023-01-02", "%Y-%m-%d").unwrap()),
            None,
        );

        let result = score_market(
            &RelativeScoreType::BrierRelative,
            &question,
            &markets,
            &probs,
        )
        .unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_error_single_market() {
        let markets = vec![create_test_market("market1", 1.0, false)];

        let probs = vec![create_test_prob("market1", "2023-01-01", 0.8)];

        let question = create_test_question(None, None);

        let result = score_market(
            &RelativeScoreType::BrierRelative,
            &question,
            &markets,
            &probs,
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("At least two markets"));
    }

    #[test]
    fn test_error_conflicting_resolutions() {
        let markets = vec![
            create_test_market("market1", 1.0, false),
            create_test_market("market2", 0.0, false), // Different resolution
        ];

        let probs = vec![
            create_test_prob("market1", "2023-01-01", 0.8),
            create_test_prob("market2", "2023-01-01", 0.7),
        ];

        let question = create_test_question(None, None);

        let result = score_market(
            &RelativeScoreType::BrierRelative,
            &question,
            &markets,
            &probs,
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("resolved differently"));
    }

    #[test]
    fn test_error_missing_question_invert() {
        let mut market = create_test_market("market1", 1.0, false);
        market.question_invert = None;

        let markets = vec![market, create_test_market("market2", 1.0, false)];

        let probs = vec![
            create_test_prob("market1", "2023-01-01", 0.8),
            create_test_prob("market2", "2023-01-01", 0.7),
        ];

        let question = create_test_question(None, None);

        let result = score_market(
            &RelativeScoreType::BrierRelative,
            &question,
            &markets,
            &probs,
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("no question invert attribute"));
    }
}
