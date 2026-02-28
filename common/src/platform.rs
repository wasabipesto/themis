//! Platform-related data structures and utilities.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Formatted platform name
#[derive(
    Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Default, derive_more::Display,
)]
pub struct PlatformName(pub String);

/// Slugified platform name
#[derive(
    Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Default, derive_more::Display,
)]
pub struct PlatformSlug(pub String);

/// A specific market platform
#[derive(Debug, Serialize, Deserialize, Clone, derive_more::Display)]
pub enum Platforms {
    Kalshi,
    Manifold,
    Metaculus,
    Polymarket,
}

pub struct Platform {
    platform_slug: PlatformSlug,
    platform_name: PlatformName,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.platform_name)
    }
}
