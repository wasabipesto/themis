use super::*;
use regex::Regex;

const KALSHI_API_BASE: &str = "https://demo-api.kalshi.co/trade-api/v2";
const KALSHI_SITE_BASE: &str = "https://kalshi.com/markets/";
const KALSHI_EXCHANGE_RATE: f32 = 100.0;
const KALSHI_RATELIMIT: usize = 10;

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
    market_type: String,
    title: String,
    open_time: DateTime<Utc>,
    close_time: DateTime<Utc>,
    status: String,
    volume: f32,
    result: String,
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
    events: Vec<ProbUpdate>,
}

impl MarketStandardizer for MarketFull {
    fn debug(&self) -> String {
        format!("{:?}", self)
    }
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
    fn open_dt(&self) -> Result<DateTime<Utc>, MarketConvertError> {
        Ok(self.market.open_time)
    }
    fn close_dt(&self) -> Result<DateTime<Utc>, MarketConvertError> {
        Ok(self.market.close_time)
    }
    fn volume_usd(&self) -> f32 {
        self.market.volume / KALSHI_EXCHANGE_RATE
    }
    fn events(&self) -> Vec<ProbUpdate> {
        self.events.to_owned()
    }
    fn resolution(&self) -> Result<f32, MarketConvertError> {
        match self.market.result.as_str() {
            "yes" => Ok(1.0),
            "no" => Ok(0.0),
            _ => Err(MarketConvertError {
                data: self.debug(),
                message: "Market resolved to something besides YES or NO".to_string(),
            }),
        }
    }
}

impl TryInto<MarketStandard> for MarketFull {
    type Error = MarketConvertError;
    fn try_into(self) -> Result<MarketStandard, MarketConvertError> {
        Ok(MarketStandard {
            title: self.title(),
            platform: self.platform(),
            platform_id: self.platform_id(),
            url: self.url(),
            open_days: self.open_days()?,
            volume_usd: self.volume_usd(),
            prob_at_midpoint: self.prob_at_percent(0.5)?,
            prob_at_close: self.prob_at_percent(1.0)?,
            resolution: self.resolution()?,
        })
    }
}

fn is_valid(market: &MarketInfo) -> bool {
    market.status == "finalized" && market.market_type == "binary"
}

async fn get_extended_data(
    _client: &ClientWithMiddleware,
    market: &MarketInfo,
) -> Result<MarketFull, MarketConvertError> {
    Ok(MarketFull {
        market: market.clone(),
        events: Vec::from([ProbUpdate {
            time: market.open_time,
            prob: 0.5,
        }]),
    })
}

async fn get_login_token(client: &ClientWithMiddleware) -> String {
    let api_url = KALSHI_API_BASE.to_owned() + "/login";
    let credentials = LoginCredentials {
        email: var("KALSHI_USERNAME")
            .expect("Required environment variable KALSHI_USERNAME not set."),
        password: var("KALSHI_PASSWORD")
            .expect("Required environment variable KALSHI_PASSWORD not set."),
    };
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
    response.token
}

pub async fn get_markets_all() -> Vec<MarketStandard> {
    let client = get_reqwest_client_ratelimited(KALSHI_RATELIMIT);
    let token = get_login_token(&client).await;
    let api_url = KALSHI_API_BASE.to_owned() + "/markets";
    let limit: usize = 1000;
    let mut cursor: Option<String> = None;
    let mut all_market_data = Vec::new();
    loop {
        let response = client
            .get(&api_url)
            .bearer_auth(&token)
            .query(&[("limit", limit)])
            .query(&[("cursor", cursor.clone())])
            .send()
            .await
            .expect("HTTP call failed to execute")
            .error_for_status()
            .unwrap_or_else(|e| panic!("Query failed: {:?}", e))
            .json::<BulkMarketResponse>()
            .await
            .unwrap();
        let market_data_futures: Vec<_> = response
            .markets
            .iter()
            .filter(|market| is_valid(market))
            .map(|market| get_extended_data(&client, market))
            .collect();
        let market_data: Vec<MarketStandard> = join_all(market_data_futures)
            .await
            .into_iter()
            .map(|i| i.expect("Error getting extended market data"))
            .map(|market| {
                market
                    .try_into()
                    .expect("Error converting market into standard fields")
            })
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
        .map(|market| {
            market
                .try_into()
                .expect("Error converting market into standard fields.")
        })
        .collect()
}

pub async fn get_market_by_id(id: &String) -> Vec<MarketStandard> {
    let client = get_reqwest_client_ratelimited(KALSHI_RATELIMIT);
    let token = get_login_token(&client).await;
    let api_url = KALSHI_API_BASE.to_owned() + "/markets/";
    let market_single = client
        .get(api_url.clone() + id)
        .bearer_auth(&token)
        .send()
        .await
        .expect("HTTP call failed to execute")
        .json::<SingleMarketResponse>()
        .await
        .expect("Market failed to deserialize")
        .market;
    if !is_valid(&market_single) {
        println!("Market is not valid for processing, this may fail.")
    }
    let market_data = get_extended_data(&client, &market_single)
        .await
        .expect("Error getting extended market data")
        .try_into()
        .expect("ErError converting market into standard fields");
    Vec::from([market_data])
}
