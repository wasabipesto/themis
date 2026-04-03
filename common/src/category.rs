//! A high-level topic category

use serde::{Deserialize, Serialize};

/// Formatted category name
#[derive(
    Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Default, derive_more::Display,
)]
pub struct CategoryName(pub String);

/// Slugified category name
#[derive(
    Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd, Default, derive_more::Display,
)]
pub struct CategorySlug(pub String);

/// Standard category information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Category {
    pub slug: CategorySlug,
    pub name: CategoryName,
}
