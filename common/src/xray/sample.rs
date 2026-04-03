//! Generate sample data for testing and benchmarking.

// Disable pedantic lints for this module since it's mostly placeholder.
#![allow(clippy::pedantic)]

use anyhow::{Result, anyhow};
use chrono::{Duration, Utc};
use lipsum::{lipsum_title_with_rng, lipsum_with_rng, lipsum_words_with_rng};
use rand::Rng;
use sluggify::sluggify::sluggify;

use crate::platforms::Platform;
use crate::{
    HistoryChart, ImpactType, Label, Link, MarketData, MarketStatus, MarketTitle, MarketType,
    OutcomeBinary, OutcomeContinuous, OutcomeDate, OutcomeDiscrete, OutcomeNumeric, Outcomes,
    Probability, Url, XRayAnalysis, XRayAssessment, XRayConfidenceAspect,
};

fn sample_market_outcomes(market_type: &MarketType) -> Result<Outcomes> {
    let mut rng = rand::rng();
    let outcomes = match market_type {
        MarketType::Binary => Outcomes::Binary(OutcomeBinary {
            probability: Probability::new(rng.random_range(0.0..=1.0))?,
        }),
        MarketType::DiscreteOne => {
            let num_outcomes = rng.random_range(3..=10);
            let mut outcomes = Vec::with_capacity(num_outcomes);
            let mut raw_probs = Vec::with_capacity(num_outcomes);

            // Generate random probabilities
            for _ in 0..num_outcomes {
                raw_probs.push(rng.random_range(0.1..=10.0));
            }

            // Normalize to sum to 1.0
            let sum: f64 = raw_probs.iter().sum();
            for raw_prob in raw_probs {
                outcomes.push(OutcomeDiscrete {
                    label: Label(lipsum_title_with_rng(&mut rng)),
                    probability: Probability::new((raw_prob / sum) as f32)?,
                });
            }

            Outcomes::Discrete(outcomes)
        }
        MarketType::DiscreteMulti => {
            let num_outcomes = rng.random_range(3..=10);
            let mut outcomes = Vec::with_capacity(num_outcomes);

            for _ in 0..num_outcomes {
                outcomes.push(OutcomeDiscrete {
                    label: Label(lipsum_title_with_rng(&mut rng)),
                    probability: Probability::new(rng.random_range(0.0..=1.0))?,
                });
            }

            Outcomes::Discrete(outcomes)
        }
        MarketType::Numeric => {
            let num_outcomes = rng.random_range(3..=10);
            let mut outcomes = Vec::with_capacity(num_outcomes);

            // Choose natural schelling points for the range
            let schelling_points = [
                (0.0, 100.0),
                (0.0, 1000.0),
                (0.0, 10000.0),
                (1.0, 10.0),
                (10.0, 100.0),
                (100.0, 1000.0),
                (1000.0, 10000.0),
                (0.0, 50.0),
                (50.0, 150.0),
            ];
            let (min_val, max_val) = schelling_points[rng.random_range(0..schelling_points.len())];
            let range = max_val - min_val;
            let step = range / num_outcomes as f64;

            for i in 0..num_outcomes {
                let low = min_val + (i as f64 * step);
                let high = min_val + ((i + 1) as f64 * step);
                let mid = (low + high) / 2.0;

                outcomes.push(OutcomeNumeric {
                    label: Label(lipsum_title_with_rng(&mut rng)),
                    numerical_strike_low: low as f32,
                    numerical_strike_midpoint: mid as f32,
                    numerical_strike_high: high as f32,
                    probability: Probability::new(rng.random_range(0.0..=1.0))?,
                });
            }

            Outcomes::Numeric(outcomes)
        }
        MarketType::Date => {
            let num_outcomes = rng.random_range(3..=10);
            let mut outcomes = Vec::with_capacity(num_outcomes);

            // Choose natural schelling points for date ranges
            let base_date = Utc::now();
            let date_ranges = [
                Duration::days(30),   // 1 month
                Duration::days(90),   // 3 months
                Duration::days(180),  // 6 months
                Duration::days(365),  // 1 year
                Duration::days(730),  // 2 years
                Duration::days(1825), // 5 years
            ];
            let total_range = date_ranges[rng.random_range(0..date_ranges.len())];
            let step = total_range / num_outcomes as i32;

            for i in 0..num_outcomes {
                let low = base_date + (step * i as i32);
                let high = base_date + (step * (i + 1) as i32);
                let mid = low + (high - low) / 2;

                outcomes.push(OutcomeDate {
                    label: Label(lipsum_title_with_rng(&mut rng)),
                    date_strike_low: low,
                    date_strike_midpoint: mid,
                    date_strike_high: high,
                    probability: Probability::new(rng.random_range(0.0..=1.0))?,
                });
            }

            Outcomes::Date(outcomes)
        }
        MarketType::Continuous => {
            // Choose natural schelling points for min and max
            let schelling_ranges = [
                (0.0, 100.0),
                (0.0, 1000.0),
                (1.0, 10.0),
                (10.0, 100.0),
                (100.0, 1000.0),
                (0.0, 50.0),
                (50.0, 150.0),
                (0.0, 500.0),
            ];
            let (min_val, max_val) = schelling_ranges[rng.random_range(0..schelling_ranges.len())];

            // Random peak between min and max
            let peak = rng.random_range(min_val..=max_val);

            Outcomes::Continuous(OutcomeContinuous {
                label: Label(lipsum_title_with_rng(&mut rng)),
                continuous_min: min_val as f32,
                continuous_peak: peak as f32,
                continuous_max: max_val as f32,
                probability: Probability::new(rng.random_range(0.5..=1.0))?,
            })
        }
    };
    Ok(outcomes)
}

