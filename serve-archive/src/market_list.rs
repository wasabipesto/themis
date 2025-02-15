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
    total_markets: usize,
    markets: Vec<Market>,
}

pub fn build_market_list(
    query: Query<MarketListQueryParams>,
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
) -> Result<HttpResponse, ApiError> {
    // get markets from database
    let (markets, total_markets) =
        get_markets_filtered(conn, Some(&query.filters), Some(&query.list_params))?;

    let response = MarketListResponse {
        query: query.into_inner(),
        total_markets,
        markets,
    };
    Ok(HttpResponse::Ok().json(response))
}
