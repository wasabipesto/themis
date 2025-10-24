//! Anything that switches based on platform.

use anyhow::{Context, Result};
use clap::ValueEnum;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::{MarketAndProbs, MarketResult};

pub mod kalshi;
pub mod manifold;
pub mod metaculus;
pub mod polymarket;

/// Supported platforms.
#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum Platform {
    Kalshi,
    Manifold,
    Metaculus,
    Polymarket,
}

/// Deserialized JSONL line straight from the disk. One of any platform type.
/// Boxed due to large size differences between each platform.
#[derive(Clone)]
pub enum PlatformData {
    Kalshi(Box<kalshi::KalshiData>),
    Manifold(Box<manifold::ManifoldData>),
    Metaculus(Box<metaculus::MetaculusData>),
    Polymarket(Box<polymarket::PolymarketData>),
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Platform::Kalshi => write!(f, "Kalshi"),
            Platform::Manifold => write!(f, "Manifold"),
            Platform::Metaculus => write!(f, "Metaculus"),
            Platform::Polymarket => write!(f, "Polymarket"),
        }
    }
}

impl Platform {
    pub fn all() -> Vec<Platform> {
        vec![
            Platform::Kalshi,
            Platform::Manifold,
            Platform::Metaculus,
            Platform::Polymarket,
        ]
    }

    /// Based on platform, deserialize a line into that platform's datatype.
    pub fn deserialize_line(&self, line: &str) -> Result<PlatformData> {
        match self {
            Platform::Kalshi => Ok(PlatformData::Kalshi(serde_json::from_str(line)?)),
            Platform::Manifold => Ok(PlatformData::Manifold(serde_json::from_str(line)?)),
            Platform::Metaculus => Ok(PlatformData::Metaculus(serde_json::from_str(line)?)),
            Platform::Polymarket => Ok(PlatformData::Polymarket(serde_json::from_str(line)?)),
        }
    }

    /// Find the first line in the platform data file matching the search term and deserialize it.
    pub fn load_line_match(&self, base_dir: &Path, search: &str) -> Result<PlatformData> {
        let file_name = format!("{}-data.jsonl", self).to_lowercase();
        let data_file_path = base_dir.join(file_name);

        let file = File::open(&data_file_path)
            .with_context(|| format!("Failed to open file: {}", data_file_path.display()))?;
        let reader = BufReader::new(file);

        for (line_number, line) in reader.lines().enumerate() {
            match line {
                Ok(line_content) => {
                    if line_content.contains(search) {
                        return self.deserialize_line(&line_content).with_context(|| {
                            format!("Failed to deserialize matching line {}", line_number + 1)
                        });
                    }
                }
                Err(err) => {
                    log::error!("Failed to read line {}: {}", line_number + 1, err);
                    continue;
                }
            }
        }

        anyhow::bail!("No line found containing search term: {}", search)
    }

