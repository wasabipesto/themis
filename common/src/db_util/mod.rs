//! Themis database utilities

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};

pub mod debug;
pub mod pool;

/// Database connection from the pool
pub type DbConn = PooledConnection<ConnectionManager<PgConnection>>;
