use super::*;
use std::cmp;

const MANIFOLD_API_BASE: &str = "https://api.manifold.markets/v0";
const MANIFOLD_SITE_BASE: &str = "https://manifold.markets/";
const MILLIS_PER_DAY: f32 = (1000 * 60 * 60 * 24) as f32;

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct MarketInfo {
    id: String,
    question: String,
    slug: String,
    creatorUsername: String,
    isResolved: bool,
    createdTime: u64,
    closeTime: Option<i64>, // polls and bounties lack close times, can be in the past
    resolutionTime: Option<u64>,
}

impl MarketInfoDetails for MarketInfo {
    fn is_valid(&self) -> bool {
        self.isResolved
    }
}

#[derive(Debug)]
struct MarketFull {
    market: MarketInfo,
    //bets: Bet,
}

impl MarketFullDetails for MarketFull {
    fn title(&self) -> String {
        self.market.question.to_owned()
    }
    fn platform(&self) -> String {
        "manifold".to_string()
    }
    fn platform_id(&self) -> String {
        self.market.id.to_owned()
    }
    fn url(&self) -> String {
        MANIFOLD_SITE_BASE.to_owned() + &self.market.creatorUsername + "/" + &self.market.slug
    }
    fn open_days(&self) -> Result<f32, MarketConvertError> {
        match (self.market.resolutionTime, self.market.closeTime) {
            (None, None) => Err(MarketConvertError::new(
                format!("{:?}", self),
                "Manifold API response did not include closeTime or resolutionTime for resolved market",
            )),
            (Some(resolution_time), None) => Ok((resolution_time - self.market.createdTime) as f32 / MILLIS_PER_DAY),
            (None, Some(close_time)) => Ok((close_time.max(0) as u64 - self.market.createdTime) as f32 / MILLIS_PER_DAY),
            (Some(resolution_time), Some(close_time)) => {
                if (close_time.max(0) as u64) < self.market.createdTime {
                    Ok((resolution_time - self.market.createdTime) as f32 / MILLIS_PER_DAY)
                } else {
                    Ok((cmp::min(close_time as u64, resolution_time) - self.market.createdTime) as f32 / MILLIS_PER_DAY)}
                },
        }
    }
}

impl TryInto<MarketForDB> for MarketFull {
    type Error = MarketConvertError;
    fn try_into(self) -> Result<MarketForDB, MarketConvertError> {
        Ok(MarketForDB {
            title: self.title(),
            platform: self.platform(),
            platform_id: self.platform_id(),
            url: self.url(),
            open_days: self.open_days()?,
        })
    }
}

fn get_extended_data(market: MarketInfo) -> MarketFull {
    MarketFull { market }
}

pub async fn get_market_by_id(id: &String) -> Vec<MarketForDB> {
    let client = get_default_client();
    let api_url = MANIFOLD_API_BASE.to_owned() + "/market/" + &id;
    let response = client
        .get(&api_url)
        .send()
        .await
        .unwrap()
        .json::<MarketInfo>()
        .await
        .unwrap();
    let market_data = get_extended_data(response);
    Vec::from([market_data.try_into().expect("Error processing market")])
}

pub async fn get_markets_all() -> Vec<MarketForDB> {
    let client = get_default_client();
    let api_url = MANIFOLD_API_BASE.to_owned() + "/markets";
    let limit = 1000;
    let mut before: Option<String> = None;
    let mut all_market_data: Vec<MarketFull> = Vec::new();
    loop {
        let response: Vec<MarketInfo> = client
            .get(&api_url)
            .query(&[("limit", limit)])
            .query(&[("before", before)])
            .send()
            .await
            .unwrap()
            .json::<Vec<MarketInfo>>()
            .await
            .unwrap();
        let response_len = response.len();
        let market_data: Vec<MarketFull> = response
            .into_iter()
            .filter(|market| market.is_valid())
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
        .into_iter()
        .map(|market| {
            market
                .try_into()
                .expect("Error converting market into standard fields.")
        })
        .collect()
}
