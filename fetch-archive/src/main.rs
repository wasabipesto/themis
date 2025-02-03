//! This binary just parses CLI arguments and passes them to the library run process.

use clap::Parser;
use themis_fetch::platforms::{OutputMethod, Platform};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Override the default platform list to only pull from one provider
    #[arg(short, long)]
    platform: Option<Platform>,

    /// Only pull market data for a single market - requires a single platform to be specified
    #[arg(long)]
    id: Option<String>,

    /// Where to redirect the output
    #[arg(short, long, default_value = "database")]
    output: OutputMethod,

    /// Show additional output for debugging
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();
    themis_fetch::run(args.platform, args.id, args.output, args.verbose);
}
