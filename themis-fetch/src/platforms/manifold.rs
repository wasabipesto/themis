use super::*;
use serde::Deserialize;

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

fn get_extended_data(market: MarketInfo) -> MarketFull {
    MarketFull { market }
}

#[allow(unused_assignments)]
fn get_markets_all() -> Vec<MarketFull> {
    let api_url = "https://api.manifold.markets/v0/markets";
    let limit = 1000;
    let mut before: Option<String> = None;
    let mut all_market_data: Vec<MarketFull> = Vec::new();
    let client = reqwest::blocking::Client::new();
    loop {
        let response: Vec<MarketInfo> = client
            .get(api_url)
            .query(&[("limit", limit)])
            .query(&[("before", before)])
            .send()
            .unwrap()
            .json::<Vec<MarketInfo>>()
            .unwrap();
        let response_len = response.len();
        let market_data: Vec<MarketFull> = response
            .into_iter()
            .map(|market| get_extended_data(market))
            .collect();
        all_market_data.extend(market_data);
        if response_len == limit {
            before = Some(all_market_data.last().unwrap().market.id.clone());
        } else {
            break;
        }
    }
    all_market_data
}

fn get_markets_by_id(ids: &Vec<String>) -> Vec<MarketFull> {
    let api_url = "https://api.manifold.markets/v0/market";
    let client = reqwest::blocking::Client::new();
    let mut all_market_data: Vec<MarketFull> = Vec::new();
    for id in ids {
        let response = client
            .get(api_url)
            .query(&[("id", id)])
            .send()
            .unwrap()
            .json::<MarketInfo>()
            .unwrap();
        let market_data = get_extended_data(response);
        all_market_data.push(market_data);
    }
    all_market_data
}

pub fn get_data(filter_ids: &Option<Vec<String>>) -> Vec<MarketForDB> {
    let markets = if let Some(ids) = filter_ids {
        get_markets_by_id(&ids)
    } else {
        get_markets_all()
    };
    markets
        .into_iter()
        .filter(|m| m.market.isResolved)
        .map(|m| m.try_into().unwrap())
        .collect()
}