fn sample_market_data() -> Result<MarketData> {
    let mut rng = rand::rng();
    let platform = Platform::Kalshi;
    let title_text = lipsum_title_with_rng(&mut rng);
    let description = format!(
        "{}\n {}",
        lipsum_with_rng(&mut rng, 50),
        lipsum_with_rng(&mut rng, 30)
    );
    let url = Url(format!(
        "https://kalshi.com/markets/{}",
        sluggify(&title_text, None)
    ));
    let market_type = match rng.random_range(1..=100) {
        1..=20 => MarketType::Binary,
        21..=40 => MarketType::DiscreteOne,
        41..=60 => MarketType::DiscreteMulti,
        61..=80 => MarketType::Continuous,
        81..=90 => MarketType::Numeric,
        91..=100 => MarketType::Date,
        _ => MarketType::Binary,
    };
    let volume_usd = match rng.random_range(1..=100) {
        1..=10 => None,
        _ => Some(rng.random_range(100.0..=1_000_000.0)),
    };
    let unique_traders = match rng.random_range(1..=100) {
        1..=10 => None,
        _ => Some(rng.random_range(10..=10_000)),
    };
    let open_datetime = Utc::now() - Duration::hours(rng.random_range(100..=50_000));
    let expected_close_datetime = Utc::now() + Duration::hours(rng.random_range(100..=50_000));
    let last_updated_datetime = Utc::now() - Duration::hours(rng.random_range(100..=1000));
    let tags = (0..5).map(|_| lipsum_words_with_rng(&mut rng, 2)).collect();
    let outcomes = sample_market_outcomes(&market_type)?;

    let data = MarketData {
        platform_name: platform.into(),
        title: MarketTitle(title_text),
        url,
        description,
        market_status: MarketStatus::default(),
        market_type,
        volume_usd,
        unique_traders,
        open_datetime,
        expected_close_datetime,
        last_updated_datetime,
        tags,
        outcomes,
    };
    Ok(data)
}

