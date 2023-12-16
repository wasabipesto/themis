use super::*;
use reqwest::blocking::Client;

const KALSHI_API_BASE: &str = "https://demo-api.kalshi.co/trade-api/v2";

#[derive(Serialize, Debug)]
struct LoginCredentials {
    email: String,
    password: String,
}

#[derive(Deserialize, Debug)]
struct LoginResponse {
    token: String,
}

#[derive(Deserialize, Debug)]
struct MarketInfo {
    ticker: String,
    title: String,
}

#[derive(Deserialize, Debug)]
struct SingleMarketResponse {
    market: MarketInfo,
}

#[derive(Deserialize, Debug)]
struct BulkMarketResponse {
    markets: Vec<MarketInfo>,
    cursor: String,
}

#[derive(Debug)]
struct MarketFull {
    market: MarketInfo,
    //bets: Bet,
}

impl TryInto<Option<MarketForDB>> for MarketFull {
    type Error = MarketConvertError;
    fn try_into(self) -> Result<Option<MarketForDB>, MarketConvertError> {
        Ok(Some(MarketForDB {
            title: self.market.title,
            platform: Platform::Kalshi,
            platform_id: self.market.ticker,
        }))
    }
}

fn get_extended_data(market: MarketInfo) -> MarketFull {
    MarketFull { market }
}

fn get_login_token(client: &Client) -> String {
    let api_url = KALSHI_API_BASE.to_owned() + "/login";
    let credentials = LoginCredentials {
        email: var("KALSHI_USERNAME")
            .expect("Required environment variable KALSHI_USERNAME not set."),
        password: var("KALSHI_PASSWORD")
            .expect("Required environment variable KALSHI_PASSWORD not set."),
    };
    //println!("Kalshi: Logging in with: {:?}", credentials);
    let response = client
        .post(api_url)
        .json(&credentials)
        .send()
        .unwrap()
        .error_for_status()
        .unwrap_or_else(|e| panic!("Kalshi: Login failed: {:?}", e))
        .json::<LoginResponse>()
        .unwrap();
    //println!("Kalshi: Logged in with token: {}", &response.token);
    response.token
}

pub fn get_markets_by_id(ids: &Vec<String>) -> Vec<MarketForDB> {
    let client = Client::new();
    let token = get_login_token(&client);
    let api_url = KALSHI_API_BASE.to_owned() + "/markets/";
    let mut all_market_data: Vec<MarketFull> = Vec::new();
    for id in ids {
        let response = client
            .get(api_url.clone() + id)
            .bearer_auth(&token)
            .send()
            .unwrap()
            .json::<SingleMarketResponse>()
            .unwrap();
        println!("{:?}", response);
        let market_data = get_extended_data(response.market);
        all_market_data.push(market_data);
    }
    all_market_data
        .into_iter()
        .map(|m| {
            TryInto::<Option<MarketForDB>>::try_into(m)
                .unwrap()
                .unwrap()
        })
        .collect()
}

pub fn get_markets_all() -> Vec<MarketForDB> {
    let client = Client::new();
    let token = get_login_token(&client);
    let api_url = KALSHI_API_BASE.to_owned() + "/markets";
    let limit: usize = 1000;
    let mut cursor: Option<String> = None;
    let mut all_market_data: Vec<MarketFull> = Vec::new();
    loop {
        let response = client
            .get(&api_url)
            .bearer_auth(&token)
            .query(&[("limit", limit)])
            .query(&[("cursor", cursor.clone())])
            .send()
            .unwrap()
            .error_for_status()
            .unwrap_or_else(|e| panic!("Kalshi: Query failed: {:?}", e))
            .json::<BulkMarketResponse>()
            .unwrap();
        let market_data: Vec<MarketFull> = response
            .markets
            .into_iter()
            .map(|market| get_extended_data(market))
            .collect();
        all_market_data.extend(market_data);
        if response.cursor.len() > 1 {
            cursor = Some(response.cursor);
        } else {
            break;
        }
    }
    all_market_data
        .into_iter()
        .map(|m| {
            TryInto::<Option<MarketForDB>>::try_into(m)
                .unwrap()
                .unwrap()
        })
        .collect()
}
