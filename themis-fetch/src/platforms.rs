use clap::ValueEnum;
use core::fmt;
use serde::{Deserialize, Serialize};
use std::env::var;

pub mod kalshi;
pub mod manifold;

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
    url: String,
}

#[derive(Debug, Clone)]
pub struct MarketConvertError;
impl fmt::Display for MarketConvertError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "error during market conversion process")
    }
}

fn get_default_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::new()
}
