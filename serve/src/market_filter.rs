use super::*;

/// Filter parameters common to all queries.
/// Numeric filters are formatted as Vecs:
/// - If the list is empty or not supplied, no filtering is done.
/// - If the list has one parameter, it is taken as the minimum.
/// - If the list has more than one parameter, the last is taken as the maximum.
/// - All numeric values in the schema are positive, so if you want to only limit
///   by the maximum, set the first value to -1.
#[derive(Debug, Deserialize, Clone)]
pub struct CommonFilterParams {
    title_contains: Option<String>,
    platform_select: Option<String>,
    category_select: Option<String>,
    #[serde(default)]
    open_dt: Vec<DateTime<Utc>>,
    #[serde(default)]
    close_dt: Vec<DateTime<Utc>>,
    #[serde(default)]
    open_days: Vec<f32>,
    #[serde(default)]
    volume_usd: Vec<f32>,
    #[serde(default)]
    num_traders: Vec<i32>,
    #[serde(default)]
    prob_at_midpoint: Vec<f32>,
    #[serde(default)]
    prob_at_close: Vec<f32>,
    #[serde(default)]
    prob_time_weighted: Vec<f32>,
    #[serde(default)]
    resolution: Vec<f32>,
}

/// Query markets from the database, applying filters conditionally.
/// If no filters are given, this will get all markets.
pub fn get_markets_filtered(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    params: CommonFilterParams,
) -> Result<Vec<Market>, ApiError> {
    let mut query = market::table.into_boxed();

    if let Some(title_contains) = params.title_contains {
        query = query.filter(market::title.ilike(title_contains))
    }

    if let Some(platform_select) = params.platform_select {
        query = query.filter(market::platform.eq(platform_select))
    }

    if let Some(category_select) = params.category_select {
        query = query.filter(market::category.eq(category_select))
    }

    if let Some(min) = params.open_dt.first() {
        query = query.filter(market::open_dt.ge(min))
    }
    if params.open_dt.len() > 1 {
        let max = params.open_dt.last().unwrap();
        query = query.filter(market::open_dt.le(max))
    }

    if let Some(min) = params.close_dt.first() {
        query = query.filter(market::close_dt.ge(min))
    }
    if params.close_dt.len() > 1 {
        let max = params.close_dt.last().unwrap();
        query = query.filter(market::close_dt.le(max))
    }

    if let Some(min) = params.open_days.first() {
        query = query.filter(market::open_days.ge(min))
    }
    if params.open_days.len() > 1 {
        let max = params.open_days.last().unwrap();
        query = query.filter(market::open_days.le(max))
    }

    if let Some(min) = params.volume_usd.first() {
        query = query.filter(market::volume_usd.ge(min))
    }
    if params.volume_usd.len() > 1 {
        let max = params.volume_usd.last().unwrap();
        query = query.filter(market::volume_usd.le(max))
    }

    if let Some(min) = params.num_traders.first() {
        query = query.filter(market::num_traders.ge(min))
    }
    if params.num_traders.len() > 1 {
        let max = params.num_traders.last().unwrap();
        query = query.filter(market::num_traders.le(max))
    }

    if let Some(min) = params.prob_at_midpoint.first() {
        query = query.filter(market::prob_at_midpoint.ge(min))
    }
    if params.prob_at_midpoint.len() > 1 {
        let max = params.prob_at_midpoint.last().unwrap();
        query = query.filter(market::prob_at_midpoint.le(max))
    }

    if let Some(min) = params.prob_at_close.first() {
        query = query.filter(market::prob_at_close.ge(min))
    }
    if params.prob_at_close.len() > 1 {
        let max = params.prob_at_close.last().unwrap();
        query = query.filter(market::prob_at_close.le(max))
    }

    if let Some(min) = params.prob_time_weighted.first() {
        query = query.filter(market::prob_time_weighted.ge(min))
    }
    if params.prob_time_weighted.len() > 1 {
        let max = params.prob_time_weighted.last().unwrap();
        query = query.filter(market::prob_time_weighted.le(max))
    }

    if let Some(min) = params.resolution.first() {
        query = query.filter(market::resolution.ge(min))
    }
    if params.resolution.len() > 1 {
        let max = params.resolution.last().unwrap();
        query = query.filter(market::resolution.le(max))
    }

    query
        .select(Market::as_select())
        .load::<Market>(conn)
        .map_err(|e| ApiError::new(500, format!("failed to query markets: {e}")))
}
