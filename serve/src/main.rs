use actix_web::{get, middleware, web::Data, App, HttpResponse, HttpServer, Responder};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{pg::PgConnection, prelude::*};
use serde::Serialize;
use std::collections::HashMap;
use std::env::var;

mod helper;

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

#[derive(Debug, Serialize)]
struct Plot {
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
async fn calibration_plot(pool: Data<Pool<ConnectionManager<PgConnection>>>) -> impl Responder {
    // get weight from query (and more eventually)
    let weight_attribute = "market";

    // get database connection from pool
    let conn = &mut pool.get().expect("Failed to get connection from pool");
    // get all markets from database
    let markets = market::table
        //.filter(market::open_days.ge(0.0))
        //.filter(market::volume_usd.ge(0.0))
        .select(Market::as_select())
        .load::<Market>(conn)
        .expect("Failed to query table.");

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
    let bin_distance: i32 = prob_to_k(&0.10);
    let bin_look = bin_distance / 2;
    let mut bins: Vec<i32> = Vec::new();
    let mut x = 0;
    while x <= prob_to_k(&1.0) {
        bins.push(x);
        x += bin_distance;
    }

    let mut response = Vec::new();
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
            let market_k = prob_to_k(&market.prob_time_weighted);
            let bin = bins
                .iter()
                .find(|&x| x - bin_look <= market_k && market_k <= x + bin_look)
                .expect(&format!(
                    "Failed to find correct bin for {market_k} in {bins:?} with lookaround {bin_look}"
                ));
            //println!("{platform} market sorted into bin {}", k_to_prob(bin));

            // get the weighting value
            let weight: f32 = match weight_attribute {
                "market" => 1.0,
                _ => panic!("weight_attribute invalid"),
            };

            // add the market data to each counter
            *weighted_sums.get_mut(&bin).unwrap() += weight * market.resolution;
            *weighted_counts.get_mut(&bin).unwrap() += weight;
        }

        println!("{platform} sums {:?}", weighted_sums);
        println!("{platform} counts {:?}", weighted_counts);

        // divide out rolling averages into a single average value
        let x_series = bins.iter().map(|x| k_to_prob(x)).collect();
        let y_series = bins
            .iter()
            .map(|x| {
                // note that NaN is serialized as None, so if `count` is 0 the point won't be shown
                let sum = weighted_sums.get(x).unwrap();
                let count = weighted_counts.get(x).unwrap();
                sum / count
            })
            .collect();

        response.push(Plot {
            platform,
            x_series,
            y_series,
        })
    }
    println!("{:?}", response);

    HttpResponse::Ok().json(response)
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
            .wrap(middleware::Logger::default())
            .service(calibration_plot)
    })
    .bind(var("HTTP_BIND").unwrap_or(String::from("0.0.0.0:7041")))?
    .run()
    .await
}
