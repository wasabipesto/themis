use super::*;
use regex::Regex;

const KALSHI_API_BASE: &str = "https://demo-api.kalshi.co/trade-api/v2";
const KALSHI_SITE_BASE: &str = "https://kalshi.com/markets/";
const KALSHI_EXCHANGE_RATE: f32 = 100.0;

#[derive(Serialize, Debug)]
struct LoginCredentials {
    email: String,
    password: String,
}

#[derive(Deserialize, Debug)]
struct LoginResponse {
    token: String,
}

#[derive(Deserialize, Debug, Clone)]
struct MarketInfo {
    ticker: String,
    event_ticker: String,
    //market_type: String,
    title: String,
    open_time: DateTime<Utc>,
    close_time: DateTime<Utc>,
    status: String,
    volume: f32,
}

impl MarketInfoDetails for MarketInfo {
    fn is_valid(&self) -> bool {
        self.status == "finalized"
    }
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

impl MarketFullDetails for MarketFull {
    fn title(&self) -> String {
        self.market.title.to_owned()
    }
    fn platform(&self) -> String {
        "kalshi".to_string()
    }
    fn platform_id(&self) -> String {
        self.market.ticker.to_owned()
    }
    fn url(&self) -> String {
        let ticker_regex = Regex::new(r"^(\w+)-").unwrap();
        let ticker_prefix =
            if let Some(ticker_regex_result) = ticker_regex.find(&self.market.event_ticker) {
                ticker_regex_result.as_str()
            } else {
                // Some tickers do not have a prefix, just use the market ticker for both
                &self.market.event_ticker
            };
        KALSHI_SITE_BASE.to_owned()
            + &ticker_prefix.to_lowercase()
            + "/#"
            + &self.market.event_ticker.to_lowercase()
    }
    fn open_date(&self) -> Result<DateTime<Utc>, MarketConvertError> {
        Ok(self.market.open_time)
    }
    fn close_date(&self) -> Result<DateTime<Utc>, MarketConvertError> {
        Ok(self.market.close_time)
    }
    fn volume_usd(&self) -> f32 {
        self.market.volume / KALSHI_EXCHANGE_RATE
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
            volume_usd: self.volume_usd(),
        })
    }
}

async fn get_extended_data(client: &ClientWithMiddleware, market: &MarketInfo) -> MarketFull {
    MarketFull {
        market: market.clone(),
    }
}

async fn get_login_token(client: &ClientWithMiddleware) -> String {
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
        .await
        .unwrap()
        .error_for_status()
        .unwrap_or_else(|e| panic!("Kalshi: Login failed: {:?}", e))
        .json::<LoginResponse>()
        .await
        .unwrap();
    //println!("Kalshi: Logged in with token: {}", &response.token);
    response.token
}

pub async fn get_market_by_id(id: &String) -> Vec<MarketForDB> {
    let client = get_default_client();
    let token = get_login_token(&client).await;
    let api_url = KALSHI_API_BASE.to_owned() + "/markets/";
    let response = client
        .get(api_url.clone() + id)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json::<SingleMarketResponse>()
        .await
        .unwrap();
    let market_data = get_extended_data(&client, &response.market).await;
    Vec::from([market_data.try_into().expect("Error processing market")])
}

pub async fn get_markets_all() -> Vec<MarketForDB> {
    let client = get_default_client();
    let token = get_login_token(&client).await;
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
            .await
            .unwrap()
            .error_for_status()
            .unwrap_or_else(|e| panic!("Kalshi: Query failed: {:?}", e))
            .json::<BulkMarketResponse>()
            .await
            .unwrap();
        let market_data_futures: Vec<_> = response
            .markets
            .iter()
            .filter(|market| market.is_valid())
            .map(|market| get_extended_data(&client, market))
            .collect();
        all_market_data.extend(join_all(market_data_futures).await);
        if response.cursor.len() > 1 {
            cursor = Some(response.cursor);
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
