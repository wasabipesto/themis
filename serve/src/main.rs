use actix_web::web::{Data, Query};
use actix_web::{get, middleware, App, HttpResponse, HttpServer};
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::{pg::PgConnection, prelude::*};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env::var;

mod helper;
use helper::ApiError;

const DEFAULT_BIN_METHOD: &str = "prob_time_weighted";
const DEFAULT_WEIGHT_ATTR: &str = "none";

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
        num_traders -> Integer,
        prob_at_midpoint -> Float,
        prob_at_close -> Float,
        prob_time_weighted -> Float,
        resolution -> Float,
    }
}
table! {
    platform (platform_name) {
        platform_name -> Varchar,
        platform_name_fmt -> Varchar,
        platform_description -> Varchar,
        platform_avatar_url -> Varchar,
        platform_color -> Varchar,
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
    num_traders: i32,
    prob_at_midpoint: f32,
    prob_at_close: f32,
    prob_time_weighted: f32,
    resolution: f32,
}

/// Data about a platform cached in the database.
#[derive(Debug, Queryable, Serialize, Selectable)]
#[diesel(table_name = platform)]
struct Platform {
    platform_name: String,
    platform_name_fmt: String,
    platform_description: String,
    platform_avatar_url: String,
    platform_color: String,
}

/// Parameters passed to the calibration function.
#[derive(Debug, Deserialize)]
pub struct QueryParams {
    bin_method: Option<String>,
    bin_size: Option<f32>,
    weight_attribute: Option<String>,
    min_open_days: Option<f32>,
    min_num_traders: Option<i32>,
    min_volume_usd: Option<f32>,
    title_contains: Option<String>,
    categories: Option<String>,
}

/// Metadata to help label a plot.
#[derive(Debug, Serialize)]
struct Metadata {
    title: String,
    x_title: String,
    y_title: String,
}
/// Data sent to the client to render a plot, one plot per platform.
#[derive(Debug, Serialize)]
struct Trace {
    platform_name_fmt: String,
    platform_description: String,
    platform_avatar_url: String,
    platform_color: String,
    num_markets: usize,
    brier_score: f32,
    x_series: Vec<f32>,
    y_series: Vec<f32>,
    point_sizes: Vec<f32>,
    //point_descriptions: Vec<String>,
}

/// Full response for a calibration plot.
#[derive(Debug, Serialize)]
struct CalibrationPlot {
    metadata: Metadata,
    traces: Vec<Trace>,
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

/// Get information about a platform from the database to send to the client.
fn get_platform_info(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    platform_req: &String,
) -> Result<Platform, ApiError> {
    use crate::platform::dsl::*;
    let query = platform.find(&platform_req).first(conn);
    match query {
        Ok(p) => Ok(p),
        Err(_) => Err(ApiError::new(
            500,
            format!("could not find platform data for {platform_req}"),
        )),
    }
}

/// Scale a list of weights down to valid point sizes.
fn scale_list(list: Vec<f32>, output_min: f32, output_max: f32, output_default: f32) -> Vec<f32> {
    if list.is_empty() {
        return Vec::new();
    }

    let input_min = *list
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let input_max = *list
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    if input_min == input_max {
        return vec![output_default; list.len()];
    }

    list.iter()
        .map(|&value| {
            ((value - input_min) / (input_max - input_min)) * (output_max - output_min) + output_min
        })
        .collect()
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
        if bs < 0.0 || bs > 0.5 {
            return Err(ApiError::new(
                400,
                format!("`bin_size` should be between 0.0 and 0.5"),
            ));
        }
    }

    let weight_attribute = query
        .weight_attribute
        .clone()
        .unwrap_or(DEFAULT_WEIGHT_ATTR.to_string());

    let min_open_days = query.min_open_days.clone().unwrap_or(0.0);
    let min_volume_usd = query.min_volume_usd.clone().unwrap_or(0.0);
    let min_num_traders = query.min_num_traders.clone().unwrap_or(0);
    let title_contains = "%".to_string() + &query.title_contains.clone().unwrap_or_default() + "%";

    // get database connection from pool
    let conn = &mut pool.get().or_else(|e| {
        Err(ApiError::new(
            500,
            format!("failed to get connection from pool: {e}"),
        ))
    })?;

    // get all markets from database
    let markets = market::table
        .filter(market::open_days.ge(min_open_days))
        .filter(market::volume_usd.ge(min_volume_usd))
        .filter(market::num_traders.ge(min_num_traders))
        .filter(market::title.ilike(title_contains))
        .select(Market::as_select())
        .load::<Market>(conn)
        .or_else(|e| Err(ApiError::new(500, format!("failed to query markets: {e}"))))?;

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
        let mut weighted_resolution_sums = HashMap::with_capacity(bins.len());
        let mut weighted_counts = HashMap::with_capacity(bins.len());
        // populate each map with x-values from bins
        for bin in bins.clone() {
            let hash_value = bin;
            weighted_resolution_sums.insert(hash_value, 0.0);
            weighted_counts.insert(hash_value, 0.0);
        }

        // set up brier counters
        let mut weighted_brier_sum: f32 = 0.0;
        let mut weighted_brier_count: f32 = 0.0;

        // get weighted average values for all markets
        // this is a hot loop since we iterate over all markets
        for market in market_list.iter() {
            // find the closest bin based on the market's selected x value
            let market_x_value = match bin_method.as_str() {
                "prob_at_midpoint" => market.prob_at_midpoint,
                "prob_at_close" => market.prob_at_close,
                "prob_time_weighted" => market.prob_time_weighted,
                _ => {
                    return Err(ApiError::new(
                        400,
                        "the value provided for `bin_method` is not a valid option".to_string(),
                    ))
                }
            };
            if market_x_value.is_nan() {
                return Err(ApiError {
                    status_code: 500,
                    message: format!("Market X-Value is NaN: {:?}", market),
                });
            };
            let market_k = prob_to_k(&market_x_value);
            let bin = bins
                .iter()
                .find(|&x| x - bin_look <= market_k && market_k <= x + bin_look)
                .ok_or_else(|| {
                    ApiError::new(
                        500,
                        format!(
                            "failed to find correct bin for {} in {:?} with lookaround {}",
                            market_k, bins, bin_look
                        ),
                    )
                })?;

            // get the weighting value
            let market_y_value = market.resolution;
            if market_y_value.is_nan() {
                return Err(ApiError {
                    status_code: 500,
                    message: format!("Market Y-Value is NaN: {:?}", market),
                });
            };
            let weight: f32 = match weight_attribute.as_str() {
                "open_days" => market.open_days,
                "num_traders" => market.num_traders as f32,
                "volume_usd" => market.volume_usd,
                "none" => 1.0,
                _ => {
                    return Err(ApiError::new(
                        400,
                        "the value provided for `weight_attribute` is not a valid option"
                            .to_string(),
                    ))
                }
            };

            // add the market data to each counter
            *weighted_resolution_sums.get_mut(&bin).unwrap() += weight * market_y_value;
            *weighted_counts.get_mut(&bin).unwrap() += weight;
            weighted_brier_sum += weight * (market_y_value - market_x_value).powf(2.0);
            weighted_brier_count += weight;
        }

        // divide out rolling averages into a single average value
        let x_series = bins.iter().map(|bin| k_to_prob(bin)).collect();
        let y_series = bins
            .iter()
            .map(|bin| {
                // note that NaN is serialized as None, so if `count` is 0 the point won't be shown
                let sum = weighted_resolution_sums.get(bin).unwrap();
                let count = weighted_counts.get(bin).unwrap();
                sum / count
            })
            .collect();
        let point_weights = bins
            .iter()
            .map(|bin| *weighted_counts.get(bin).unwrap())
            .collect();
        let point_sizes = scale_list(point_weights, 8.0, 32.0, 10.0);

        // get cached platform info from database
        let platform_info = get_platform_info(conn, &platform)?;

        // save it all to the trace and push it to result
        traces.push(Trace {
            platform_name_fmt: platform_info.platform_name_fmt,
            platform_description: platform_info.platform_description,
            platform_avatar_url: platform_info.platform_avatar_url,
            platform_color: platform_info.platform_color,
            num_markets: market_list.len(),
            brier_score: weighted_brier_sum / weighted_brier_count,
            x_series,
            y_series,
            point_sizes,
        })
    }

    traces.sort_unstable_by_key(|t| t.platform_name_fmt.clone());

    let metadata = Metadata {
        title: format!("Calibration Plot"),
        x_title: match bin_method.as_str() {
            "prob_at_midpoint" => format!("Probability at Midpoint"),
            "prob_at_close" => format!("Probability at Close"),
            "prob_time_weighted" => format!("Time-Weighted Probability"),
            _ => panic!("given bin_method not in x_title map"),
        },
        y_title: match weight_attribute.as_str() {
            "open_days" => format!("Resolution, Weighted by Duration"),
            "num_traders" => format!("Resolution, Weighted by Traders"),
            "volume_usd" => format!("Resolution, Weighted by Volume"),
            "none" => format!("Resolution, Unweighted"),
            _ => panic!("given weight_attribute not in y_title map"),
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
