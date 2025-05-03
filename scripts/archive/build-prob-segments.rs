#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! anyhow = "1.0"
//! serde_json = "1.0"
//! themis-extract = { path = "../extract" }
//! clap = { version = "4.4", features = ["derive"] }
//! ```

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use themis_extract::platforms::*;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory for JSON files
    #[arg(short, long, default_value = "cache")]
    directory: PathBuf,

    /// Platform data file to search
    #[arg(short, long)]
    platform: Platform,

    /// String to search for
    #[arg(short, long)]
    search: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let input = args
        .platform
        .load_line_match(&args.directory, &args.search)?;

    let segments = match input {
        PlatformData::Kalshi(input) => {
            kalshi::build_prob_segments(&input.history, &input.market.close_time)
        }
        PlatformData::Manifold(input) => manifold::build_prob_segments(&input.bets),
        PlatformData::Metaculus(input) => metaculus::build_prob_segments(
            &input
                .extended_data
                .question
                .aggregations
                .recency_weighted
                .history,
            &input.extended_data.actual_close_time,
        )?,
        PlatformData::Polymarket(input) => polymarket::build_prob_segments(&input.prices_history),
    };
    println!("{}", serde_json::to_string(&segments)?);
    Ok(())
}
