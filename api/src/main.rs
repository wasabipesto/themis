//! Themis API server

#[macro_use]
extern crate rocket;

use anyhow::{Context, Result, anyhow};
use build_time::build_time_utc;
use error::ApiError;
use log::info;
use rocket::State;
use rocket::serde::json::Json;
use std::env;

mod error;
use error::ResultExt;

use themis_common::XRayAnalysis;
use themis_common::db_util::debug::{TableDebugInfo, get_table_debug_info};
use themis_common::db_util::pool::{DbPool, init_db_pool};

// ============================================================================
// Routes
// ============================================================================

/// Health check and API metadata endpoint
#[get("/")]
fn index() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "Themis API",
        "version": env!("CARGO_PKG_VERSION"),
        "build_time": build_time_utc!(),
        "routes": [
            "/",
            "/xray/sample",
            "/xray/db_test",
            "/xray/error_test"
        ]
    }))
}

/// Returns sample X-Ray analysis data for testing
#[get("/xray/sample")]
fn xray_sample(pool: &State<DbPool>) -> Result<Json<XRayAnalysis>, ApiError> {
    // Get a database connection
    let mut _conn = pool.get().context("Failed to get database connection")?;

    Ok(Json(XRayAnalysis::default()))
}

/// Tests database connectivity and returns table row counts
#[get("/xray/db_test")]
fn xray_db_test(pool: &State<DbPool>) -> Result<Json<Vec<TableDebugInfo>>, ApiError> {
    // Get a database connection
    let mut conn = pool.get().context("Failed to get database connection")?;

    // Convert results to JSON object
    let table_counts = get_table_debug_info(&mut conn)?;

    Ok(Json(table_counts))
}

/// Demonstrates errors so I remember how to use them later.
#[get("/xray/error_test")]
fn xray_error_test() -> Result<Json<XRayAnalysis>, ApiError> {
    use rand::Rng;
    let mut rng = rand::rng();
    match rng.random_range(1..=100) {
        1..=30 => Err(anyhow!("Some funny internal error happened."))?,
        31..=60 => Err(anyhow!("I think you did something wrong.")).bad_request()?,
        61..=90 => Err(anyhow!("Sorry, couldn't find it.")).not_found()?,
        _ => Ok(Json(XRayAnalysis::default())),
    }
}

// ============================================================================
// Application Setup
// ============================================================================

/// Launches the Rocket web server with configured routes and database pool
#[launch]
fn rocket() -> _ {
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    info!("Starting Themis API server...");

    // Initialize database pool
    let pool = init_db_pool().expect("Failed to initialize database pool");

    // Build and configure Rocket
    rocket::build().manage(pool).mount(
        "/",
        routes![index, xray_sample, xray_db_test, xray_error_test],
    )
}
