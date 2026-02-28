//! Themis common utilities, definitions, and processes

use serde::{Deserialize, Serialize};

pub mod category;
pub mod criterion;
pub mod db_util;
pub mod market;
pub mod outcome;
pub mod platform;
pub mod probability;
pub mod question;
pub mod scores;
//pub mod xray;

/// URL wrapper type
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Default)]
pub struct Url(String);

/// Generic label text
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Default)]
pub struct Label(String);

impl Label {
    pub fn new(label: &str) -> Self {
        Label(label.to_string())
    }
}
