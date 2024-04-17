//! This module has all of the common utilities and market standardization tools required to query the API and convert responses into DB rows.

use chrono::serde::{ts_milliseconds, ts_milliseconds_option, ts_seconds};
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
use std::cmp::Ordering;
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
        prob_at_pct -> Array<Float>,
        prob_time_avg -> Float,
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
    prob_at_pct: Vec<f32>,
    prob_time_avg: f32,
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

    /// Get a list of the market probabilities from 0% to 100% of the market duration.
    fn prob_at_pct_list(&self) -> Result<Vec<f32>, MarketConvertError> {
        (0..=100)
            .map(|pct| self.prob_at_percent(pct as f32 / 100.0))
            .collect()
    }

    /// Get the market's time-averaged probability between two timestamps.
    /// This is calculated by taking the average of all market probabilities
    /// weighted by how long the market was at that probability.
    /// We trust that events are ordered properly before this stage and throw
    /// errors if they were not.
    fn prob_time_avg_between(
        &self,
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
    ) -> Result<f32, MarketConvertError> {
        let all_events = self.events();

        // get the probability at the start of the window
        let last_event_before_window = all_events
            .iter()
            .filter(|event| event.time <= window_start)
            .last();
        let prob_at_window_start = match last_event_before_window {
            Some(event) => event.prob,
            None => DEFAULT_OPENING_PROB,
        };
        let mut prev_event = &ProbUpdate {
            time: window_start,
            prob: prob_at_window_start,
        };

        let events_in_window: Vec<&ProbUpdate> = all_events
            .iter()
            .filter(|event| event.time > window_start && event.time < window_end)
            .collect();

        // set up lookback and counters
        let mut cumulative_prob: f32 = 0.0;
        let mut cumulative_time: f32 = 0.0;
        for event in events_in_window {
            // skip any events that don't change the probability
            if event.prob == prev_event.prob {
                continue;
            }
            // compare timstamp against previous event to catch some potential ordering errors
            match event.time.cmp(&prev_event.time) {
                Ordering::Greater => {
                    // add time between last event and this one
                    let duration = (event.time - prev_event.time).num_seconds() as f32;
                    cumulative_prob += prev_event.prob * duration;
                    cumulative_time += duration;
                }
                Ordering::Equal => {
                    return Err(MarketConvertError {
                        data: self.debug(),
                        message: format!(
                            "General: Market events {:?} and {:?} occured simultaneously but with different probabilities.",
                            event, prev_event
                        ),
                        level: 4,
                    });
                }
                Ordering::Less => {
                    return Err(MarketConvertError {
                        data: self.debug(),
                        message: format!(
                            "General: Market events were not sorted properly, event {:?} timestamp should be greater than event {:?}.",
                            event, prev_event
                        ),
                        level: 4,
                    });
                }
            }
            // save the event for comparison
            prev_event = event
        }

        // add the duration between the last event and window end
        // if there are no events in the window this starts at the window start
        {
            let duration = (window_end - prev_event.time).num_seconds() as f32;
            cumulative_prob += prev_event.prob * duration;
            cumulative_time += duration;
        }

        let prob_time_avg = cumulative_prob / cumulative_time;
        if (0.0..=1.0).contains(&prob_time_avg) {
            Ok(prob_time_avg)
        } else if prob_time_avg.is_nan() {
            Err(MarketConvertError {
                data: self.debug(),
                message: format!(
                    "General: prob_time_avg is NaN (probably because duration was too short): {cumulative_prob} / {cumulative_time}."
                ),
                level: 3,
            })
        } else {
            Err(MarketConvertError {
                data: self.debug(),
                message: format!(
                    "General: prob_time_avg calculation result was out of bounds: {cumulative_prob} / {cumulative_time} = {prob_time_avg}."
                ),
                level: 3,
            })
        }
    }

    /// Get the market's time-averaged probability over the course of the market.
    fn prob_time_avg_whole(&self) -> Result<f32, MarketConvertError> {
        self.prob_time_avg_between(self.open_dt()?, self.close_dt()?)
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
                        url.eq(excluded(url)),
                        open_dt.eq(excluded(open_dt)),
                        close_dt.eq(excluded(close_dt)),
                        open_days.eq(excluded(open_days)),
                        volume_usd.eq(excluded(volume_usd)),
                        num_traders.eq(excluded(num_traders)),
                        category.eq(excluded(category)),
                        prob_at_midpoint.eq(excluded(prob_at_midpoint)),
                        prob_at_close.eq(excluded(prob_at_close)),
                        prob_time_avg.eq(excluded(prob_time_avg)),
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
/// If no period is supplied, the rate limit is per second.
fn get_reqwest_client_ratelimited(
    request_count: usize,
    interval_ms: Option<u64>,
) -> ClientWithMiddleware {
    // get requested period or default
    let interval_duration = std::time::Duration::from_millis(interval_ms.unwrap_or(1000));
    // retry requests that get server errors with an exponential backoff timer
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    // rate limit to n requests per second
    let rate_limiter = RateLimiter::builder()
        .interval(interval_duration)
        .refill(request_count)
        .max(request_count)
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
            message: "Failed to execute HTTP call.".to_string(),
            level: 5,
        }),
    }?;

    let status = response.status();
    let response_text = response.text().await.map_err(|e| MarketConvertError {
        data: e.to_string(),
        message: "Failed to get response body text.".to_string(),
        level: 4,
    })?;

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

/// Print a standard log line with the current datetime.
fn log_to_stdout(message: &str) {
    println!("{:?} - {}", chrono::offset::Local::now(), message);
}
