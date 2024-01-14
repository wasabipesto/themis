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
pub mod polymarket;

const DEFAULT_OPENING_PROB: f32 = 0.5;
const SECS_PER_DAY: f32 = (60 * 60 * 24) as f32;

/// All possible platforms that are supported by this application.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize)]
pub enum Platform {
    Kalshi,
    Manifold,
    Metaculus,
    Polymarket,
}

/// All possible methods to output markets.
#[derive(Debug, Copy, Clone, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputMethod {
    Database,
    Stdout,
    Null,
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
        open_dt -> Timestamptz,
        close_dt -> Timestamptz,
        open_days -> Float,
        volume_usd -> Float,
        num_traders -> Integer,
        category -> Varchar,
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
    open_dt: DateTime<Utc>,
    close_dt: DateTime<Utc>,
    open_days: f32,
    volume_usd: f32,
    num_traders: i32,
    category: String,
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

    /// Get which category the market is in.
    fn category(&self) -> String;

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
                    "General: Requested probability at {:?} before market open at {:?}.",
                    time,
                    self.open_dt()?
                ),
                level: 3,
            });
        }
        let mut prev_prob = DEFAULT_OPENING_PROB;
        for event in self.events() {
            if event.prob < 0.0 || 1.0 < event.prob {
                // prob is out of bounds, throw error
                return Err(MarketConvertError {
                    data: self.debug(),
                    message: format!(
                        "General: Event probability {} is out of bounds.",
                        event.prob
                    ),
                    level: 3,
                });
            }
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
        if self.close_dt()? < self.open_dt()? {
            // close time is before market starts, throw error
            return Err(MarketConvertError {
                data: self.debug(),
                message: format!(
                    "General: Close time of {:?} is before market open at {:?}.",
                    self.close_dt()?,
                    self.open_dt()?
                ),
                level: 1,
            });
        }
        // calculate duration from start
        let duration_from_start = (self.close_dt()? - self.open_dt()?).num_seconds() as f32 * pct;
        let time = self.open_dt()? + Duration::seconds(duration_from_start as i64);
        self.prob_at_time(time)
    }

    /// Get the market's probability at a specific percent of the way though the duration of a market.
    fn prob_time_weighted(&self) -> Result<f32, MarketConvertError> {
        let mut prev_event: Option<ProbUpdate> = None;
        let mut cumulative_prob: f32 = 0.0;
        let mut cumulative_time: f32 = 0.0;
        for event in self.events() {
            // make sure we haven't passed outside the market open window
            if self.close_dt()? < event.time {
                break;
            }
            // check if this is the first event
            match &prev_event {
                None => {
                    if event.time > self.open_dt()? {
                        // add time between start time and first event
                        let duration = (event.time - self.open_dt()?).num_seconds() as f32;
                        cumulative_prob += DEFAULT_OPENING_PROB * duration;
                        cumulative_time += duration;
                    } else {
                        return Err(MarketConvertError {
                            data: self.debug(),
                            message: format!(
                                "General: Market event {:?} occured before market start {:?}.",
                                event,
                                self.open_dt()?
                            ),
                            level: 1,
                        });
                    }
                }
                Some(prev) => {
                    if event.time > prev.time {
                        // add time between last event and this one
                        let duration = (event.time - prev.time).num_seconds() as f32;
                        cumulative_prob += prev.prob * duration;
                        cumulative_time += duration;
                    } else if event.time == prev.time {
                        // this event happened at the exact same moment as the last, let's assume the probs are equal and move on
                        continue;
                    } else {
                        return Err(MarketConvertError {
                            data: self.debug(),
                            message: format!(
                                "General: Market events were not sorted properly, event {:?} occured before earlier event {:?}.",
                                event, prev
                            ),
                            level: 4,
                        });
                    }
                }
            }
            // save the previous event
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
                if self.events().len() > 0 {
                    // there are some events but they're all outside the market window
                    return Err(MarketConvertError {
                        data: self.debug(),
                        message: format!(
                            "General: Market had {} events but none fell within open duration.",
                            self.events().len()
                        ),
                        level: 1,
                    });
                } else {
                    // there are no events whatsoever, just assume it was the default throughout
                    let duration = (self.close_dt()? - self.open_dt()?).num_seconds() as f32;
                    cumulative_prob = DEFAULT_OPENING_PROB * duration;
                    cumulative_time = duration;
                }
            }
        }
        let prob_time_weighted = cumulative_prob / cumulative_time;
        if 0.0 <= prob_time_weighted && prob_time_weighted <= 1.0 {
            Ok(prob_time_weighted)
        } else {
            if prob_time_weighted.is_nan() {
                Err(MarketConvertError {
                    data: self.debug(),
                    message: format!(
                        "General: prob_time_weighted is NaN: {cumulative_prob} / {cumulative_time}."
                    ),
                    level: 1,
                })
            } else {
                Err(MarketConvertError {
                    data: self.debug(),
                    message: format!(
                        "General: prob_time_weighted calculation result was out of bounds: {cumulative_prob} / {cumulative_time} = {prob_time_weighted}."
                    ),
                    level: 2,
                })
            }
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
                        open_dt.eq(excluded(open_dt)),
                        close_dt.eq(excluded(close_dt)),
                        open_days.eq(excluded(open_days)),
                        volume_usd.eq(excluded(volume_usd)),
                        num_traders.eq(excluded(num_traders)),
                        category.eq(excluded(category)),
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
        OutputMethod::Null => (),
    }
}

