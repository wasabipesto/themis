pub mod manifold;
use clap::ValueEnum;
use serde::Serialize;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize)]
pub enum Platform {
    Kalshi,
    Manifold,
    Metaculus,
    Polymarket,
    PredictIt,
}

#[derive(Debug, Serialize)]
pub struct MarketForDB {
    title: String,
    platform: Platform,
    platform_id: String,
}
