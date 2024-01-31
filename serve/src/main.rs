use actix_web::web::{Data, Query};
use actix_web::{get, middleware, App, HttpResponse, HttpServer};
use chrono::{DateTime, Utc};
use db_util::{get_all_platforms, get_platform_by_name, market, platform, Market};
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::{pg::PgConnection, prelude::*};
use helper::{scale_list, ApiError};
use market_calibration::{build_calibration_plot, CalibrationQueryParams};
use market_filter::{get_markets_filtered, CommonFilterParams};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env::var;

mod db_util;
mod helper;
mod market_calibration;
mod market_filter;

/// Metadata to help label a plot.
#[derive(Debug, Serialize)]
struct PlotMetadata {
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
struct PlotData {
    metadata: PlotMetadata,
    traces: Vec<Trace>,
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
    query: Query<CommonFilterParams>,
    pool: Data<Pool<ConnectionManager<PgConnection>>>,
) -> Result<HttpResponse, ApiError> {
    // get database connection from pool
    let conn = &mut pool
        .get()
        .map_err(|e| ApiError::new(500, format!("failed to get connection from pool: {e}")))?;

    // get markets from database
    let markets = get_markets_filtered(conn, query.into_inner())?;

    // send to client
    Ok(HttpResponse::Ok().json(markets))
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

    // get markets from database
    let markets = get_markets_filtered(conn, query.filters.clone())?;

    // build the plot
    build_calibration_plot(query, conn, markets)
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
    })
    .bind(var("HTTP_BIND").unwrap_or(String::from("0.0.0.0:7041")))?
    .run()
    .await
}
