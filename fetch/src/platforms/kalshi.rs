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
    category: String,
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

/// (Indirect) API response with candlestick price info.
/// These may be empty if there was no trade during the period.
#[derive(Deserialize, Debug)]
struct CandlestickPrice {
    // Last traded YES contract price on the market during the candlestick period.
    //close: Option<u32>,
    // Highest traded YES contract price on the market during the candlestick period.
    //high: Option<u32>,
    // Lowest traded YES contract price on the market during the candlestick period.
    //low: Option<u32>,
    /// Mean traded YES contract price on the market during the candlestick period.
    mean: Option<u32>,
    // First traded YES contract price on the market during the candlestick period.
    //open: Option<u32>,
    // Last traded YES contract price on the market before the candlestick period.
    //previous: Option<u32>,
}

/// (Indirect) API response with candlestick info.
#[derive(Deserialize, Debug)]
struct CandlestickInfo {
    #[serde(with = "ts_seconds")]
    end_period_ts: DateTime<Utc>,
    //volume: u32,
    //yes_ask: u32,
    //yes_bid: u32,
    price: CandlestickPrice,
    //open_interest: u32,
}

/// API response after requesting market events from `/history`.
#[derive(Deserialize, Debug)]
struct BulkCandlestickResponse {
    candlesticks: Vec<CandlestickInfo>,
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
            if let Some(ticker_regex_result) = ticker_regex.captures(&self.market.event_ticker) {
                ticker_regex_result
                    .get(1)
                    .expect("failed to get first regex match even though regex reported a match")
                    .as_str()
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
    fn category(&self) -> String {
        match self.market.category.as_str() {
            "COVID-19" => "Science".to_string(),
            "Climate and Weather" => "Climate".to_string(),
            "Companies" => "Economics".to_string(),
            "Economics" => "Economics".to_string(),
            "Entertainment" => "Culture".to_string(),
            "Financials" => "Economics".to_string(),
            "Health" => "Science".to_string(),
            "Politics" => "Politics".to_string(),
            "Science & Technology" => "Science".to_string(),
            "Science and Technology" => "Science".to_string(),
            "Transportation" => "Politics".to_string(),
            _ => "None".to_string(),
        }
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
                message: "Kalshi: Market resolved to something besides YES or NO".to_string(),
                level: 0,
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
            open_dt: self.open_dt()?,
            close_dt: self.close_dt()?,
            open_days: self.open_days()?,
            volume_usd: self.volume_usd(),
            num_traders: self.num_traders(),
            category: self.category(),
            prob_at_midpoint: self.prob_at_percent(0.5)?,
            prob_at_close: self.prob_at_percent(1.0)?,
            prob_at_pct: self.prob_at_pct_list()?,
            prob_time_avg: self.prob_time_avg_whole()?,
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
        None => get_reqwest_client_ratelimited(KALSHI_RATELIMIT, None),
    };

    let api_url = KALSHI_API_BASE.to_owned() + "/login";
    let credentials = LoginCredentials {
        email: var("KALSHI_USERNAME")
            .expect("Required environment variable KALSHI_USERNAME not set."),
        password: var("KALSHI_PASSWORD")
            .expect("Required environment variable KALSHI_PASSWORD not set."),
    };
    let response: LoginResponse = send_request(client.post(api_url).json(&credentials))
        .await
        .expect("Kalshi: Login failed.");

    println!("Kalshi: Logged in with token {}", response.token);
    response.token
}

/// Convert API events into standard events.
fn get_prob_updates(events: Vec<CandlestickInfo>) -> Result<Vec<ProbUpdate>, MarketConvertError> {
    let mut result = Vec::new();
    //events.sort_unstable_by_key(|b| b.end_period_ts);
    for event in events {
        if let Some(mean_price) = event.price.mean {
            result.push(ProbUpdate {
                time: event.end_period_ts, // subtract half an hour?
                prob: mean_price as f32 / 100.0,
            });
        }
    }

    Ok(result)
}

/// Download full market history and store events in the container.
/// Uses the GetMarketCandlesticks endpoint:
/// https://trading-api.readme.io/reference/getmarketcandlesticks
/// Note that we only get the data aggregated into hourly buckets.
async fn get_extended_data(
    client: &ClientWithMiddleware,
    token: &String,
    market: &MarketInfo,
) -> Result<MarketFull, MarketConvertError> {
    // url-encode the ticker by replacing all "%" with "%25"
    let ticker_urlencoded = Regex::new(r"%").unwrap().replace_all(&market.ticker, "%25");
    // get the series ticker by taking the text up to the first "-"
    let series_ticker = Regex::new(r"-.*").unwrap().replace_all(&market.ticker, "");
    // url-encode it also
    let series_ticker_urlencoded = Regex::new(r"%").unwrap().replace_all(&series_ticker, "%25");
    // build url in the form of https://trading-api.kalshi.com/trade-api/v2/series/{series_ticker}/markets/{ticker}/candlesticks
    let api_url = KALSHI_API_BASE.to_owned()
        + "/series/"
        + &series_ticker_urlencoded
        + "/markets/"
        + &ticker_urlencoded
        + "/candlesticks";
    // you can query by minute, hour, or day - I chose hour
    // limit of 5000 data points per request (208 days)
    let query_period_interval = 60i64;
    let mut query_start_ts = market.open_time.timestamp();
    let mut query_end_ts = query_start_ts + query_period_interval * 5000;
    let mut all_bet_data = Vec::new();
    loop {
        let response: BulkCandlestickResponse = send_request(
            client
                .get(&api_url)
                .bearer_auth(token)
                .query(&[("period_interval", query_period_interval)])
                .query(&[("start_ts", query_start_ts)])
                .query(&[("end_ts", query_end_ts)]),
        )
        .await?;
        all_bet_data.extend(response.candlesticks);
        if query_end_ts <= market.close_time.timestamp() {
            query_start_ts = query_end_ts;
            query_end_ts = query_start_ts + query_period_interval * 5000;
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
    log_to_stdout("Kalshi: Processing started...");
    let client = get_reqwest_client_ratelimited(KALSHI_RATELIMIT, None);
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
        let response: BulkMarketResponse = send_request(
            client
                .get(&api_url)
                .bearer_auth(&token)
                .query(&[("limit", limit)])
                .query(&[("cursor", cursor.clone())]),
        )
        .await
        .expect("Kalshi: API query error.");
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
            .filter_map(|market_downloaded_result| match market_downloaded_result {
                Ok(market_downloaded) => {
                    // market downloaded successfully
                    match market_downloaded.try_into() {
                        // market processed successfully
                        Ok(market_converted) => Some(market_converted),
                        // market failed processing
                        Err(error) => {
                            eval_error(error, verbose);
                            None
                        }
                    }
                }
                Err(error) => {
                    // market failed downloadng
                    eval_error(error, verbose);
                    None
                }
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
    log_to_stdout("Kalshi: Processing complete.");
}

/// Download, process and store one market from the platform.
pub async fn get_market_by_id(id: &String, output_method: OutputMethod, verbose: bool) {
    let client = get_reqwest_client_ratelimited(KALSHI_RATELIMIT, None);
    let token = get_login_token(Some(client.clone())).await;
    let api_url = KALSHI_API_BASE.to_owned() + "/markets/";
    if verbose {
        println!("Kalshi: Connecting to API at {}", api_url)
    }
    let market_single: SingleMarketResponse =
        send_request(client.get(api_url.clone() + id).bearer_auth(&token))
            .await
            .expect("Kalshi: API query error.");
    if !is_valid(&market_single.market) {
        println!("Kalshi: Market is not valid for processing, this may fail.")
    }
    let market_data: MarketStandard = get_extended_data(&client, &token, &market_single.market)
        .await
        .expect("Error getting extended market data")
        .try_into()
        .expect("Error converting market into standard fields");
    if verbose {
        println!("Kalshi: Saving processed market to {:?}...", output_method)
    }
    save_markets(Vec::from([market_data]), output_method);
}

/// Get a new token if the old one expired.
#[allow(dead_code)]
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