fn sample_confidence_aspects(_market: &MarketData) -> Result<Vec<XRayConfidenceAspect>> {
    let data = vec![
        XRayConfidenceAspect {
            icon: "mdi:arrow-collapse-horizontal".into(),
            title: "Narrow bid-ask spread".into(),
            aspect_name: "narrow-bid-ask-spread".into(),
            impact_type: ImpactType::Positive,
            impact_amount: 10.0,
            description: "A small gap between \"bid\" and \"ask\" orders means that traders are confident in the current probability. Currently the bid-ask spread is just 1 cent, the lowest it can be.".into(),
            links: Vec::new(),
        },
        XRayConfidenceAspect {
            icon: "mdi:trophy-variant".into(),
            title: "Good past performance on Survivor questions".into(),
            aspect_name: "past-performance-in-niche".into(),
            impact_type: ImpactType::Positive,
            impact_amount: 10.0,
            description: "Kalshi has 14 past markets about Survivor with a cumulative trade volume of $1,148,345. One month prior to resolution, those markets were on average 11% away from the actual resolution value.".into(),
            links: Vec::new(),
        },
        XRayConfidenceAspect {
            icon: "mdi:handshake".into(),
            title: "Other prediction markets agree".into(),
            aspect_name: "arbitrage".into(),
            impact_type: ImpactType::Positive,
            impact_amount: 10.0,
            description: "Polymarket has their own market on this question with $34,125 in volume and an estimate within 5% of this one.".into(),
            links: vec![Link {preface: None, text: "See it here.".into(), url: Url("https://polymarket.com/event/survivor-49-winner".into())}],
        },
        XRayConfidenceAspect {
            icon: "mdi:clock-fast".into(),
            title: "Expected to resolve soon".into(),
            aspect_name: "resolution-timeframe".into(),
            impact_type: ImpactType::Positive,
            impact_amount: 5.0,
            description: "This market is scheduled to close in less than a week, which means traders can invest without locking up capital for a long time.".into(),
            links: Vec::new(),
        },
        XRayConfidenceAspect {
            icon: "mdi:chart-line-variant".into(),
            title: "Price has been stable for 30 days".into(),
            aspect_name: "price-stability".into(),
            impact_type: ImpactType::Positive,
            impact_amount: 5.0,
            description: "The price hasn't drifted more than 10% in the last month, even though the market has been liquid the entire time.".into(),
            links: Vec::new(),
        },
        XRayConfidenceAspect {
            icon: "mdi:waves".into(),
            title: "Good market depth".into(),
            aspect_name: "overall-market-depth".into(),
            impact_type: ImpactType::Positive,
            impact_amount: 0.0,
            description: "There are $14,323 of limit orders within 5% of the current price, which is very high for Kalshi and indicates high confidence. If you had insider information, you could wager $10,000 to win up to $104,365.".into(),
            links: Vec::new(),
        },
        XRayConfidenceAspect {
            icon: "mdi:percent".into(),
            title: "Kalshi return on investment".into(),
            aspect_name: "kalshi-return-on-investment".into(),
            impact_type: ImpactType::Positive,
            impact_amount: 0.0,
            description: "Kalshi offers 3.25% interest on held positions. On longer markets this would be expected to improve accuracy but this market will resolve soon so the point is moot.".into(),
            links: Vec::new(),
        },
        XRayConfidenceAspect {
            icon: "mdi:gavel".into(),
            title: "Unambiguous resolution criteria".into(),
            aspect_name: "resolution-criteria".into(),
            impact_type: ImpactType::Positive,
            impact_amount: 0.0,
            description: "The resolution criteria is very clear and covers edge cases clearly. We should expect that the market's resolution will seem fair in hindsight.".into(),
            links: Vec::new(),
        },
        XRayConfidenceAspect {
            icon: "mdi:gift".into(),
            title: "Moderate liquidity rewards".into(),
            aspect_name: "liquidity-rewards".into(),
            impact_type: ImpactType::Positive,
            impact_amount: 0.0,
            description: "Kalshi provides liquidity rewards for at least one contract within this question, with a pool of $300 for those with open orders. This is moderate for Kalshi.".into(),
            links: Vec::new(),
        },
        XRayConfidenceAspect {
            icon: "mdi:volume-low".into(),
            title: "Low recent trade volume".into(),
            aspect_name: "recent-trade-volume".into(),
            impact_type: ImpactType::Negative,
            impact_amount: -5.0,
            description: "Trade volume in the last week is low compared to other points in this market's history. This could indicate that new information has yet to be priced into this market.".into(),
            links: Vec::new(),
        },
        XRayConfidenceAspect {
            icon: "mdi:account-group-outline".into(),
            title: "Low number of unique traders".into(),
            aspect_name: "total-unique-traders".into(),
            impact_type: ImpactType::Negative,
            impact_amount: -5.0,
            description: "The majority of trades on this market are from just 24 accounts. This is low for Kalshi and could indicate poor visibility.".into(),
            links: Vec::new(),
        },
        XRayConfidenceAspect {
            icon: "mdi:account-alert".into(),
            title: "Many new or unprofitable traders".into(),
            aspect_name: "trader-profiling".into(),
            impact_type: ImpactType::Neutral,
            impact_amount: 0.0,
            description: "There are 13 instances of new users betting large amounts on this market, which could affect the price significantly. Additionally, there are 2 instances of users with very poor past performance shifting this market by more than 3%. However, all of these have been corrected by other traders.".into(),
            links: Vec::new(),
        },
        XRayConfidenceAspect {
            icon: "mdi:alert-circle".into(),
            title: "Potential misinformation in comments".into(),
            aspect_name: "comment-misinformation".into(),
            impact_type: ImpactType::Neutral,
            impact_amount: 0.0,
            description: "Analysis of the comment section shows that some users may be attempting to manipulate the market through misinformation.".into(),
            links: Vec::new(),
        },
        XRayConfidenceAspect {
            icon: "mdi:newspaper-variant-outline".into(),
            title: "Inconsistent news coverage".into(),
            aspect_name: "news-coverage".into(),
            impact_type: ImpactType::Neutral,
            impact_amount: 0.0,
            description: "Recent news articles seem to indicate that the probability may be more uncertain than this market indicates.".into(),
            links: Vec::new(),
        },
    ];
    Ok(data)
}

