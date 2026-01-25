//! Error handling types and utilities for the Themis API.

use anyhow::Error as AnyhowError;
use rocket::http::Status;
use rocket::response::{self, Responder};
use rocket::{Request, Response};
use serde::Serialize;
use std::io::Cursor;

/// JSON error response body
#[derive(Debug, Serialize)]
struct ErrorBody {
    status: u16,
    message: String,
}

/// API error types with associated HTTP status codes
#[derive(Debug)]
pub enum ApiError {
    /// Bad request (400)
    BadRequest(String),
    /// Resource not found (404)
    NotFound(String),
    /// Internal server error (500)
    Internal(String),
}

impl ApiError {
    /// Returns the HTTP status code for this error
    fn status(&self) -> Status {
        match self {
            ApiError::BadRequest(_) => Status::BadRequest,
            ApiError::NotFound(_) => Status::NotFound,
            ApiError::Internal(_) => Status::InternalServerError,
        }
    }

    /// Returns the error message string
    fn message(&self) -> &str {
        match self {
            ApiError::BadRequest(msg) => msg,
            ApiError::NotFound(msg) => msg,
            ApiError::Internal(msg) => msg,
        }
    }
}

impl From<AnyhowError> for ApiError {
    fn from(err: AnyhowError) -> Self {
        ApiError::Internal(err.to_string())
    }
}

impl<'r> Responder<'r, 'static> for ApiError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let status = self.status();
        let body = ErrorBody {
            status: status.code,
            message: self.message().to_string(),
        };

        let json = serde_json::to_string(&body).unwrap_or_else(|_| {
            r#"{"status":500,"message":"Failed to serialize error"}"#.to_string()
        });

        Response::build()
            .status(status)
            .header(rocket::http::ContentType::JSON)
            .sized_body(json.len(), Cursor::new(json))
            .ok()
    }
}

/// Extension trait to convert Results into ApiErrors with specific status codes
pub trait ResultExt<T> {
    /// Converts error to BadRequest (400)
    fn bad_request(self) -> Result<T, ApiError>;
    /// Converts error to NotFound (404)
    fn not_found(self) -> Result<T, ApiError>;
}

impl<T, E: std::fmt::Display> ResultExt<T> for Result<T, E> {
    fn bad_request(self) -> Result<T, ApiError> {
        self.map_err(|e| ApiError::BadRequest(e.to_string()))
    }

    fn not_found(self) -> Result<T, ApiError> {
        self.map_err(|e| ApiError::NotFound(e.to_string()))
    }
}
