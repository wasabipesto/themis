use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use std::fs::File;

const OUTPUT_KEYWORD_DB: &str = "db";
const OUTPUT_KEYWORD_STDOUT: &str = "stdout";

#[allow(non_snake_case)]
#[derive(Deserialize, Clone, Debug)]
struct ManifoldLiteMarket {
    id: String,
    isResolved: bool,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Clone, Debug)]
struct ManifoldFullMarket {
    id: String,
    question: String,
}

fn manifold_get_market_ids() -> Vec<String> {
    let api_url = "https://api.manifold.markets/v0/markets";
    let limit = 1000;
    let mut before: Option<String> = None;
    let mut market_ids: Vec<String> = Vec::new();
    let client = reqwest::blocking::Client::new();
    loop {
        let response = client
            .get(api_url)
            .query(&[("limit", limit)])
            .query(&[("before", before)])
            .send()
            .unwrap()
            .json::<Vec<ManifoldLiteMarket>>()
            .unwrap();
        market_ids.append(
            &mut response
                .clone()
                .into_iter()
                .filter(|response| response.isResolved)
                .map(|response| response.id)
                .collect(),
        );
        if response.len() < limit {
            break;
        }
        before = Some(response.last().unwrap().clone().id);
        println!(
            "Manifold: Downloading bulk market data at {}",
            before.clone().unwrap()
        );
    }
    market_ids
}

fn manifold_get_market_data(market_ids: Vec<String>) -> Vec<MarketForDB> {
    let api_url = "https://api.manifold.markets/v0/market";
    let client = reqwest::blocking::Client::new();
    let mut market_data: Vec<MarketForDB> = Vec::new();
    for id in market_ids {
        println!(
            "Manifold: Downloading detailed market data for {}",
            id.clone()
        );
        let response = client
            .get(api_url)
            .query(&[("id", id)])
            .send()
            .unwrap()
            .json::<ManifoldFullMarket>()
            .unwrap();
        market_data.push(MarketForDB {
            title: response.question,
            platform: Platform::Manifold,
            platform_id: response.id,
        })
    }
    market_data
}

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

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize)]
enum Platform {
    Kalshi,
    Manifold,
    Metaculus,
    Polymarket,
    PredictIt,
}

#[derive(Debug, Serialize)]
struct MarketForDB {
    title: String,
    platform: Platform,
    platform_id: String,
}

fn main() {
    let args = Args::parse();
    println!("Initialization: {:?}", &args);

    let platforms: Vec<&Platform> = match &args.platform {
        Some(platform) => Vec::from([platform]),
        None => Vec::from([&Platform::Kalshi, &Platform::Manifold]),
    };
    println!("Initialization: Platforms {:?} selected.", &platforms);

    println!("Initialization: Checking environment variables.");
    // check environment variables
    // - database credentials
    // - kalshi credentials

    println!("Initialization: Starting platform processing.");
    let mut collated_data: Vec<MarketForDB> = Vec::new();
    for platform in platforms.clone() {
        // get all resolved market IDs or just the one requested
        let market_ids: Vec<String> = if let Some(single_id) = &args.id {
            println!("{:?}: Downloading bulk market data...", &platform);
            Vec::from([single_id.to_owned()])
        } else {
            println!("{:?}: Downloading bulk market data...", &platform);
            match platform {
                Platform::Manifold => manifold_get_market_ids(),
                _ => panic!("Unimplemented."),
            }
        };
        // get detailed data for each market and convert into database format
        println!("{:?}: Downloading detailed market data...", &platform);
        collated_data.append(&mut match platform {
            Platform::Manifold => manifold_get_market_data(market_ids),
            _ => panic!("Unimplemented."),
        });
        println!("{:?}: Processing complete.", &platform);
    }
    println!(
        "{:?} platforms processed. {:?} markets processed.",
        platforms.len(),
        collated_data.len()
    );
    // save collated data to database, stdout, or file
    match args.output.as_str() {
        OUTPUT_KEYWORD_DB => println!("Unimplemented."),
        OUTPUT_KEYWORD_STDOUT => {
            println!("{}", serde_json::to_string_pretty(&collated_data).unwrap())
        }
        filename => {
            serde_json::to_writer_pretty(&File::create(filename).unwrap(), &collated_data).unwrap()
        }
    }
}
