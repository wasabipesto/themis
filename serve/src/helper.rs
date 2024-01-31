use actix_web::{http::StatusCode, ResponseError};
use diesel::result::Error as DieselError;
use serde_json::json;
use std::fmt;

use super::*;

/// Scale a list of weights down to valid point sizes.
pub fn scale_list(
    list: Vec<f32>,
    output_min: f32,
    output_max: f32,
    output_default: f32,
) -> Vec<f32> {
    if list.is_empty() {
        return Vec::new();
    }

    let input_min = *list
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let input_max = *list
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    if input_min == input_max {
        return vec![output_default; list.len()];
    }

    list.iter()
        .map(|&value| {
            ((value - input_min) / (input_max - input_min)) * (output_max - output_min) + output_min
        })
        .collect()
}

/// Sort all markets into Vecs based on the platform name.
pub fn categorize_markets_by_platform(markets: Vec<Market>) -> HashMap<String, Vec<Market>> {
    let mut markets_by_platform: HashMap<String, Vec<Market>> = HashMap::new();
    for market in markets {
        // this is a hot loop since we iterate over all markets
        if let Some(market_list) = markets_by_platform.get_mut(&market.platform) {
            market_list.push(market);
        } else {
            markets_by_platform.insert(market.platform.clone(), Vec::from([market]));
        }
    }
    markets_by_platform
}

/// A multi-purpose error struct.
#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub status_code: u16,
    pub message: String,
}

impl ApiError {
    pub fn new(status_code: u16, message: String) -> ApiError {
        ApiError {
            status_code,
            message,
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl From<DieselError> for ApiError {
    fn from(error: DieselError) -> ApiError {
        match error {
            DieselError::DatabaseError(_, err) => ApiError::new(409, err.message().to_string()),
            DieselError::NotFound => ApiError::new(404, "Record not found".to_string()),
            err => ApiError::new(500, format!("Diesel error: {}", err)),
        }
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match StatusCode::from_u16(self.status_code) {
            Ok(status_code) => status_code,
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let message = match status_code.as_u16() < 500 {
            true => self.message.clone(),
            false => {
                eprintln!("{}", self.message);
                "Internal server error".to_string()
            }
        };

        HttpResponse::build(status_code).json(json!({ "message": message }))
    }
}
