//! Themis API server

#[macro_use]
extern crate rocket;

use anyhow::{Context, Result, anyhow};
use build_time::build_time_utc;
use diesel::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use dotenvy::dotenv;
use error::ApiError;
use rocket::State;
use rocket::fairing::AdHoc;
use rocket::serde::json::Json;
use std::env;

mod error;
use error::ResultExt;

use themis_common::XRayAnalysis;

/// Database connection pool type
type DbPool = Pool<ConnectionManager<PgConnection>>;

/// Database connection from the pool
type DbConn = PooledConnection<ConnectionManager<PgConnection>>;

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
fn xray_db_test(pool: &State<DbPool>) -> Result<Json<serde_json::Value>, ApiError> {
    // Get a database connection
    let mut conn: DbConn = pool.get().context("Failed to get database connection")?;

    // Define a struct to receive the query result
    #[derive(QueryableByName, Debug)]
    struct TableCount {
        #[diesel(sql_type = diesel::sql_types::Text)]
        table_name: String,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        row_count: i64,
    }

    // Query to get all tables and their row counts
    let query = r#"
        SELECT
            tablename as table_name,
            (xpath('/row/cnt/text()', xml_count))[1]::text::bigint as row_count
        FROM (
            SELECT
                tablename,
                query_to_xml(format('SELECT COUNT(*) as cnt FROM %I.%I', schemaname, tablename), false, true, '') as xml_count
            FROM pg_tables
            WHERE schemaname = 'public'
        ) t
        ORDER BY table_name
    "#;

    let results: Vec<TableCount> = diesel::sql_query(query)
        .load(&mut conn)
        .context("Failed to execute query")?;

    // Convert results to JSON object
    let mut table_counts = serde_json::Map::new();
    for result in results {
        table_counts.insert(result.table_name, serde_json::json!(result.row_count));
    }

    Ok(Json(serde_json::Value::Object(table_counts)))
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

/// Initializes the database connection pool from environment variables
fn init_db_pool() -> Result<DbPool> {
    // Load .env file
    dotenv().ok();

    // Build database URL from environment variables
    let db_host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
    let db_port = env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
    let db_name = env::var("POSTGRES_DB").unwrap_or_else(|_| "themis".to_string());
    let db_user = env::var("POSTGRES_USER").unwrap_or_else(|_| "themis".to_string());
    let db_password = env::var("POSTGRES_PASSWORD")
        .context("POSTGRES_PASSWORD must be set in .env or environment")?;

    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        db_user, db_password, db_host, db_port, db_name
    );

    info!(
        "Connecting to database at {}:{}/{}",
        db_host, db_port, db_name
    );

    info!("Initializing database connection pool");

    // Create connection manager and pool
    let manager = ConnectionManager::<PgConnection>::new(database_url);

    // Configure pool with settings from environment or defaults
    let pool_size = env::var("PGRST_DB_POOL")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(10);

    Pool::builder()
        .max_size(pool_size)
        .build(manager)
        .context("Failed to create database connection pool")
}

/// Launches the Rocket web server with configured routes and database pool
#[launch]
fn rocket() -> _ {
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    info!("Starting Themis API server...");

    // Initialize database pool
    let pool = init_db_pool().expect("Failed to initialize database pool");

    // Build and configure Rocket
    rocket::build()
        .manage(pool)
        .mount(
            "/",
            routes![index, xray_sample, xray_db_test, xray_error_test],
        )
        .attach(AdHoc::on_liftoff("Startup Message", |_| {
            Box::pin(async {
                info!("Rocket has launched! API is ready to accept requests.");
            })
        }))
}
