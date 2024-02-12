use super::*;

// Diesel macro to get database schema.
table! {
    market (id) {
        id -> Int4,
        title -> Varchar,
        platform -> Varchar,
        platform_id -> Varchar,
        url -> Varchar,
        open_dt -> Timestamptz,
        close_dt -> Timestamptz,
        open_days -> Float,
        volume_usd -> Float,
        num_traders -> Integer,
        category -> Varchar,
        prob_at_midpoint -> Float,
        prob_at_close -> Float,
        prob_time_avg -> Float,
        resolution -> Float,
    }
}

/// Data returned from the database, same as what we inserted.
#[derive(Debug, Queryable, Serialize, Selectable, Clone)]
#[diesel(table_name = market)]
pub struct Market {
    pub title: String,
    pub platform: String,
    pub platform_id: String,
    pub url: String,
    pub open_dt: DateTime<Utc>,
    pub close_dt: DateTime<Utc>,
    pub open_days: f32,
    pub volume_usd: f32,
    pub num_traders: i32,
    pub category: String,
    pub prob_at_midpoint: f32,
    pub prob_at_close: f32,
    pub prob_time_avg: f32,
    pub resolution: f32,
}

// Diesel macro to get database schema.
table! {
    platform (name) {
        name -> Varchar,
        name_fmt -> Varchar,
        description -> Varchar,
        avatar_url -> Varchar,
        site_url -> Varchar,
        color -> Varchar,
        color_accent -> Varchar,
    }
}

/// Data about a platform cached in the database.
#[derive(Debug, Queryable, Serialize, Selectable)]
#[diesel(table_name = platform)]
pub struct Platform {
    pub name: String,
    pub name_fmt: String,
    pub description: String,
    pub avatar_url: String,
    pub site_url: String,
    pub color: String,
    pub color_accent: String,
}

/// Get information about a platform from the database.
pub fn get_platform_by_name(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    platform_req: &String,
) -> Result<Platform, ApiError> {
    use crate::platform::dsl::*;
    platform
        .find(&platform_req)
        .first(conn)
        .map_err(|e| ApiError::new(500, format!("failed to query db for {platform_req}: {e}")))
}

/// Get all data on all platforms.
pub fn get_all_platforms(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
) -> Result<Vec<Platform>, ApiError> {
    platform::table
        .select(Platform::as_select())
        .load::<Platform>(conn)
        .map_err(|e| ApiError::new(500, format!("failed to query db for platforms: {e}")))
}
