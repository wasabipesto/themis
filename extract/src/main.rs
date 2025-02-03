use clap::Parser;
use log::{debug, info};
use std::env;
use std::path::PathBuf;

use themis_extract::platforms::{Platform, PlatformHandler};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about)]
struct Args {
    /// Override the default platform list to only process one platform
    #[arg(short, long)]
    platform: Option<Platform>,

    /// Directory for JSON files
    #[arg(short, long, default_value = "../output")]
    directory: PathBuf,

    /// Set the log level (e.g., error, warn, info, debug, trace)
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

fn main() {
    // get command line args
    let args = Args::parse();

    // read log level from arg and update environment variable
    let log_level = args.log_level.to_lowercase();
    match log_level.as_str() {
        "error" | "warn" | "info" | "debug" | "trace" => env::set_var("RUST_LOG", log_level),
        _ => {
            // invalid, reset to 'info' as a default
            println!("Invalid log level, resetting to INFO.");
            env::set_var("RUST_LOG", "info")
        }
    }
    env_logger::init();
    debug!("Command line args: {:?}", args);

    // if the user requested a specific platform, format it into a list
    // otherwise, return the default platform list
    let platforms: Vec<Platform> = match args.platform {
        Some(platform) => Vec::from([platform]),
        None => Platform::all(),
    };
    debug!("Platforms to process: {:?}", platforms);

    info!("Loading data from file.");
    for platform in platforms {
        let items = platform
            .load_data(&args.directory)
            .expect("Failed to load platform data");
        info!("{platform}: {} items loaded.", items.len());
    }
}
