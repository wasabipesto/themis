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

    let line = args
        .platform
        .load_line_match(&args.directory, &args.search)?;

    let segments = match line {
        PlatformData::Kalshi(line) => {
            kalshi::build_prob_segments(&line.history, &line.market.close_time)
        }
        PlatformData::Manifold(_line) => todo!(),
        PlatformData::Metaculus(_line) => todo!(),
        PlatformData::Polymarket(_line) => todo!(),
    };
    println!("{}", serde_json::to_string(&segments)?);
    Ok(())
}
