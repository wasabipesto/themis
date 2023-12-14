use super::*;
use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Deserialize, Clone, Debug)]
struct Market {
    id: String,
    question: String,
    isResolved: bool,
}

#[allow(unused_assignments)]
fn get_markets_all() -> Vec<Market> {
    let api_url = "https://api.manifold.markets/v0/markets";
    let limit = 1000;
    let mut before: Option<String> = None;
    let mut response: Vec<Market> = Vec::with_capacity(limit);
    let mut collated_responses: Vec<Market> = Vec::new();
    let client = reqwest::blocking::Client::new();
    loop {
        response = client
            .get(api_url)
            .query(&[("limit", limit)])
            .query(&[("before", before)])
            .send()
            .unwrap()
            .json::<Vec<Market>>()
            .unwrap();
        if response.len() == limit {
            before = Some(response.last().unwrap().clone().id);
            collated_responses.extend(response);
        } else {
            collated_responses.extend(response);
            break;
        }
    }
    collated_responses
}

fn get_markets_by_id(ids: &Vec<String>) -> Vec<Market> {
    let api_url = "https://api.manifold.markets/v0/market";
    let client = reqwest::blocking::Client::new();
    let mut collated_responses: Vec<Market> = Vec::new();
    for id in ids {
        let response = client
            .get(api_url)
            .query(&[("id", id)])
            .send()
            .unwrap()
            .json::<Market>()
            .unwrap();
        collated_responses.push(response);
    }
    collated_responses
}

fn massage_markets(response_markets: Vec<Market>) -> Vec<MarketForDB> {
    let mut db_markets: Vec<MarketForDB> = Vec::with_capacity(response_markets.len());
    for market in response_markets {
        if market.isResolved {
            db_markets.push(MarketForDB {
                title: market.question,
                platform: Platform::Manifold,
                platform_id: market.id,
            })
        }
    }
    db_markets
}

pub fn get_data(filter_ids: &Option<Vec<String>>) -> Vec<MarketForDB> {
    let response_markets = if let Some(ids) = filter_ids {
        get_markets_by_id(&ids)
    } else {
        get_markets_all()
    };
    massage_markets(response_markets)
}
