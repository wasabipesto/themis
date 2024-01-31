use super::*;

/// Filter parameters common to all queries.
#[serde_as]
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CommonFilterParams {
    title_contains: Option<String>,
    platform_select: Option<String>,
    category_select: Option<String>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    open_ts_min: Option<i64>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    open_ts_max: Option<i64>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    close_ts_min: Option<i64>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    close_ts_max: Option<i64>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    open_days_min: Option<f32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    open_days_max: Option<f32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    volume_usd_min: Option<f32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    volume_usd_max: Option<f32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    num_traders_min: Option<i32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    num_traders_max: Option<i32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    prob_at_midpoint_min: Option<f32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    prob_at_midpoint_max: Option<f32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    prob_at_close_min: Option<f32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    prob_at_close_max: Option<f32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    prob_time_weighted_min: Option<f32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    prob_time_weighted_max: Option<f32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    resolution_min: Option<f32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    resolution_max: Option<f32>,
}

/// Pagination and sorting parameters, for listing markets
#[serde_as]
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct PageSortParams {
    #[serde_as(as = "Option<DisplayFromStr>")]
    limit: Option<i64>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    offset: Option<i64>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    sort_attribute: Option<String>,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(default)]
    sort_desc: bool,
}

/// Build a query from the database, applying filters conditionally.
/// If no filters are given, this will get all markets.
pub fn get_markets_filtered(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    common_params: Option<&CommonFilterParams>,
    list_params: Option<&PageSortParams>,
) -> Result<Vec<Market>, ApiError> {
    let mut query = market::table.into_boxed();

    if let Some(params) = common_params {
        if let Some(title_contains) = &params.title_contains {
            query = query.filter(market::title.ilike(title_contains))
        }
        if let Some(platform_select) = &params.platform_select {
            query = query.filter(market::platform.eq(platform_select))
        }
        if let Some(category_select) = &params.category_select {
            query = query.filter(market::category.eq(category_select))
        }
        if let Some(ts) = params.open_ts_min {
            if let Some(dt) = DateTime::from_timestamp(ts, 0) {
                query = query.filter(market::open_dt.ge(dt))
            } else {
                return Err(ApiError::new(
                    400,
                    format!("value for open_dt_min could not be converted into DateTime: {ts}",),
                ));
            }
        }
        if let Some(ts) = params.open_ts_max {
            if let Some(dt) = DateTime::from_timestamp(ts, 0) {
                query = query.filter(market::open_dt.le(dt))
            } else {
                return Err(ApiError::new(
                    400,
                    format!("value for open_ts_max could not be converted into DateTime: {ts}",),
                ));
            }
        }
        if let Some(ts) = params.close_ts_min {
            if let Some(dt) = DateTime::from_timestamp(ts, 0) {
                query = query.filter(market::open_dt.ge(dt))
            } else {
                return Err(ApiError::new(
                    400,
                    format!("value for close_ts_min could not be converted into DateTime: {ts}",),
                ));
            }
        }
        if let Some(ts) = params.close_ts_max {
            if let Some(dt) = DateTime::from_timestamp(ts, 0) {
                query = query.filter(market::open_dt.le(dt))
            } else {
                return Err(ApiError::new(
                    400,
                    format!("value for close_ts_max could not be converted into DateTime: {ts}",),
                ));
            }
        }
        if let Some(min) = params.open_days_min {
            query = query.filter(market::open_days.ge(min))
        }
        if let Some(max) = params.open_days_max {
            query = query.filter(market::open_days.le(max))
        }
        if let Some(min) = params.volume_usd_min {
            query = query.filter(market::volume_usd.ge(min))
        }
        if let Some(max) = params.volume_usd_max {
            query = query.filter(market::volume_usd.le(max))
        }
        if let Some(min) = params.num_traders_min {
            query = query.filter(market::num_traders.ge(min))
        }
        if let Some(max) = params.num_traders_max {
            query = query.filter(market::num_traders.le(max))
        }
        if let Some(min) = params.prob_at_midpoint_min {
            query = query.filter(market::prob_at_midpoint.ge(min))
        }
        if let Some(max) = params.prob_at_midpoint_max {
            query = query.filter(market::prob_at_midpoint.le(max))
        }
        if let Some(min) = params.prob_at_close_min {
            query = query.filter(market::prob_at_close.ge(min))
        }
        if let Some(max) = params.prob_at_close_max {
            query = query.filter(market::prob_at_close.le(max))
        }
        if let Some(min) = params.prob_time_weighted_min {
            query = query.filter(market::prob_time_weighted.ge(min))
        }
        if let Some(max) = params.prob_time_weighted_max {
            query = query.filter(market::prob_time_weighted.le(max))
        }
        if let Some(min) = params.resolution_min {
            query = query.filter(market::resolution.ge(min))
        }
        if let Some(max) = params.resolution_max {
            query = query.filter(market::resolution.le(max))
        }
    }

    if let Some(params) = list_params {
        if let Some(limit) = params.limit {
            query = query.limit(limit)
        }
        if let Some(offset) = params.offset {
            query = query.offset(offset)
        }
        if let Some(sort_attribute) = &params.sort_attribute {
            match sort_attribute.as_str() {
                "title" => match params.sort_desc {
                    false => query = query.order(market::title.asc()),
                    true => query = query.order(market::title.desc()),
                },
                "platform" => match params.sort_desc {
                    false => query = query.order(market::platform.asc()),
                    true => query = query.order(market::platform.desc()),
                },
                "platform_id" => match params.sort_desc {
                    false => query = query.order(market::platform_id.asc()),
                    true => query = query.order(market::platform_id.desc()),
                },
                "url" => match params.sort_desc {
                    false => query = query.order(market::url.asc()),
                    true => query = query.order(market::url.desc()),
                },
                "open_dt" => match params.sort_desc {
                    false => query = query.order(market::open_dt.asc()),
                    true => query = query.order(market::open_dt.desc()),
                },
                "close_dt" => match params.sort_desc {
                    false => query = query.order(market::close_dt.asc()),
                    true => query = query.order(market::close_dt.desc()),
                },
                "open_days" => match params.sort_desc {
                    false => query = query.order(market::open_days.asc()),
                    true => query = query.order(market::open_days.desc()),
                },
                "volume_usd" => match params.sort_desc {
                    false => query = query.order(market::volume_usd.asc()),
                    true => query = query.order(market::volume_usd.desc()),
                },
                "num_traders" => match params.sort_desc {
                    false => query = query.order(market::num_traders.asc()),
                    true => query = query.order(market::num_traders.desc()),
                },
                "category" => match params.sort_desc {
                    false => query = query.order(market::category.asc()),
                    true => query = query.order(market::category.desc()),
                },
                "prob_at_midpoint" => match params.sort_desc {
                    false => query = query.order(market::prob_at_midpoint.asc()),
                    true => query = query.order(market::prob_at_midpoint.desc()),
                },
                "prob_at_close" => match params.sort_desc {
                    false => query = query.order(market::prob_at_close.asc()),
                    true => query = query.order(market::prob_at_close.desc()),
                },
                "prob_time_weighted" => match params.sort_desc {
                    false => query = query.order(market::prob_time_weighted.asc()),
                    true => query = query.order(market::prob_time_weighted.desc()),
                },
                "resolution" => match params.sort_desc {
                    false => query = query.order(market::resolution.asc()),
                    true => query = query.order(market::resolution.desc()),
                },
                _ => {
                    return Err(ApiError::new(
                        400,
                        format!(
                            "value for sort_attribute is not a valid attribute: {sort_attribute}",
                        ),
                    ))
                }
            }
        }
    }

    query
        .select(Market::as_select())
        .load::<Market>(conn)
        .map_err(|e| ApiError::new(500, format!("failed to query markets: {e}")))
}
