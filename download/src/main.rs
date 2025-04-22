//! Themis fetch binary source.
//! Be warned: running this with all platforms enabled takes a lot of memory and disk space!

use chrono::DateTime;
use chrono::Duration;
use chrono::Utc;
use clap::Parser;
use log::{debug, info};
use std::env;
use std::fs;
use std::path::PathBuf;
use tokio::task::JoinHandle;

use themis_download::platforms::{Platform, PlatformHandler};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about)]
struct Args {
    /// Override the default platform list to only pull from one platform
    #[arg(short, long)]
    platform: Option<Platform>,

    /// Output directory for JSON files
    #[arg(short, long, default_value = "../cache")]
    output_dir: PathBuf,

    /// Only download markets that resolved since this date/time (ISO 8601)
    #[arg(long)]
    resolved_since: Option<DateTime<Utc>>,

    /// Only download markets that resolved since this many days ago
    #[arg(long)]
    resolved_since_days_ago: Option<i64>,

    /// Reset the index before resuming cache downloads to catch new items
    #[arg(long)]
    reset_index: bool,

    /// Completely reset the data cache, restarting the download from scratch
    #[arg(long)]
    reset_cache: bool,

    /// Set the log level (e.g., error, warn, info, debug, trace)
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
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

    // ensure output directory exists
    // if it doesn't exist, create it
    let output_dir = args.output_dir;
    if !output_dir.exists() {
        info!("Creating output directory \"{}\"", output_dir.display());
        fs::create_dir_all(&output_dir).expect("Could not create output directory!");
    }

    // start the download for all platforms in parallel
    let resolved_since = match args.resolved_since_days_ago {
        Some(days_ago) => Some(Utc::now() - Duration::days(days_ago)),
        None => args.resolved_since,
    };
    let tasks: Vec<JoinHandle<()>> = platforms
        .into_iter()
        .map(|platform| {
            tokio::spawn({
                let output_dir = output_dir.clone();
                async move {
                    platform
                        .download(
                            &output_dir,
                            &args.reset_index,
                            &args.reset_cache,
                            &resolved_since,
                        )
                        .await;
                }
            })
        })
        .collect();
    futures::future::try_join_all(tasks)
        .await
        .expect("Failed to join tasks");
    info!("All platform downloads complete.");
}
