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
}

table! {
    market (id) {
        id -> Int4,
        title -> Varchar,
        platform -> Varchar,
        platform_id -> Varchar,
        url -> Varchar,
    }
}

#[derive(Debug, Clone)]
pub struct MarketConvertError;
impl fmt::Display for MarketConvertError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "error during market conversion process")
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
