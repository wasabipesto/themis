use clap::Parser;
use serde_json::{to_string_pretty, to_writer_pretty};
use std::fs::File;
use std::time::Instant;

pub mod platforms;
use crate::platforms::{MarketForDB, Platform};

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
    #[arg(short, long, default_value = "db")]
    output: String,

    /// Show additional output for debugging
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();
    if args.verbose {
        println!("Initialization: {:?}", &args);
    }

    let platforms: Vec<&Platform> = match &args.platform {
        Some(platform) => Vec::from([platform]),
        None => Vec::from([&Platform::Kalshi, &Platform::Manifold]),
    };

    let market_ids: Option<Vec<String>> = if let Some(market_id) = &args.id {
        Some(Vec::from([market_id.clone()]))
    } else {
        None
    };

    if args.verbose {
        println!("Initialization: Checking environment variables...");
    }

    if args.verbose {
        println!("Initialization: Processing platforms: {:?}", &platforms);
    }
    let total_timer = Instant::now();
    let mut markets: Vec<MarketForDB> = Vec::new();
    for platform in platforms.clone() {
        let platform_timer = Instant::now();
        println!("{:?}: Processing started...", &platform);
        let platform_markets = match &platform {
            Platform::Manifold => {
                if let Some(ids) = &market_ids {
                    platforms::manifold::get_markets_by_id(ids)
                } else {
                    platforms::manifold::get_markets_all()
                }
            }
            Platform::Kalshi => {
                if let Some(ids) = &market_ids {
                    platforms::kalshi::get_markets_by_id(ids)
                } else {
                    platforms::kalshi::get_markets_all()
                }
            }
            _ => panic!("Unimplemented."),
        };
        println!(
            "{:?}: Processing complete: {:?} markets processed in {:?}.",
            &platform,
            platform_markets.len(),
            platform_timer.elapsed()
        );
        markets.extend(platform_markets);
    }
    println!(
        "All processing complete: {:?} markets processed in {:?}.",
        markets.len(),
        total_timer.elapsed()
    );
    // save collated data to database, stdout, or file
    match args.output.as_str() {
        OUTPUT_KEYWORD_DB => println!("Unimplemented."),
        OUTPUT_KEYWORD_STDOUT => {
            println!("{}", to_string_pretty(&markets).unwrap())
        }
        filename => to_writer_pretty(&File::create(filename).unwrap(), &markets).unwrap(),
    }
}
