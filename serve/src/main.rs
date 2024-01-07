use actix_web::web::{Data, Query};
use actix_web::{get, middleware, App, HttpResponse, HttpServer};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{pg::PgConnection, prelude::*};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env::var;

mod helper;
use helper::ApiError;

const DEFAULT_BIN_METHOD: &str = "prob_time_weighted";
const DEFAULT_WEIGHT_ATTR: &str = "count";

// Diesel macro to get markets from the database table.
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
        prob_time_weighted -> Float,
        resolution -> Float,
    }
}

/// Data returned from the database, same as what we inserted.
#[derive(Debug, Queryable, Serialize, Selectable)]
#[diesel(table_name = market)]
struct Market {
    title: String,
    platform: String,
    platform_id: String,
    url: String,
    open_days: f32,
    volume_usd: f32,
    prob_at_midpoint: f32,
    prob_at_close: f32,
    prob_time_weighted: f32,
    resolution: f32,
}

/// Parameters passed to the calibration function.
#[derive(Debug, Deserialize)]
pub struct QueryParams {
    bin_method: Option<String>,
    bin_size: Option<f32>,
    weight_attribute: Option<String>,
    min_open_days: Option<f32>,
    min_volume_usd: Option<f32>,
}

/// Metadata to help label a plot.
#[derive(Debug, Serialize)]
struct Metadata {
    title: String,
    x_title: String,
    y_title: String,
}

/// Full response for a calibration plot.
#[derive(Debug, Serialize)]
struct CalibrationPlot {
    metadata: Metadata,
    traces: Vec<Trace>,
}

/// Data sent to the client to render a plot, one plot per platform.
#[derive(Debug, Serialize)]
struct Trace {
    platform: String,
    //platform_description: String,
    //platform_avatar_url: String,
    //brier_score: f32,
    x_series: Vec<f32>,
    y_series: Vec<f32>,
    //point_sizes: Vec<f32>,
    //point_descriptions: Vec<String>,
}

/// A quick and dirty f32 mask into u32 for key lookup.
/// Only needs to work for values between 0 and 1 at increaments of 0.01.
fn prob_to_k(f: &f32) -> i32 {
    (f * 1000.0) as i32
}

/// Inverse of the above function.
fn k_to_prob(k: &i32) -> f32 {
    *k as f32 / 1000.0
}

