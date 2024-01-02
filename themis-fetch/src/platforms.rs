use chrono::{DateTime, NaiveDateTime, Utc};
use clap::ValueEnum;
use core::fmt;
use diesel::prelude::*;
use diesel::{Insertable, Queryable};
use reqwest_leaky_bucket::leaky_bucket::RateLimiter;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde::{Deserialize, Serialize};
use std::env::var;

pub mod kalshi;
pub mod manifold;

const SECS_PER_DAY: f32 = (60 * 60 * 24) as f32;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize)]
pub enum Platform {
    Kalshi,
    Manifold,
    Metaculus,
    Polymarket,
    PredictIt,
}

#[derive(Debug, Serialize, Insertable, Queryable)]
#[diesel(table_name = market)]
pub struct MarketForDB {
    title: String,
    platform: String,
    platform_id: String,
    url: String,
    open_days: f32,
    volume_usd: f32,
}

pub trait MarketInfoDetails {
    fn is_valid(&self) -> bool;
}

pub trait MarketFullDetails {
    fn title(&self) -> String;
    fn platform(&self) -> String;
    fn platform_id(&self) -> String;
    fn url(&self) -> String;
    fn open_date(&self) -> Result<DateTime<Utc>, MarketConvertError>;
    fn close_date(&self) -> Result<DateTime<Utc>, MarketConvertError>;
    fn open_days(&self) -> Result<f32, MarketConvertError> {
        Ok((self.close_date()? - self.open_date()?).num_seconds() as f32 / SECS_PER_DAY)
    }
    fn volume_usd(&self) -> f32;
}

#[derive(Debug, Clone)]
pub struct MarketConvertError {
    message: String,
    market: String,
}
impl MarketConvertError {
    pub fn new(market: String, message: &str) -> Self {
        MarketConvertError {
            message: message.to_string(),
            market,
        }
    }
}
impl fmt::Display for MarketConvertError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Market Conversion Error: {}: {}",
            self.message, self.market
        )
    }
}

table! {
    market (id) {
        id -> Int4,
        title -> Varchar,
        platform -> Varchar,
        platform_id -> Varchar,
        url -> Varchar,
        open_days -> Float,
        volume_usd -> Float,
    }
}

fn get_default_client() -> ClientWithMiddleware {
    // retry requests that get server errors with an exponential backoff timer
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    // rate limit to 2 requests per 100ms (20rps) sustained with a max burst of 100 requests
    let rate_limiter = RateLimiter::builder().max(100).initial(0).refill(2).build();

    ClientBuilder::new(reqwest::Client::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .with(reqwest_leaky_bucket::rate_limit_all(rate_limiter))
        .build()
}
