//! Tools to download and process markets from the Polymarket API.

use super::*;

//const POLYMARKET_GAMMA_API_BASE: &str = "https://gamma-api.polymarket.com/query";
const POLYMARKET_CLOB_API_BASE: &str = "https://clob.polymarket.com";
const POLYMARKET_SITE_BASE: &str = "https://polymarket.com";
const POLYMARKET_RATELIMIT: usize = 100;
//const POLYMARKET_EPSILON: f32 = 0.0001;

/// (Indirect) API response with standard market info.
#[allow(non_snake_case)]
#[derive(Deserialize, Debug, Clone)]
struct MarketInfo {
    condition_id: String,
    question: String,
    market_slug: String,
    //active: bool,
    closed: bool,
    end_date_iso: Option<DateTime<Utc>>,
    //categories: Vec<String>,
    //parent_categories: Vec<String>,
    tokens: Vec<TokenData>,
}

#[derive(Deserialize, Debug, Clone)]
struct TokenData {
    token_id: String,
    //outcome: String,
    winner: bool,
}

#[derive(Deserialize, Debug, Clone)]
struct CLOBResponse {
    next_cursor: String,
    data: Vec<MarketInfo>,
}

/// API response with market history point.
#[derive(Deserialize, Debug, Clone)]
struct PricesHistoryPoint {
    t: i64,
    p: f32,
}

#[derive(Deserialize, Debug, Clone)]
struct PricesHistoryResponse {
    history: Vec<PricesHistoryPoint>,
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
        self.market.question.to_owned()
    }
    fn platform(&self) -> String {
        "polymarket".to_string()
    }
    fn platform_id(&self) -> String {
        self.market.condition_id.to_owned()
    }
    fn url(&self) -> String {
        POLYMARKET_SITE_BASE.to_owned() + "/event/" + &self.market.market_slug
    }
    fn open_dt(&self) -> Result<DateTime<Utc>, MarketConvertError> {
        if let Some(first_event) = self.events().first() {
            Ok(first_event.time)
        } else {
            Err(MarketConvertError {
                data: self.debug(),
                message: format!("Polymarket: No events in event list (cannot get market bounds)."),
                level: 3,
            })
        }
    }
    fn close_dt(&self) -> Result<DateTime<Utc>, MarketConvertError> {
        if let Some(close_dt) = self.market.end_date_iso {
            Ok(close_dt)
        } else {
            Err(MarketConvertError {
                data: self.debug(),
                message: format!("Polymarket: Market field end_date_iso is empty."),
                level: 0,
            })
        }
    }
    fn volume_usd(&self) -> f32 {
        //self.market.volume
        0.0 // TODO
    }
    fn num_traders(&self) -> i32 {
        0 // TODO
    }
    fn category(&self) -> String {
        "None".to_string() // TODO
    }
    fn events(&self) -> Vec<ProbUpdate> {
        self.events.to_owned()
    }
    fn resolution(&self) -> Result<f32, MarketConvertError> {
        match (self.market.tokens.first(), self.market.tokens.last()) {
            (Some(token_1), Some(token_2)) => match (token_1.winner, token_2.winner) {
                (true, false) => Ok(1.0),
                (false, true) => Ok(0.0),
                (true, true) => Err(MarketConvertError {
                    data: self.debug(),
                    message: format!("Polymarket: Both tokens are winners."),
                    level: 1,
                }),
                (false, false) => Err(MarketConvertError {
                    data: self.debug(),
                    message: format!("Polymarket: Neither token is a winner."),
                    level: 1,
                }),
            },
            _ => Err(MarketConvertError {
                data: self.debug(),
                message: format!("Polymarket: Market field `tokens` has less than two values."),
                level: 3,
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
            prob_time_weighted: self.prob_time_weighted()?,
            resolution: self.resolution()?,
        })
    }
}

/// Test if a market is suitable for analysis.
fn is_valid(market: &MarketInfo) -> bool {
    market.closed == true && market.tokens.len() == 2 && market.end_date_iso < Some(Utc::now())
}