fn sample_assessment_data(confidence_aspects: &[XRayConfidenceAspect]) -> Result<XRayAssessment> {
    // Get confidence_percent from confidence_aspects
    let confidence_percent = 50.0
        + confidence_aspects
            .iter()
            .map(|aspect| aspect.impact_amount)
            .sum::<f32>()
            .clamp(0.0, 100.0);

    // Build final assessment based on confidence_percent
    match confidence_percent {
        0.0..40.0 => Ok(XRayAssessment {
            title: "Low Confidence".to_string(),
            description: String::default(),
            confidence_percent,
        }),
        40.0..80.0 => Ok(XRayAssessment {
            title: "Medium Confidence".to_string(),
            description: String::default(),
            confidence_percent,
        }),
        80.0..100.0 => Ok(XRayAssessment {
            title: "High Confidence".to_string(),
            description: String::default(),
            confidence_percent,
        }),
        _ => Err(anyhow!("Invalid confidence percent")),
    }
}

fn sample_history_chart(_market: &MarketData) -> Result<HistoryChart> {
    let data = HistoryChart {
        title: "History Chart".to_string(),
        points: vec![],
    };
    Ok(data)
}

fn sample_similar_markets(count: usize) -> Result<Vec<MarketData>> {
    let mut data = Vec::with_capacity(count);
    for _ in 0..count {
        data.push(sample_market_data()?);
    }
    Ok(data)
}

#[allow(clippy::field_reassign_with_default)]
pub fn sample_xray_analysis() -> Result<XRayAnalysis> {
    let market = sample_market_data()?;
    let confidence_aspects = sample_confidence_aspects(&market)?;
    let assessment = sample_assessment_data(&confidence_aspects)?;
    let history_chart = sample_history_chart(&market)?;
    let similar_markets = sample_similar_markets(5)?;

    let data = XRayAnalysis {
        market,
        assessment,
        confidence_aspects,
        history_chart,
        similar_markets,
    };
    Ok(data)
}
