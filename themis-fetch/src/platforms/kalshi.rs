use super::*;
use regex::Regex;
use reqwest::blocking::Client;

const KALSHI_API_BASE: &str = "https://demo-api.kalshi.co/trade-api/v2";
const KALSHI_SITE_BASE: &str = "https://kalshi.com/markets/";

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
    event_ticker: String,
    //market_type: String,
    title: String,
    //open_time: String,
    //close_time: String,
    status: String,
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
        fn get_url(m: &MarketFull) -> String {
            let re = Regex::new(r"^(\w+)-").unwrap();
            KALSHI_SITE_BASE.to_owned()
                + &re.captures(&m.market.event_ticker).unwrap()[1].to_lowercase()
                + "/#"
                + &m.market.event_ticker.to_lowercase()
        }

        if self.market.status == "finalized" {
            Ok(Some(MarketForDB {
                title: self.market.title.clone(),
                platform: Platform::Kalshi,
                platform_id: self.market.ticker.clone(),
                url: get_url(&self), // TODO
            }))
        } else {
            Ok(None)
        }
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

pub fn get_market_by_id(id: &String) -> Vec<MarketForDB> {
    let client = get_default_client();
    let token = get_login_token(&client);
    let api_url = KALSHI_API_BASE.to_owned() + "/markets/";
    let response = client
        .get(api_url.clone() + id)
        .bearer_auth(&token)
        .send()
        .unwrap()
        .json::<SingleMarketResponse>()
        .unwrap();
    let market_data = get_extended_data(response.market);
    Vec::from([TryInto::<Option<MarketForDB>>::try_into(market_data)
        .expect("Error processing market")
        .expect("Market is not resolved.")])
}

pub fn get_markets_all() -> Vec<MarketForDB> {
    let client = get_default_client();
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
