//! This module has all of the common utilities and market standardization tools required to query the API and convert responses into DB rows.

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use clap::ValueEnum;
use core::fmt;
use diesel::upsert::excluded;
use diesel::{pg::PgConnection, prelude::*, Connection, Insertable};
use futures::future::join_all;
use reqwest::header::{HeaderValue, AUTHORIZATION};
use reqwest::StatusCode;
use reqwest_chain::Chainer;
use reqwest_leaky_bucket::leaky_bucket::RateLimiter;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, Error};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::to_string_pretty;
use std::env::var;

pub mod kalshi;
pub mod manifold;
pub mod metaculus;

const DEFAULT_OPENING_PROB: f32 = 0.5;
const SECS_PER_DAY: f32 = (60 * 60 * 24) as f32;

/// All possible platforms that are supported by this application.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize)]
pub enum Platform {
    Kalshi,
    Manifold,
    Metaculus,
}

/// All possible methods to output markets.
#[derive(Debug, Copy, Clone, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputMethod {
    Database,
    Stdout,
    //File,
}

// Diesel macro to save the markets to a database table.
table! {
    market (id) {
        id -> Int4,
        title -> Varchar,
        platform -> Varchar,
        platform_id -> Varchar,
        url -> Varchar,
        open_days -> Float,
        volume_usd -> Float,
        num_traders -> Integer,
        prob_at_midpoint -> Float,
        prob_at_close -> Float,
        prob_time_weighted -> Float,
        resolution -> Float,
    }
}

/// The central market type that all platform-specific objects are converted into.
/// This is the object type that is sent to the database, file, or console.
#[derive(Debug, Serialize, Insertable, AsChangeset)]
#[diesel(table_name = market)]
pub struct MarketStandard {
    title: String,
    platform: String,
    platform_id: String,
    url: String,
    open_days: f32,
    volume_usd: f32,
    num_traders: i32,
    prob_at_midpoint: f32,
    prob_at_close: f32,
    prob_time_weighted: f32,
    resolution: f32,
}

/// Simple struct for market events. The timestamp declares when the probability became that value.
#[derive(Debug, Clone)]
pub struct ProbUpdate {
    time: DateTime<Utc>,
    prob: f32,
}

/// Common traits used to standardize platform-specific market objects into the standard types.
pub trait MarketStandardizer {
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

    /// Get the number of unique traders on the market.
    fn num_traders(&self) -> i32;

    /// Get a list of probability-affecting events during the market (derived from bets/trades).
    fn events(&self) -> Vec<ProbUpdate>;

    /// Get the actual resolved value (0 for no, 1 for yes, or in-between)
    fn resolution(&self) -> Result<f32, MarketConvertError>;

    /// Get the market's probability at a specific time.
    /// If a time before the first event is requested, we use a default opening of 50%.
    /// Returns an error if a time before market open is requested.
    /// Returns the last traded value if a time after market close is requested.
    fn prob_at_time(&self, time: DateTime<Utc>) -> Result<f32, MarketConvertError> {
        if time < self.open_dt()? {
            // requested time is before market starts, throw error
            return Err(MarketConvertError {
                data: self.debug(),
                message: format!(
                    "Requested probability at {:?} before market open at {:?}.",
                    time,
                    self.open_dt()?
                ),
            });
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

    /// Get the market's probability at a specific percent of the way though the duration of a market.
    fn prob_time_weighted(&self) -> Result<f32, MarketConvertError> {
        let mut prev_event: Option<ProbUpdate> = None;
        let mut cumulative_prob: f32 = 0.0;
        let mut cumulative_time: f32 = 0.0;
        for event in self.events() {
            match &prev_event {
                None => {
                    // add time between start time and first event
                    let duration = (event.time - self.open_dt()?).num_seconds() as f32;
                    cumulative_prob += DEFAULT_OPENING_PROB * duration;
                    cumulative_time += duration;
                }
                Some(prev) => {
                    // add time between last event and this one
                    let duration = (event.time - prev.time).num_seconds() as f32;
                    cumulative_prob += prev.prob * duration;
                    cumulative_time += duration;
                }
            }
            prev_event = Some(event);
        }
        match &prev_event {
            Some(prev) => {
                // add time between last event and close time
                // if the close time was moved to before the last event then we can't determine how long the last bet was valid for
                if self.close_dt()? > prev.time {
                    let duration = (self.close_dt()? - prev.time).num_seconds() as f32;
                    cumulative_prob += prev.prob * duration;
                    cumulative_time += duration;
                }
            }
            None => {
                // there are no events whatsoever, just assume it was the default throughout
                let duration = (self.close_dt()? - self.open_dt()?).num_seconds() as f32;
                cumulative_prob += DEFAULT_OPENING_PROB * duration;
                cumulative_time += duration;
            }
        }
        if cumulative_time > 10.0 {
            Ok(cumulative_prob / cumulative_time)
        } else {
            Err(MarketConvertError {
                data: self.debug(),
                message: format!(
                    "Market was only open for {cumulative_time} seconds, can't get proper prob_time_weighted."
                ),
            })
        }
    }
}

fn save_markets(markets: Vec<MarketStandard>, method: OutputMethod) {
    match method {
        OutputMethod::Database => {
            use crate::platforms::market::dsl::*;
            let mut conn = PgConnection::establish(
                &var("DATABASE_URL").expect("Required environment variable DATABASE_URL not set."),
            )
            .expect("Error connecting to datbase.");
            for chunk in markets.chunks(1000) {
                diesel::insert_into(market)
                    .values(chunk)
                    .on_conflict((platform, platform_id))
                    .do_update()
                    .set((
                        open_days.eq(excluded(open_days)),
                        volume_usd.eq(excluded(volume_usd)),
                        num_traders.eq(excluded(num_traders)),
                        prob_at_midpoint.eq(excluded(prob_at_midpoint)),
                        prob_at_close.eq(excluded(prob_at_close)),
                        prob_time_weighted.eq(excluded(prob_time_weighted)),
                        resolution.eq(excluded(resolution)),
                    ))
                    .execute(&mut conn)
                    .expect("Failed to insert rows into table.");
            }
        }
        OutputMethod::Stdout => {
            println!("{}", to_string_pretty(&markets).unwrap())
        }
    }
}

/// Basic error type that returns the market as a debug string and a simple error message.
#[derive(Debug, Clone)]
pub struct MarketConvertError {
    data: String,
    message: String,
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
    // rate limit to n requests per second
    let rate_limiter = RateLimiter::builder()
        .interval(std::time::Duration::from_millis(1000))
        .refill(rps)
        .max(rps)
        .build();

    ClientBuilder::new(reqwest::Client::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .with(reqwest_leaky_bucket::rate_limit_all(rate_limiter))
        .build()
}

/// Convert timestamp (milliseconds) to datetime and error on failure
fn get_datetime_from_millis(ts: i64) -> Result<DateTime<Utc>, ()> {
    let dt = NaiveDateTime::from_timestamp_millis(ts);
    match dt {
        Some(dt) => Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc)),
        None => Err(()),
    }
}

/// Convert timestamp (seconds) to datetime and error on failure
fn get_datetime_from_secs(ts: i64) -> Result<DateTime<Utc>, ()> {
    let dt = NaiveDateTime::from_timestamp_opt(ts, 0);
    match dt {
        Some(dt) => Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc)),
        None => Err(()),
    }
}
