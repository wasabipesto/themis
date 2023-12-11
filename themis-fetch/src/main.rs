use clap::{Parser, ValueEnum};

/// Simple program to greet a person
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Platform {
    Kalshi,
    Manifold,
    Metaculus,
    Polymarket,
    PredictIt,
}

fn main() {
    let args = Args::parse();
    // get platform list
    let platforms: Vec<&Platform> = match &args.platform {
        Some(platform) => Vec::from([platform]),
        None => Vec::from([&Platform::Kalshi, &Platform::Manifold]),
    };
    println!("Getting data from {:?}", &platforms)
}
