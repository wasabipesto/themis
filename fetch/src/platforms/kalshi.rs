//! Tools to download and process markets from the Kalshi API.

use super::*;
use regex::Regex;

const KALSHI_API_BASE: &str = "https://trading-api.kalshi.com/trade-api/v2";
const KALSHI_SITE_BASE: &str = "https://kalshi.com/markets/";
const KALSHI_EXCHANGE_RATE: f32 = 100.0;
const KALSHI_RATELIMIT: usize = 10;

/// Holds API login credentials to be submitted.
#[derive(Serialize, Debug)]
struct LoginCredentials {
    email: String,
    password: String,
}

/// API response after requesting an authorization token.
#[derive(Deserialize, Debug)]
struct LoginResponse {
    token: String,
}

/// (Indirect) API response with standard market info.
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

/// API response after requesting a single market from `/market`.
#[derive(Deserialize, Debug)]
struct SingleMarketResponse {
    market: MarketInfo,
}

/// API response after requesting multiple markets from `/markets`.
#[derive(Deserialize, Debug)]
struct BulkMarketResponse {
    markets: Vec<MarketInfo>,
    cursor: String,
}

/// (Indirect) API response with standard event info.
#[derive(Deserialize, Debug)]
struct EventInfo {
    ts: i64,
    //volume: u32,
    //yes_ask: u32,
    //yes_bid: u32,
    yes_price: f32,
    //open_interest: u32,
}

/// API response after requesting market events from `/history`.
#[derive(Deserialize, Debug)]
struct BulkEventResponse {
    history: Vec<EventInfo>,
    cursor: String,
}

/// Container for market data and events, used to hold data for conversion.
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
    fn num_traders(&self) -> i32 {
        0 // TODO
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

/// Standard conversion setup (would move this up to `platforms` if I could).
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
            num_traders: self.num_traders(),
            prob_at_midpoint: self.prob_at_percent(0.5)?,
            prob_at_close: self.prob_at_percent(1.0)?,
            prob_time_weighted: self.prob_time_weighted()?,
            resolution: self.resolution()?,
        })
    }
}

/// Test if a market is suitable for analysis.
fn is_valid(market: &MarketInfo) -> bool {
    market.status == "finalized" && market.market_type == "binary"
}

/// Request an authorization token from email & password.
async fn get_login_token(client_opt: Option<ClientWithMiddleware>) -> String {
    let client = match client_opt {
        Some(client) => client,
        None => get_reqwest_client_ratelimited(KALSHI_RATELIMIT),
    };

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

/// Convert API events into standard events.
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

/// Download full market history and store events in the container.
async fn get_extended_data(
    client: &ClientWithMiddleware,
    token: &String,
    market: &MarketInfo,
) -> Result<MarketFull, MarketConvertError> {
    let ticker_urlencoded = Regex::new(r"%").unwrap().replace_all(&market.ticker, "%25");
    let api_url = KALSHI_API_BASE.to_owned() + "/markets/" + &ticker_urlencoded + "/history";
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

/// Download, process and store all valid markets from the platform.
pub async fn get_markets_all(output_method: OutputMethod, verbose: bool) {
    println!("Kalshi: Processing started...");
    let client = get_reqwest_client_ratelimited(KALSHI_RATELIMIT);
    let token = get_login_token(Some(client.clone())).await;
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
            .map(|market_downloaded_result| match market_downloaded_result {
                Ok(market_downloaded) => {
                    // market downloaded successfully
                    match market_downloaded.try_into() {
                        // market processed successfully
                        Ok(market_converted) => Some(market_converted),
                        // market failed processing
                        Err(e) => {
                            eprintln!("Error converting market into standard fields: {e}");
                            None
                        }
                    }
                }
                Err(e) => {
                    // market failed downloadng
                    eprintln!("Error downloading full market data: {e}");
                    return None;
                }
            })
            .flatten()
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

/// Download, process and store one market from the platform.
pub async fn get_market_by_id(id: &String, output_method: OutputMethod, verbose: bool) {
    let client = get_reqwest_client_ratelimited(KALSHI_RATELIMIT);
    let token = get_login_token(Some(client.clone())).await;
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

/// Get a new token if the old one expired.
struct FetchTokenMiddleware;

#[async_trait::async_trait]
impl Chainer for FetchTokenMiddleware {
    type State = ();

    async fn chain(
        &self,
        result: Result<reqwest::Response, Error>,
        _state: &mut Self::State,
        request: &mut reqwest::Request,
    ) -> Result<Option<reqwest::Response>, Error> {
        let response = result?;
        if response.status() != StatusCode::UNAUTHORIZED {
            return Ok(Some(response));
        };
        request.headers_mut().insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", get_login_token(None).await))
                .expect("invalid header value"),
        );
        Ok(None)
    }
}
