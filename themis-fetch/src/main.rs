use clap::Parser;
use serde_json::{to_string_pretty, to_writer_pretty};
use std::fs::File;

pub mod platforms;
use crate::platforms::manifold;
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
    println!("Initialization: {:?}", &args);

    let platforms: Vec<&Platform> = match &args.platform {
        Some(platform) => Vec::from([platform]),
        None => Vec::from([&Platform::Kalshi, &Platform::Manifold]),
    };

    let filter_ids: Option<Vec<String>> = if let Some(filter_id) = &args.id {
        Some(Vec::from([filter_id.clone()]))
    } else {
        None
    };

    println!("Initialization: Checking environment variables...");
    // check environment variables
    // - database credentials
    // - kalshi credentials

    println!("Initialization: Processing platforms: {:?}", &platforms);
    let mut markets: Vec<MarketForDB> = Vec::new();
    for platform in platforms.clone() {
        println!("{:?}: Processing started...", &platform);
        markets.append(&mut match platform {
            Platform::Manifold => manifold::get_data(&filter_ids),
            _ => panic!("Unimplemented."),
        });
        println!("{:?}: Processing complete.", &platform);
    }
    println!(
        "All processing complete: {:?} platforms processed. {:?} markets processed.",
        platforms.len(),
        markets.len()
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
