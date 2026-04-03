//! Utilities for the X-Ray module.

pub mod sample;

// Re-export types from lib.rs for backwards compatibility
pub use crate::{
    HistoryChart, HistoryChartPoint, ImpactType, Link, MarketData, MarketStatus, MarketTitle,
    MarketType, OutcomeBinary, OutcomeContinuous, OutcomeDate, OutcomeDiscrete, OutcomeNumeric,
    Outcomes, Probability, Url, XRayAnalysis, XRayAssessment, XRayConfidenceAspect,
};
