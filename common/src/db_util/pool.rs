//! Themis database utilities

use anyhow::{Context, Result};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use dotenvy::dotenv;
use log::info;
use std::env;

/// Database connection pool type
pub type DbPool = Pool<ConnectionManager<PgConnection>>;

/// Initializes the database connection pool from environment variables
pub fn init_db_pool() -> Result<DbPool> {
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
    let pool_size = env::var("POSTGRES_POOL_SIZE")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(10);

    Pool::builder()
        .max_size(pool_size)
        .build(manager)
        .context("Failed to create database connection pool")
}
