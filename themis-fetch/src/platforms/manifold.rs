use super::*;

const MANIFOLD_API_BASE: &str = "https://api.manifold.markets/v0";
const MANIFOLD_SITE_BASE: &str = "https://manifold.markets/";

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct MarketInfo {
    id: String,
    question: String,
    slug: String,
    creatorUsername: String,
    isResolved: bool,
}

#[derive(Debug)]
struct MarketFull {
    market: MarketInfo,
    //bets: Bet,
}

impl TryInto<Option<MarketForDB>> for MarketFull {
    type Error = MarketConvertError;
    fn try_into(self) -> Result<Option<MarketForDB>, MarketConvertError> {
        if self.market.isResolved {
            Ok(Some(MarketForDB {
                title: self.market.question,
                platform: Platform::Manifold,
                platform_id: self.market.id,
                url: MANIFOLD_SITE_BASE.to_owned()
                    + &self.market.creatorUsername
                    + "/"
                    + &self.market.slug,
            }))
        } else {
            Ok(None)
        }
    }
}

fn get_extended_data(market: MarketInfo) -> MarketFull {
    MarketFull { market }
}

pub fn get_market_by_id(id: &String) -> Vec<MarketForDB> {
    let client = get_default_client();
    let api_url = MANIFOLD_API_BASE.to_owned() + "/market";
    let response = client
        .get(&api_url)
        .query(&[("id", id)])
        .send()
        .unwrap()
        .json::<MarketInfo>()
        .unwrap();
    let market_data = get_extended_data(response);
    Vec::from([TryInto::<Option<MarketForDB>>::try_into(market_data)
        .expect("Error processing market")
        .expect("Market is not resolved.")])
}

pub fn get_markets_all() -> Vec<MarketForDB> {
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
        .into_iter()
        .filter(|m| m.market.isResolved)
        .map(|m| {
            TryInto::<Option<MarketForDB>>::try_into(m)
                .unwrap()
                .unwrap()
        })
        .collect()
}