/// Convert API events into standard events.
fn get_prob_updates(
    mut points: Vec<PricesHistoryPoint>,
) -> Result<Vec<ProbUpdate>, MarketConvertError> {
    let mut result: Vec<ProbUpdate> = Vec::new();
    points.sort_unstable_by_key(|point| point.t);
    for point in points {
        if let Some(last_point) = result.last() {
            if last_point.prob == point.p {
                // skip adding to the list if the prob is the same
                continue;
            }
        }
        if let Ok(time) = get_datetime_from_secs(point.t) {
            result.push(ProbUpdate {
                time,
                prob: point.p,
            });
        } else {
            return Err(MarketConvertError {
                data: format!("{:?}", point),
                message: format!(
                    "Polymarket: History event timestamp could not be converted into DateTime"
                ),
                level: 3,
            });
        }
    }

    Ok(result)
}

/// Download full market history and store events in the container.
async fn get_extended_data(
    client: &ClientWithMiddleware,
    market: &MarketInfo,
) -> Result<MarketFull, MarketConvertError> {
    let api_url = POLYMARKET_CLOB_API_BASE.to_owned() + "/prices-history";
    let clob_id = match market.tokens.first() {
        Some(token) => Ok(token.token_id.to_owned()),
        None => Err(MarketConvertError {
            data: format!("{:?}", market),
            message: format!("Polymarket: Market field `tokens` is empty."),
            level: 3,
        }),
    }?;
    let mut events = Vec::new();
    for i in 0..=5 {
        // get fidelity window
        let fidelity = match i {
            0 => 10,
            1 => 60,
            2 => 180,
            3 => 360,
            4 => 1200,
            5 => 3600,
            _ => 999999,
        };
        // make the request
        let response: PricesHistoryResponse = send_request(
            client
                .get(&api_url)
                .query(&[("interval", "all")])
                .query(&[("market", clob_id.to_owned())])
                .query(&[("fidelity", fidelity)]),
        )
        .await?;

        // break out if we get data
        if response.history.len() > 0 {
            events.extend(response.history);
            break;
        } else if i >= 5 {
            return Err(MarketConvertError {
                data: format!("{:?}", market),
                message: format!("Polymarket: CLOB returned empty list for price history, even at fidelity = {fidelity}."),
                level: 2,
            });
        }
    }

    Ok(MarketFull {
        market: market.clone(),
        events: get_prob_updates(events)?,
    })
}

/// Download, process and store all valid markets from the platform.
pub async fn get_markets_all(output_method: OutputMethod, verbose: bool) {
    println!("Polymarket: Processing started...");
    let client = get_reqwest_client_ratelimited(POLYMARKET_RATELIMIT, None);
    let api_url = POLYMARKET_CLOB_API_BASE.to_owned() + "/markets";
    if verbose {
        println!("Polymarket: Connecting to API at {}", api_url)
    }
    let limit: usize = 100;
    let mut cursor: Option<String> = None;
    loop {
        if verbose {
            println!("Polymarket: Getting markets starting at {:?}...", cursor)
        }
        let response: CLOBResponse =
            send_request(client.get(&api_url).query(&[("next_cursor", cursor)]))
                .await
                .expect("Polymarket: API query error.");
        if verbose {
            println!("Polymarket: Processing {} markets...", response.data.len())
        }
        let market_data_futures: Vec<_> = response
            .data
            .iter()
            .filter(|market| is_valid(market))
            .map(|market| get_extended_data(&client, market))
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
                "Polymarket: Saving {} processed markets to {:?}...",
                market_data.len(),
                output_method
            )
        }
        save_markets(market_data, output_method);
        if response.data.len() == limit {
            cursor = Some(response.next_cursor);
        } else {
            break;
        }
    }
    println!("Polymarket: Processing complete.");
}

/// Download, process and store one market from the platform.
pub async fn get_market_by_id(id: &String, output_method: OutputMethod, verbose: bool) {
    let client = get_reqwest_client_ratelimited(POLYMARKET_RATELIMIT, None);
    let api_url = POLYMARKET_CLOB_API_BASE.to_owned() + "/markets/" + id;
    if verbose {
        println!("Polymarket: Connecting to API at {}", api_url)
    }
    let single_market: MarketInfo = send_request(client.get(&api_url))
        .await
        .expect("Polymarket: API query error.");
    if !is_valid(&single_market) {
        println!("Polymarket: Market is not valid for processing, this may fail.")
    }
    let market_data = get_extended_data(&client, &single_market)
        .await
        .expect("Error getting extended market data")
        .try_into()
        .expect("Error converting market into standard fields");
    if verbose {
        println!(
            "Polymarket: Saving processed market to {:?}...",
            output_method
        )
    }
    save_markets(Vec::from([market_data]), output_method);
}