    /// Find the appropriate data file based on platform name, then load and deserialize all lines.
    pub fn load_data(&self, base_dir: &Path, fail_fast: &bool) -> Result<Vec<PlatformData>> {
        let file_name = format!("{}-data.jsonl", self).to_lowercase();
        let data_file_path = base_dir.join(file_name);

        let file = File::open(&data_file_path)
            .with_context(|| format!("Failed to open file: {}", data_file_path.display()))?;
        let reader = BufReader::new(file);

        let mut result = Vec::new();
        for (line_number, line) in reader.lines().enumerate() {
            match line {
                Ok(line_content) => match self.deserialize_line(&line_content) {
                    Ok(item) => {
                        result.push(item);
                    }
                    Err(err) => {
                        let mut err_msg = format!(
                            "Failed to deserialize file {} line {}: {}",
                            data_file_path.display(),
                            line_number + 1,
                            err
                        );

                        // Try to extract column number from error message and show context
                        let err_str = err.to_string();

                        // Extract column number using simple string parsing
                        if let Some(column_start) = err_str.find("column ") {
                            let column_substr = &err_str[column_start + 7..];
                            // Find where the number ends - look for first non-digit or end of string
                            let column_end = column_substr
                                .find(|c: char| !c.is_ascii_digit())
                                .unwrap_or(column_substr.len());
                            let column_str = &column_substr[..column_end];

                            if !column_str.is_empty() {
                                if let Ok(column) = column_str.parse::<usize>() {
                                    if column > 0 && column <= line_content.len() {
                                        let char_pos = column - 1; // Convert to 0-based index

                                        // Handle UTF-8 properly by working with char boundaries
                                        let chars: Vec<char> = line_content.chars().collect();
                                        if char_pos < chars.len() {
                                            let start_char_pos = char_pos.saturating_sub(25);
                                            let end_char_pos =
                                                std::cmp::min(char_pos + 25, chars.len());

                                            let before: String =
                                                chars[start_char_pos..char_pos].iter().collect();
                                            let at_pos = chars[char_pos];
                                            let after: String =
                                                chars[char_pos + 1..end_char_pos].iter().collect();

                                            err_msg.push_str(&format!(
                                                " - Context around column {}: `{}[{}]{}`",
                                                column, before, at_pos, after
                                            ));
                                        } else {
                                            err_msg.push_str(&format!(
                                                " - Additionally, column {} exceeds character count (chars: {}).",
                                                column,
                                                chars.len()
                                            ));
                                        }
                                    } else {
                                        err_msg.push_str(&format!(
                                            " - Additionally, column {} out of bounds (line length: {}).",
                                            column,
                                            line_content.len()
                                        ));
                                    }
                                } else {
                                    err_msg.push_str(&format!(
                                        " - Additionally, could not parse column number from: '{}'",
                                        column_str
                                    ));
                                }
                            } else {
                                err_msg.push_str("- Empty column number found in error message.");
                            }
                        }

                        log::error!("{}", err_msg);
                        if *fail_fast {
                            anyhow::bail!(err_msg);
                        }
                    }
                },
                Err(err) => {
                    let err_msg = format!(
                        "Failed to deserialize file {} line {}: {}",
                        data_file_path.display(),
                        line_number + 1,
                        err
                    );
                    log::error!("{}", err_msg);
                    anyhow::bail!(err_msg);
                }
            }
        }
        Ok(result)
    }

    /// Call each platform's standardize function.
    pub fn standardize(&self, input_unsorted: PlatformData) -> MarketResult<Vec<MarketAndProbs>> {
        match input_unsorted {
            PlatformData::Kalshi(input) => kalshi::standardize(&input),
            PlatformData::Manifold(input) => manifold::standardize(&input),
            PlatformData::Metaculus(input) => metaculus::standardize(&input),
            PlatformData::Polymarket(input) => polymarket::standardize(&input),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde_json_error_format() {
        // Test what serde_json error messages look like with various malformed JSON
        let test_cases = [
            r#"{"valid": "json", "but": "missing quote here, "invalid": "field"}"#,
            r#"{"valid": true, "another": false, "bad_comma":, "field": "value"}"#,
            r#"{"unclosed": "string"#,
            r#"{"trailing": "comma",}"#,
        ];

        for (i, malformed_json) in test_cases.iter().enumerate() {
            println!("Test case {}: {}", i + 1, malformed_json);
            let result: Result<serde_json::Value, _> = serde_json::from_str(malformed_json);
            if let Err(err) = result {
                println!("  Serde JSON error: {}", err);

                // Test our parsing logic
                let err_str = err.to_string();
                if let Some(column_start) = err_str.find("column ") {
                    let column_substr = &err_str[column_start + 7..];
                    let column_end = column_substr
                        .find(|c: char| !c.is_ascii_digit())
                        .unwrap_or(column_substr.len());
                    let column_str = &column_substr[..column_end];

                    if !column_str.is_empty() {
                        if let Ok(column) = column_str.parse::<usize>() {
                            println!("  Extracted column: {}", column);

                            // Show context like our real code does
                            if column > 0 && column <= malformed_json.len() {
                                let chars: Vec<char> = malformed_json.chars().collect();
                                if column - 1 < chars.len() {
                                    let char_pos = column - 1;
                                    let start_char_pos = char_pos.saturating_sub(25);
                                    let end_char_pos = std::cmp::min(char_pos + 25, chars.len());

                                    let before: String =
                                        chars[start_char_pos..char_pos].iter().collect();
                                    let at_pos = chars[char_pos];
                                    let after: String =
                                        chars[char_pos + 1..end_char_pos].iter().collect();

                                    println!("  Context: \"{}[{}]{}\"", before, at_pos, after);
                                }
                            }
                        }
                    }
                } else {
                    println!("  No column information found");
                }
            }
            println!();
        }
    }
}
