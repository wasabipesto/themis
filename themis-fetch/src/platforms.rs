use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use clap::ValueEnum;
use core::fmt;
use diesel::{prelude::*, Insertable};
use futures::future::join_all;
use reqwest_leaky_bucket::leaky_bucket::RateLimiter;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde::{Deserialize, Serialize};
use serde_json;
use std::env::var;

pub mod kalshi;
pub mod manifold;

const DEFAULT_OPENING_PROB: f32 = 0.5;
const SECS_PER_DAY: f32 = (60 * 60 * 24) as f32;

/// All possible platforms that are supported by this application.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize)]
pub enum Platform {
    Kalshi,
    Manifold,
    //Metaculus,
    //Polymarket,
    //PredictIt,
}

// Diesel macro to save the markets to a datbase table.
table! {
    market (id) {
        id -> Int4,
        title -> Varchar,
        platform -> Varchar,
        platform_id -> Varchar,
        url -> Varchar,
        open_days -> Float,
        volume_usd -> Float,
        prob_at_midpoint -> Float,
        prob_at_close -> Float,
    }
}

/// The central market type that all platform-specific objects are converted into.
/// This is the object type that is sent to the database, file, or console.
#[derive(Debug, Serialize, Insertable)]
#[diesel(table_name = market)]
pub struct MarketForDB {
    title: String,
    platform: String,
    platform_id: String,
    url: String,
    open_days: f32,
    volume_usd: f32,
    prob_at_midpoint: f32,
    prob_at_close: f32,
}

/// Simple struct for market events. The timestamp declares when the probability became that value.
#[derive(Debug, Clone)]
pub struct MarketEvent {
    time: DateTime<Utc>,
    prob: f32,
}

/// Common traits used to massage platform-specific market objects into the standard types.
pub trait MarketFullDetails {
    /// Get the string representation of the market for debug pruposes.
    fn debug(&self) -> String;

    /// Get the market title (usually the question).
    fn title(&self) -> String;

    /// Get the platform name.
    fn platform(&self) -> String;

    /// Get the platform's internal ID for the market.
    fn platform_id(&self) -> String;

    /// Get the canonical URL for the market.
    fn url(&self) -> String;

    /// Get the time the market openend.
    fn open_dt(&self) -> Result<DateTime<Utc>, MarketConvertError>;

    /// Get the time the market closed.
    fn close_dt(&self) -> Result<DateTime<Utc>, MarketConvertError>;

    /// Get the total duration of the market in days.
    fn open_days(&self) -> Result<f32, MarketConvertError> {
        Ok((self.close_dt()? - self.open_dt()?).num_seconds() as f32 / SECS_PER_DAY)
    }

    /// Get the total traded market volume in USD.
    fn volume_usd(&self) -> f32;

    /// Get a list of probability-affecting events during the market (derived from bets/trades).
    fn events(&self) -> Vec<MarketEvent>;

    /// Get the market's probability at a specific time.
    /// If a time before the first event is requested, we use a default opening of 50%.
    /// Returns an error if a time before market open is requested.
    /// Returns the last traded value if a time after market close is requested.
    fn prob_at_time(&self, time: DateTime<Utc>) -> Result<f32, MarketConvertError> {
        if time < self.open_dt()? {
            // requested time is before market starts, throw error
            return Err(MarketConvertError::new(
                self.debug(),
                format!(
                    "Requested probability at {:?} before market open at {:?}.",
                    time,
                    self.open_dt()?
                )
                .as_str(),
            ));
        }
        let mut prev_prob = DEFAULT_OPENING_PROB;
        for event in self.events() {
            // once we find an after the requested time, return the prob from the previous event
            if event.time > time {
                return Ok(prev_prob);
            }
            prev_prob = event.prob;
        }
        match self.events().last() {
            // no bets, return the default
            None => Ok(DEFAULT_OPENING_PROB),
            // requested time is after the last bet, return the final prob
            Some(event) => Ok(event.prob),
        }
    }

    /// Get the market's probability at a specific time before closing.
    /// Returns None if a time before market open is requested.
    fn prob_duration_before_close(&self, dur: Duration) -> Result<Option<f32>, MarketConvertError> {
        let time = self.close_dt()? - dur;
        if time > self.open_dt()? {
            Ok(Some(self.prob_at_time(time)?))
        } else {
            Ok(None)
        }
    }

    /// Get the market's probability at a specific percent of the way though the duration of a market.
    fn prob_at_percent(&self, pct: f32) -> Result<f32, MarketConvertError> {
        let time = self.open_dt()?
            + Duration::seconds(
                ((self.close_dt()? - self.open_dt()?).num_seconds() as f32 * pct) as i64,
            );
        self.prob_at_time(time)
    }
}

/// Basic error type that returns the market as a debug string and a simple error message.
#[derive(Debug, Clone)]
pub struct MarketConvertError {
    data: String,
    message: String,
}
impl MarketConvertError {
    pub fn new(data: String, message: &str) -> Self {
        MarketConvertError {
            data,
            message: message.to_string(),
        }
    }
}
impl fmt::Display for MarketConvertError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Market Conversion Error: {}: {}",
            self.message, self.data
        )
    }
}

/// A default API client with middleware to ratelimit and retry on failure.
fn get_reqwest_client_ratelimited(rps: usize) -> ClientWithMiddleware {
    // retry requests that get server errors with an exponential backoff timer
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    // rate limit to n requests per 100ms sustained with a max burst of 10x that
    let req_per_100ms = rps / 10;
    let rate_limiter = RateLimiter::builder()
        .max(rps)
        .initial(0)
        .refill(req_per_100ms)
        .build();

    ClientBuilder::new(reqwest::Client::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .with(reqwest_leaky_bucket::rate_limit_all(rate_limiter))
        .build()
}
