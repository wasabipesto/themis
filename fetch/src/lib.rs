use diesel::{pg::PgConnection, prelude::*, Connection};
use serde_json::to_string_pretty;
use std::env::var;
use std::time::Instant;

pub mod platforms;
use platforms::{market, MarketStandard, Platform};

/// Options for how to output the markets
#[derive(Debug, Copy, Clone, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputMethod {
    Database,
    Stdout,
    //File,
}

/// The main path for processing markets by platform
#[tokio::main]
pub async fn run(
    platform: Option<Platform>,
    id: Option<String>,
    output: OutputMethod,
    verbose: bool,
) {
    // if the user requested a specific platform, format it into a list
    // otherwise, return the default platform list
    let platforms: Vec<&Platform> = match &platform {
        Some(platform) => Vec::from([platform]),
        None => Vec::from([&Platform::Kalshi, &Platform::Manifold]),
    };

    if verbose {
        println!("Initialization: Processing platforms: {:?}", &platforms);
    }
    let total_timer = Instant::now();
    let mut markets: Vec<MarketStandard> = Vec::new();
    for platform in platforms {
        let timer = Instant::now();
        println!("{:?}: Processing started...", &platform);
        let platform_markets = match (&platform, &id) {
            (Platform::Kalshi, None) => platforms::kalshi::get_markets_all().await,
            (Platform::Kalshi, Some(id)) => platforms::kalshi::get_market_by_id(id).await,
            (Platform::Manifold, None) => platforms::manifold::get_markets_all().await,
            (Platform::Manifold, Some(id)) => platforms::manifold::get_market_by_id(id).await,
        };
        println!(
            "{:?}: {} markets processed in {:?}.",
            &platform,
            platform_markets.len(),
            timer.elapsed()
        );
        markets.extend(platform_markets);
    }
    println!(
        "All processing complete: {} markets processed in {:?}.",
        markets.len(),
        total_timer.elapsed()
    );

    // save collated data to database, stdout, or file
    match output {
        OutputMethod::Database => {
            println!("Database: Saving all markets to db...");
            let mut conn = PgConnection::establish(
                &var("DATABASE_URL").expect("Required environment variable DATABASE_URL not set."),
            )
            .expect("Error connecting to datbase.");
            for chunk in markets.chunks(1000) {
                diesel::insert_into(market::table)
                    .values(chunk)
                    .on_conflict_do_nothing() // TODO: upsert
                    .execute(&mut conn)
                    .expect("Failed to insert rows into table.");
            }
            println!("Database: All markets saved.");
        }
        OutputMethod::Stdout => {
            println!("{}", to_string_pretty(&markets).unwrap())
        }
    }
}
