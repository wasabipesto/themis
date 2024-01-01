use clap::Parser;
use diesel::{pg::PgConnection, prelude::*, Connection};
use serde_json::{to_string_pretty, to_writer_pretty};
use std::env::var;
use std::fs::File;
use std::time::Instant;

mod platforms;
use crate::platforms::{market, MarketForDB, Platform};

const OUTPUT_KEYWORD_DB: &str = "db";
const OUTPUT_KEYWORD_STDOUT: &str = "stdout";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Override the default platform list to only pull from one provider
    #[arg(short, long)]
    platform: Option<Platform>,

    /// Only pull market data for a single market - requires a single platform to be specified
    #[arg(long)]
    id: Option<String>,

    /// Redirect output to the database ["db"], the console ["stdout"], or a file [value used as filename]
    #[arg(short, long, default_value = OUTPUT_KEYWORD_DB)]
    output: String,

    /// Show additional output for debugging
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();
    // print the user inputs for debug purposes
    if args.verbose {
        println!("Initialization: {:?}", &args);
    }

    // if the user requested a specific platform, format it into a list
    // otherwise, return the default platform list
    let platforms: Vec<&Platform> = match &args.platform {
        Some(platform) => Vec::from([platform]),
        None => Vec::from([&Platform::Kalshi, &Platform::Manifold]),
    };

    if args.verbose {
        println!("Initialization: Processing platforms: {:?}", &platforms);
    }
    let total_timer = Instant::now();
    let mut markets: Vec<MarketForDB> = Vec::new();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        for platform in platforms.clone() {
            let platform_timer = Instant::now();
            println!("{:?}: Processing started...", &platform);
            let platform_markets = if let Some(id) = &args.id {
                match &platform {
                    Platform::Manifold => platforms::manifold::get_market_by_id(id).await,
                    Platform::Kalshi => platforms::kalshi::get_market_by_id(id).await,
                    _ => panic!("Unimplemented."),
                }
            } else {
                match &platform {
                    Platform::Manifold => platforms::manifold::get_markets_all().await,
                    Platform::Kalshi => platforms::kalshi::get_markets_all().await,
                    _ => panic!("Unimplemented."),
                }
            };
            println!(
                "{:?}: Processing complete: {:?} markets processed in {:?}.",
                &platform,
                platform_markets.len(),
                platform_timer.elapsed()
            );
            markets.extend(platform_markets);
        }
    });
    println!(
        "All processing complete: {:?} markets processed in {:?}.",
        markets.len(),
        total_timer.elapsed()
    );

    // save collated data to database, stdout, or file
    match args.output.as_str() {
        OUTPUT_KEYWORD_DB => {
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
        }
        OUTPUT_KEYWORD_STDOUT => {
            println!("{}", to_string_pretty(&markets).unwrap())
        }
        filename => to_writer_pretty(&File::create(filename).unwrap(), &markets).unwrap(),
    }
}
