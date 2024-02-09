use super::*;

#[derive(Debug, Deserialize, Serialize)]
pub struct MarketListQueryParams {
    #[serde(flatten)]
    pub filters: CommonFilterParams,
    #[serde(flatten)]
    pub list_params: PageSortParams,
}

#[derive(Debug, Serialize)]
pub struct MarketListResponse {
    query: MarketListQueryParams,
    markets: Vec<Market>,
}

pub fn build_market_list(
    query: Query<MarketListQueryParams>,
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
) -> Result<HttpResponse, ApiError> {
    // get markets from database
    let markets = get_markets_filtered(conn, Some(&query.filters), Some(&query.list_params))?;

    let response = MarketListResponse {
        query: query.into_inner(),
        markets,
    };
    Ok(HttpResponse::Ok().json(response))
}
