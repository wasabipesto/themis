//! Generate sample data for testing and benchmarking.

use anyhow::{Result, anyhow};
use chrono::DateTime;
use lipsum::{lipsum_title_with_rng, lipsum_with_rng, lipsum_words_with_rng};
use sluggify::sluggify::sluggify;

use crate::{
    HistoryChart, ImpactType, MarketData, MarketStatus, MarketTitle, MarketType, Platform, Url,
    XRayAnalysis, XRayAssessment, XRayConfidenceAspect,
};

fn sample_market_data() -> Result<MarketData> {
    let mut rng = rand::rng();
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
    let tags = (0..5).map(|_| lipsum_words_with_rng(&mut rng, 2)).collect();

    let data = MarketData {
        platform_name: Platform::Kalshi.into(),
        title: MarketTitle(title_text),
        url,
        description,
        market_status: MarketStatus::default(),
        market_type: MarketType::default(),
        volume_usd: None,
        unique_traders: None,
        open_datetime: DateTime::default(),
        expected_close_datetime: DateTime::default(),
        last_updated_datetime: DateTime::default(),
        tags,
        outcomes: Vec::new(),
    };
    Ok(data)
}

fn sample_confidence_aspects(_market: &MarketData) -> Result<Vec<XRayConfidenceAspect>> {
    let data = vec![XRayConfidenceAspect {
        icon: "mdi:arrow-collapse-horizontal".into(),
        title: "Narrow bid-ask spread".into(),
        aspect_name: "bid-ask-spread".into(),
        impact_type: ImpactType::Positive,
        impact_amount: 10.0,
        description: "A small gap between \"bid\" and \"ask\" orders means that traders are confident in the current probability. Currently the bid-ask spread is just 1 cent, the lowest it can be.".into(),
        links: Vec::new(),
    }];
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
        0.0..=40.0 => Ok(XRayAssessment {
            title: "Low Confidence".to_string(),
            description: String::default(),
            confidence_percent,
        }),
        40.0..=80.0 => Ok(XRayAssessment {
            title: "Medium Confidence".to_string(),
            description: String::default(),
            confidence_percent,
        }),
        80.0..=100.0 => Ok(XRayAssessment {
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
