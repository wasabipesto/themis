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

#[derive(Deserialize, Debug)]
struct EventInfo {
    ts: i64,
    //volume: u32,
    //yes_ask: u32,
    //yes_bid: u32,
    yes_price: f32,
    //open_interest: u32,
}

#[derive(Deserialize, Debug)]
struct BulkEventResponse {
    history: Vec<EventInfo>,
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
            prob_time_weighted: self.prob_time_weighted()?,
            resolution: self.resolution()?,
        })
    }
}

fn is_valid(market: &MarketInfo) -> bool {
    market.status == "finalized" && market.market_type == "binary"
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

fn get_prob_updates(mut events: Vec<EventInfo>) -> Result<Vec<ProbUpdate>, MarketConvertError> {
    let mut result = Vec::new();
    let mut prev_price = 0.0;
    events.sort_unstable_by_key(|b| b.ts);
    for event in events {
        if event.yes_price != prev_price {
            if let Ok(time) = get_datetime_from_secs(event.ts) {
                result.push(ProbUpdate {
                    time,
                    prob: event.yes_price / 100.0,
                })
            } else {
                return Err(MarketConvertError {
                    data: format!("{:?}", event),
                    message:
                        "Kalshi: Bet createdTime timestamp could not be converted into DateTime"
                            .to_string(),
                });
            }
            prev_price = event.yes_price;
        }
    }

    Ok(result)
}

async fn get_extended_data(
    client: &ClientWithMiddleware,
    token: &String,
    market: &MarketInfo,
) -> Result<MarketFull, MarketConvertError> {
    let api_url = KALSHI_API_BASE.to_owned() + "/markets/" + &market.ticker + "/history";
    let limit: usize = 1000;
    let mut cursor: Option<String> = None;
    let mut all_bet_data = Vec::new();
    loop {
        let response = client
            .get(&api_url)
            .bearer_auth(&token)
            .query(&[("limit", limit)])
            .query(&[("cursor", cursor.clone())])
            .query(&[("min_ts", 0)])
            .send()
            .await
            .expect("HTTP call failed to execute")
            .error_for_status()
            .unwrap_or_else(|e| panic!("Query failed: {:?}", e))
            .json::<BulkEventResponse>()
            .await
            .unwrap();
        all_bet_data.extend(response.history);
        if response.cursor.len() > 1 {
            cursor = Some(response.cursor);
        } else {
            break;
        }
    }
    Ok(MarketFull {
        market: market.clone(),
        events: get_prob_updates(all_bet_data)?,
    })
}

pub async fn get_markets_all(output_method: OutputMethod, verbose: bool) {
    println!("Kalshi: Processing started...");
    let client = get_reqwest_client_ratelimited(KALSHI_RATELIMIT);
    let token = get_login_token(&client).await;
    let api_url = KALSHI_API_BASE.to_owned() + "/markets";
    if verbose {
        println!("Kalshi: Connecting to API at {}", api_url)
    }
    let limit: usize = 1000;
    let mut cursor: Option<String> = None;
    loop {
        if verbose {
            println!("Kalshi: Getting markets starting at {:?}...", cursor)
        }
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
        if verbose {
            println!("Kalshi: Processing {} markets...", response.markets.len())
        }
        let market_data_futures: Vec<_> = response
            .markets
            .iter()
            .filter(|market| is_valid(market))
            .map(|market| get_extended_data(&client, &token, market))
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
        if verbose {
            println!(
                "Kalshi: Saving {} processed markets to {:?}...",
                market_data.len(),
                output_method
            )
        }
        save_markets(market_data, output_method);
        if response.cursor.len() > 1 {
            cursor = Some(response.cursor);
        } else {
            break;
        }
    }
}

pub async fn get_market_by_id(id: &String, output_method: OutputMethod, verbose: bool) {
    let client = get_reqwest_client_ratelimited(KALSHI_RATELIMIT);
    let token = get_login_token(&client).await;
    let api_url = KALSHI_API_BASE.to_owned() + "/markets/";
    if verbose {
        println!("Kalshi: Connecting to API at {}", api_url)
    }
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
        println!("Kalshi: Market is not valid for processing, this may fail.")
    }
    let market_data = get_extended_data(&client, &token, &market_single)
        .await
        .expect("Error getting extended market data")
        .try_into()
        .expect("ErError converting market into standard fields");
    if verbose {
        println!("Kalshi: Saving processed market to {:?}...", output_method)
    }
    save_markets(Vec::from([market_data]), output_method);
}
