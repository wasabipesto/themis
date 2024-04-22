use actix_web::web::{Data, Query};
use actix_web::{get, middleware, App, HttpResponse, HttpServer};
use chrono::{DateTime, Utc};
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::{pg::PgConnection, prelude::*};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_yaml;
use std::collections::{HashMap, HashSet};
use std::env::var;
use std::fs::File;

mod db_util;
mod group_comparison;
mod helper;
mod market_accuracy;
mod market_calibration;
mod market_filter;
mod market_list;

use db_util::{
    get_all_platforms, get_market_by_platform_id, get_platform_by_name, market, platform, Market,
    Platform,
};
use group_comparison::build_group_comparison;
use helper::{categorize_markets_by_platform, get_scale_params, scale_data_point, ApiError};
use market_accuracy::{build_accuracy_plot, AccuracyQueryParams};
use market_calibration::{build_calibration_plot, CalibrationQueryParams};
use market_filter::{get_markets_filtered, CommonFilterParams, PageSortParams};
use market_list::{build_market_list, MarketListQueryParams};

#[derive(Debug, Serialize)]
struct IndexResponse {
    status: String,
    routes: Vec<String>,
}

#[get("/")]
async fn list_routes() -> Result<HttpResponse, ApiError> {
    let response = IndexResponse {
        status: "OK".to_string(),
        routes: Vec::from([
            "/".to_string(),
            "/list_platforms".to_string(),
            "/list_markets".to_string(),
            "/calibration_plot".to_string(),
            "/accuracy_plot".to_string(),
            "/group_accuracy".to_string(),
        ]),
    };
    Ok(HttpResponse::Ok().json(response))
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

#[get("/group_accuracy")]
async fn group_accuracy(
    //query: Query<AccuracyQueryParams>,
    pool: Data<Pool<ConnectionManager<PgConnection>>>,
) -> Result<HttpResponse, ApiError> {
    // get database connection from pool
    let conn = &mut pool
        .get()
        .map_err(|e| ApiError::new(500, format!("failed to get connection from pool: {e}")))?;

    // build the plot
    build_group_comparison(conn)
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
            .service(list_routes)
            .service(list_platforms)
            .service(list_markets)
            .service(calibration_plot)
            .service(accuracy_plot)
            .service(group_accuracy)
    })
    .bind(var("HTTP_BIND").unwrap_or(String::from("0.0.0.0:7041")))?
    .run()
    .await
}
