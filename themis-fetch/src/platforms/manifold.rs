use super::*;

#[allow(non_snake_case)]
#[derive(Deserialize, Clone, Debug)]
struct MarketInfo {
    id: String,
    question: String,
    isResolved: bool,
}

#[derive(Debug)]
struct MarketFull {
    market: MarketInfo,
    //bets: Bet,
}

impl TryInto<MarketForDB> for MarketFull {
    type Error = MarketConvertError;
    fn try_into(self) -> Result<MarketForDB, MarketConvertError> {
        Ok(MarketForDB {
            title: self.market.question,
            platform: Platform::Manifold,
            platform_id: self.market.id,
        })
    }
}

async fn get_extended_data(market: MarketInfo) -> MarketFull {
    MarketFull { market }
}

#[allow(unused_assignments)]
pub async fn get_markets_all() -> Vec<MarketForDB> {
    let api_url = "https://api.manifold.markets/v0/markets";
    let limit = 1000;
    let mut before: Option<String> = None;
    let mut all_market_data: Vec<MarketFull> = Vec::new();
    let client = reqwest::Client::new();
    loop {
        let query = client
            .get(api_url)
            .query(&[("limit", limit)])
            .query(&[("before", before)]);
        let response = query
            .send()
            .await
            .unwrap()
            .json::<Vec<MarketInfo>>()
            .await
            .unwrap();
        let response_len = response.len();
        let market_data_futures = response.into_iter().map(|market| get_extended_data(market));
        let market_data = join_all(market_data_futures).await;
        all_market_data.extend(market_data);
        if response_len == limit {
            before = Some(all_market_data.last().unwrap().market.id.clone());
        } else {
            break;
        }
    }
    all_market_data
        .into_iter()
        .filter(|m| m.market.isResolved)
        .map(|m| m.try_into().unwrap())
        .collect()
}

pub async fn get_markets_by_id(ids: &Vec<String>) -> Vec<MarketForDB> {
    let api_url = "https://api.manifold.markets/v0/market";
    let client = reqwest::Client::new();
    let mut all_market_data: Vec<MarketFull> = Vec::new();
    for id in ids {
        let response = client
            .get(api_url)
            .query(&[("id", id)])
            .send()
            .await
            .unwrap();
        let response_json = response.json::<MarketInfo>().await.unwrap();
        let market_data = get_extended_data(response_json).await;
        all_market_data.push(market_data);
    }
    all_market_data
        .into_iter()
        .filter(|m| m.market.isResolved)
        .map(|m| m.try_into().unwrap())
        .collect()
}