/// Basic error type that returns the market as a debug string and a simple error message.
#[derive(Debug, Clone)]
pub struct MarketConvertError {
    data: String,
    message: String,
    level: u8,
}
impl fmt::Display for MarketConvertError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.message, self.data)
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

/// Send the request, check for common errors, and parse the response.
async fn send_request<T: for<'de> serde::Deserialize<'de>>(
    req: reqwest_middleware::RequestBuilder,
) -> Result<T, MarketConvertError> {
    let response = match req.send().await {
        Ok(r) => Ok(r),
        Err(e) => Err(MarketConvertError {
            data: e.to_string(),
            message: format!("Failed to execute HTTP call."),
            level: 5,
        }),
    }?;

    let status = response.status();
    let response_text = response.text().await.unwrap();

    if !status.is_success() {
        return Err(MarketConvertError {
            data: response_text.to_owned(),
            message: format!("Query returned status code {status}."),
            level: 4,
        });
    }

    serde_json::from_str(&response_text).map_err(|e| MarketConvertError {
        data: response_text.to_owned(),
        message: format!("Failed to deserialize: {e}."),
        level: 4,
    })
}

/// Convert timestamp (milliseconds) to datetime and error on failure.
fn get_datetime_from_millis(ts: i64) -> Result<DateTime<Utc>, ()> {
    let dt = NaiveDateTime::from_timestamp_millis(ts);
    match dt {
        Some(dt) => Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc)),
        None => Err(()),
    }
}

/// Convert timestamp (seconds) to datetime and error on failure.
fn get_datetime_from_secs(ts: i64) -> Result<DateTime<Utc>, ()> {
    let dt = NaiveDateTime::from_timestamp_opt(ts, 0);
    match dt {
        Some(dt) => Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc)),
        None => Err(()),
    }
}

/// Evaluate processing errors based on their level.
/// Level 0 is for expected events like market validity
/// Level 1 is for things that probably shouldn't happen but are uncommon
/// Level 2 is for events that should be brought to the user's attention
/// Level 3 is for actual processing errors which can be ignored
/// Level 4+ is for actual processing errors which should not be ignored
fn eval_error(error: MarketConvertError, verbose: bool) {
    let error_level = match verbose {
        false => error.level,
        true => error.level + 1,
    };
    match error_level {
        0 => (),
        1 => (),
        2 => eprintln!("{}", error),
        3 => eprintln!("{}", error),
        _ => panic!("{}", error),
    }
}
