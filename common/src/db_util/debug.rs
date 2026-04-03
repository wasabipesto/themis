//! Themis database utilities

use anyhow::{Context, Result};
#[allow(clippy::wildcard_imports)]
use diesel::prelude::*;
#[allow(clippy::wildcard_imports)]
use diesel::sql_types::*;
use serde::{Deserialize, Serialize};

use super::DbConn;

/// Some debug information about the current table state
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, QueryableByName)]
pub struct TableDebugInfo {
    #[diesel(sql_type = Text)]
    pub table_name: String,
    #[diesel(sql_type = BigInt)]
    pub row_count: i64,
    #[diesel(sql_type = Nullable<BigInt>)]
    pub total_size_bytes: Option<i64>,
    #[diesel(sql_type = Nullable<BigInt>)]
    pub table_size_bytes: Option<i64>,
    #[diesel(sql_type = Nullable<BigInt>)]
    pub index_size_bytes: Option<i64>,
    #[diesel(sql_type = Nullable<BigInt>)]
    pub live_tuples: Option<i64>,
    #[diesel(sql_type = Nullable<BigInt>)]
    pub dead_tuples: Option<i64>,
    #[diesel(sql_type = Nullable<Timestamp>)]
    pub last_vacuum: Option<chrono::NaiveDateTime>,
    #[diesel(sql_type = Nullable<Timestamp>)]
    pub last_autovacuum: Option<chrono::NaiveDateTime>,
    #[diesel(sql_type = Nullable<Timestamp>)]
    pub last_analyze: Option<chrono::NaiveDateTime>,
    #[diesel(sql_type = Nullable<Timestamp>)]
    pub last_autoanalyze: Option<chrono::NaiveDateTime>,
}

/// Get all tables and their row counts, for debugging
///
/// # Errors
/// Returns an error if:
/// - The database connection fails
/// - The SQL query fails to evaluate
/// - The returned data fails to deserialize
pub fn get_table_debug_info(conn: &mut DbConn) -> Result<Vec<TableDebugInfo>> {
    let query = "
        SELECT
            t.tablename as table_name,
            (xpath('/row/cnt/text()', xml_count))[1]::text::bigint as row_count,
            pg_total_relation_size(quote_ident(t.tablename)::regclass) as total_size_bytes,
            pg_relation_size(quote_ident(t.tablename)::regclass) as table_size_bytes,
            pg_indexes_size(quote_ident(t.tablename)::regclass) as index_size_bytes,
            s.n_live_tup as live_tuples,
            s.n_dead_tup as dead_tuples,
            s.last_vacuum,
            s.last_autovacuum,
            s.last_analyze,
            s.last_autoanalyze
        FROM (
            SELECT
                tablename,
                query_to_xml(format('SELECT COUNT(*) as cnt FROM %I.%I', schemaname, tablename), false, true, '') as xml_count
            FROM pg_tables
            WHERE schemaname = 'public'
        ) t
        LEFT JOIN pg_stat_user_tables s ON s.relname = t.tablename AND s.schemaname = 'public'
        ORDER BY table_name
    ";

    let results: Vec<TableDebugInfo> = diesel::sql_query(query)
        .load(conn)
        .context("Failed to execute query")?;

    Ok(results)
}
