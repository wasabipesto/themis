use actix_web::web::{Data, Query};
use actix_web::{get, middleware, App, HttpResponse, HttpServer};
use chrono::{DateTime, Utc};
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::{pg::PgConnection, prelude::*};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env::var;

mod db_util;
mod helper;
mod market_accuracy;
mod market_calibration;
mod market_filter;
mod market_list;

use db_util::{get_all_platforms, get_platform_by_name, market, platform, Market, Platform};
use helper::{categorize_markets_by_platform, get_scale_params, scale_data_point, ApiError};
use market_accuracy::{build_accuracy_plot, AccuracyQueryParams};
use market_calibration::{build_calibration_plot, CalibrationQueryParams};
use market_filter::{get_markets_filtered, CommonFilterParams, PageSortParams};
use market_list::{build_market_list, MarketListQueryParams};

const POINT_SIZE_MIN: f32 = 6.0;
const POINT_SIZE_MAX: f32 = 28.0;
const POINT_SIZE_DEFAULT: f32 = 8.0;

#[get("/")]
async fn index() -> String {
    "OK".to_string()
}

#[get("/list_platforms")]
async fn list_platforms(
    pool: Data<Pool<ConnectionManager<PgConnection>>>,
) -> Result<HttpResponse, ApiError> {
    // get database connection from pool
    let conn = &mut pool
        .get()
        .map_err(|e| ApiError::new(500, format!("failed to get connection from pool: {e}")))?;

    // get all platforms from database
    let platforms = get_all_platforms(conn)?;

    // send to client
    Ok(HttpResponse::Ok().json(platforms))
}

#[get("/list_markets")]
async fn list_markets(
    query: Query<MarketListQueryParams>,
    pool: Data<Pool<ConnectionManager<PgConnection>>>,
) -> Result<HttpResponse, ApiError> {
    // get database connection from pool
    let conn = &mut pool
        .get()
        .map_err(|e| ApiError::new(500, format!("failed to get connection from pool: {e}")))?;

    // send to client
    build_market_list(query, conn)
}

#[get("/calibration_plot")]
async fn calibration_plot(
    query: Query<CalibrationQueryParams>,
    pool: Data<Pool<ConnectionManager<PgConnection>>>,
) -> Result<HttpResponse, ApiError> {
    // get database connection from pool
    let conn = &mut pool
        .get()
        .map_err(|e| ApiError::new(500, format!("failed to get connection from pool: {e}")))?;

    // build the plot
    build_calibration_plot(query, conn)
}

#[get("/accuracy_plot")]
async fn accuracy_plot(
    query: Query<AccuracyQueryParams>,
    pool: Data<Pool<ConnectionManager<PgConnection>>>,
) -> Result<HttpResponse, ApiError> {
    // get database connection from pool
    let conn = &mut pool
        .get()
        .map_err(|e| ApiError::new(500, format!("failed to get connection from pool: {e}")))?;

    // build the plot
    build_accuracy_plot(query, conn)
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
            .service(list_platforms)
            .service(list_markets)
            .service(calibration_plot)
            .service(accuracy_plot)
    })
    .bind(var("HTTP_BIND").unwrap_or(String::from("0.0.0.0:7041")))?
    .run()
    .await
}