#[get("/calibration_plot")]
async fn calibration_plot(
    query: Query<QueryParams>,
    pool: Data<Pool<ConnectionManager<PgConnection>>>,
) -> Result<HttpResponse, ApiError> {
    // get query parameters or defaults
    let bin_method = query
        .bin_method
        .clone()
        .unwrap_or(DEFAULT_BIN_METHOD.to_string());
    let bin_size = query.bin_size.clone();
    if let Some(bs) = bin_size {
        if bs < 0.0 {
            return Err(ApiError::new(
                400,
                "`bin_size` should be greater than 0".to_string(),
            ));
        }
        if bs > 0.5 {
            return Err(ApiError::new(
                400,
                "`bin_size` should be less than 0.5".to_string(),
            ));
        }
    }

    let weight_attribute = query
        .weight_attribute
        .clone()
        .unwrap_or(DEFAULT_WEIGHT_ATTR.to_string());

    let min_open_days = query.min_open_days.clone().unwrap_or(0.0);
    let min_volume_usd = query.min_volume_usd.clone().unwrap_or(0.0);

    // get database connection from pool
    let conn = &mut match pool.get() {
        Ok(conn) => conn,
        Err(e) => {
            return Err(ApiError::new(
                500,
                format!("failed to get connection from pool: {e}"),
            ));
        }
    };

    // get all markets from database
    let query = market::table
        .filter(market::open_days.ge(min_open_days))
        .filter(market::volume_usd.ge(min_volume_usd))
        .select(Market::as_select())
        .load::<Market>(conn);
    let markets = match query {
        Ok(m) => m,
        Err(e) => {
            return Err(ApiError::new(500, format!("failed to query markets: {e}")));
        }
    };

    // sort all markets based on the platform
    // this is a hot loop since we iterate over all markets
    let mut markets_by_platform: HashMap<String, Vec<Market>> = HashMap::new();
    for market in markets {
        if let Some(market_list) = markets_by_platform.get_mut(&market.platform) {
            market_list.push(market);
        } else {
            markets_by_platform.insert(market.platform.clone(), Vec::from([market]));
        }
    }

    // generate the x-value bins
    // note that we use u32 here instead of f32 since floating points are hard to use as keys
    let bin_size: i32 = prob_to_k(&bin_size.unwrap_or(0.05));
    let bin_look = bin_size / 2;
    let mut bins: Vec<i32> = Vec::new();
    let mut x = bin_look;
    while x <= prob_to_k(&1.0) {
        bins.push(x);
        x += bin_size;
    }

    let mut traces = Vec::new();
    for (platform, market_list) in markets_by_platform {
        // build sums and counts to use as rolling averages
        let mut weighted_sums = HashMap::with_capacity(bins.len());
        let mut weighted_counts = HashMap::with_capacity(bins.len());
        // populate each map with x-values from bins
        for bin in bins.clone() {
            let hash_value = bin;
            weighted_sums.insert(hash_value, 0.0);
            weighted_counts.insert(hash_value, 0.0);
        }

        // get weighted average values for all markets
        // this is a hot loop since we iterate over all markets
        for market in market_list {
            // find the closest bin based on the market's resolution value
            let market_k = prob_to_k(&match bin_method.as_str() {
                "prob_at_midpoint" => market.prob_at_midpoint,
                "prob_at_close" => market.prob_at_close,
                "prob_time_weighted" => market.prob_time_weighted,
                _ => {
                    return Err(ApiError::new(
                        400,
                        "the value provided for `bin_method` is not a valid option".to_string(),
                    ))
                }
            });
            let bin_q = bins
                .iter()
                .find(|&x| x - bin_look <= market_k && market_k <= x + bin_look);
            let bin = match bin_q {
                Some(m) => m,
                None => {
                    return Err(ApiError::new(500, format!(
                        "failed to find correct bin for {market_k} in {bins:?} with lookaround {bin_look}"
                    )));
                }
            };

            // get the weighting value
            let weight: f32 = match weight_attribute.as_str() {
                "open_days" => market.open_days,
                "volume_usd" => market.volume_usd,
                "count" => 1.0,
                _ => {
                    return Err(ApiError::new(
                        400,
                        "the value provided for `weight_attribute` is not a valid option"
                            .to_string(),
                    ))
                }
            };

            // add the market data to each counter
            *weighted_sums.get_mut(&bin).unwrap() += weight * market.resolution;
            *weighted_counts.get_mut(&bin).unwrap() += weight;
        }

        // divide out rolling averages into a single average value
        let x_series = bins.iter().map(|x| k_to_prob(x)).collect();
        let y_series = bins
            .iter()
            .map(|bin| {
                // note that NaN is serialized as None, so if `count` is 0 the point won't be shown
                let sum = weighted_sums.get(bin).unwrap();
                let count = weighted_counts.get(bin).unwrap();
                sum / count
            })
            .collect();

        traces.push(Trace {
            platform,
            x_series,
            y_series,
        })
    }

    let metadata = Metadata {
        title: format!("Calibration Plot"),
        x_title: match bin_method.as_str() {
            "prob_at_midpoint" => format!("Probability at Midpoint"),
            "prob_at_close" => format!("Probability at Close"),
            "prob_time_weighted" => format!("Time-Weighted Probability"),
            _ => panic!(""),
        },
        y_title: match weight_attribute.as_str() {
            "open_days" => format!("Resolution, Weighted by Duration"),
            "volume_usd" => format!("Resolution, Weighted by Volume"),
            "count" => format!("Resolution, Unweighted"),
            _ => panic!(""),
        },
    };

    let response = CalibrationPlot { metadata, traces };

    Ok(HttpResponse::Ok().json(response))
}

/// Server startup tasks.
#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
    // build database pool
    let database_url =
        var("DATABASE_URL").expect("Required environment variable DATABASE_URL not set.");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
        .build(manager)
        .expect("Failed to create database connection pool.");

    // set up logging
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // start the actual server
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(pool.clone()))
            .wrap(actix_cors::Cors::permissive())
            .wrap(middleware::Logger::default())
            .service(calibration_plot)
    })
    .bind(var("HTTP_BIND").unwrap_or(String::from("0.0.0.0:7041")))?
    .run()
    .await
}
