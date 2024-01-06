use std::time::Instant;

pub mod platforms;
use platforms::{OutputMethod, Platform};

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
    for platform in platforms {
        let timer = Instant::now();
        println!("{:?}: Processing started...", &platform);
        match (&platform, &id) {
            (Platform::Kalshi, None) => platforms::kalshi::get_markets_all(output, verbose).await,
            (Platform::Kalshi, Some(id)) => {
                platforms::kalshi::get_market_by_id(id, output, verbose).await
            }
            (Platform::Manifold, None) => {
                platforms::manifold::get_markets_all(output, verbose).await
            }
            (Platform::Manifold, Some(id)) => {
                platforms::manifold::get_market_by_id(id, output, verbose).await
            }
        }
        println!(
            "{:?}: Platform complete in {:?}",
            &platform,
            timer.elapsed()
        );
    }
    println!("All platforms complete in {:?}", total_timer.elapsed());
}
