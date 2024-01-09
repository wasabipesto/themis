//! Tools to download and process markets from the Metaculus API.

use super::*;

const METACULUS_API_BASE: &str = "https://www.metaculus.com/api2";
const METACULUS_SITE_BASE: &str = "https://www.metaculus.com";
const METACULUS_USD_PER_FORECAST: f32 = 0.05;
const METACULUS_RATELIMIT: usize = 100;

#[derive(Deserialize, Debug, Clone)]
struct BulkMarketResponse {
    //next: String,
    results: Vec<MarketInfo>,
}

#[derive(Deserialize, Debug, Clone)]
struct MarketInfo {
    id: u32,
    title: String,
    //group_label: String,
    active_state: String,
    page_url: String,
    number_of_forecasters: i32,
    prediction_count: u32,
    created_time: DateTime<Utc>,
    //publish_time: DateTime<Utc>,
    effected_close_time: Option<DateTime<Utc>>,
    possibilities: MarketTypePossibilities,
    community_prediction: PredictionHistory,
    resolution: Option<f32>,
}

#[derive(Deserialize, Debug, Clone)]
struct MarketTypePossibilities {
    r#type: String,
}

#[derive(Deserialize, Debug, Clone)]
struct PredictionHistory {
    history: Vec<PredictionPoint>,
}

#[derive(Deserialize, Debug, Clone)]
struct PredictionPoint {
    t: f32,
    x2: PredictionPointX2,
}

#[derive(Deserialize, Debug, Clone)]
struct PredictionPointX2 {
    avg: f32,
    //var: f32,
    //weighted_avg: f32,
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
        "metaculus".to_string()
    }
    fn platform_id(&self) -> String {
        self.market.id.to_string()
    }
    fn url(&self) -> String {
        METACULUS_SITE_BASE.to_owned() + &self.market.page_url
    }
    fn open_dt(&self) -> Result<DateTime<Utc>, MarketConvertError> {
        Ok(self.market.created_time)
    }
    fn close_dt(&self) -> Result<DateTime<Utc>, MarketConvertError> {
        if let Some(close_time) = self.market.effected_close_time {
            Ok(close_time)
        } else {
            Err(MarketConvertError {
                data: self.debug(),
                message: format!("Metaculus: effected_close_time is missing from closed market"),
            })
        }
    }
    fn volume_usd(&self) -> f32 {
        self.market.prediction_count as f32 * METACULUS_USD_PER_FORECAST
    }
    fn num_traders(&self) -> i32 {
        self.market.number_of_forecasters
    }
    fn events(&self) -> Vec<ProbUpdate> {
        self.events.to_owned()
    }
    fn resolution(&self) -> Result<f32, MarketConvertError> {
        if let Some(resolution) = self.market.resolution {
            if 0.0 <= resolution && resolution <= 1.0 {
                Ok(resolution)
            } else {
                Err(MarketConvertError {
                    data: self.debug(),
                    message: format!("Metaculus: Market resolution value out of bounds"),
                })
            }
        } else {
            Err(MarketConvertError {
                data: self.debug(),
                message: format!("Metaculus: Market resolution value is null"),
            })
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
    market.active_state == "RESOLVED"
        && market.possibilities.r#type == "binary"
        && market.resolution >= Some(0.0)
}

/// Convert API events into standard events.
fn get_prob_updates(
    mut points: Vec<PredictionPoint>,
) -> Result<Vec<ProbUpdate>, MarketConvertError> {
    let mut result = Vec::new();
    points.sort_unstable_by_key(|point| point.t as i64);
    for point in points {
        if let Ok(time) = get_datetime_from_secs(point.t as i64) {
            result.push(ProbUpdate {
                time,
                prob: point.x2.avg,
            });
        } else {
            return Err(MarketConvertError {
                data: format!("{:?}", point),
                message: "Metaculus: History event timestamp could not be converted into DateTime"
                    .to_string(),
            });
        }
    }

    Ok(result)
}

/// Download full market history and store events in the container.
fn get_extended_data(market: &MarketInfo) -> Result<MarketFull, MarketConvertError> {
    Ok(MarketFull {
        market: market.clone(),
        events: get_prob_updates(market.community_prediction.history.clone())?,
    })
}

/// Download, process and store all valid markets from the platform.
pub async fn get_markets_all(output_method: OutputMethod, verbose: bool) {
    println!("Metaculus: Processing started...");
    let client = get_reqwest_client_ratelimited(METACULUS_RATELIMIT);
    let api_url = METACULUS_API_BASE.to_owned() + "/questions";
    if verbose {
        println!("Metaculus: Connecting to API at {}", api_url)
    }
    let limit = 100;
    let mut offset: usize = 0;
    loop {
        if verbose {
            println!("Metaculus: Getting markets starting at {:?}...", offset)
        }
        let market_response = client
            .get(&api_url)
            .query(&[("limit", limit)])
            .query(&[("offset", offset)])
            .send()
            .await
            .expect("HTTP call failed to execute")
            .json::<BulkMarketResponse>()
            .await
            .expect("Market failed to deserialize");
        if verbose {
            println!(
                "Metaculus: Processing {} markets...",
                market_response.results.len()
            )
        }
        let market_data: Vec<MarketStandard> = market_response
            .results
            .iter()
            .filter(|market| is_valid(market))
            .map(|market| match get_extended_data(market) {
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
                "Metaculus: Saving {} processed markets to {:?}...",
                market_data.len(),
                output_method
            )
        }
        save_markets(market_data, output_method);
        if market_response.results.len() == limit {
            offset += limit;
        } else {
            break;
        }
    }
}

/// Download, process and store one market from the platform.
pub async fn get_market_by_id(id: &str, output_method: OutputMethod, verbose: bool) {
    let client = get_reqwest_client_ratelimited(METACULUS_RATELIMIT);
    let api_url = METACULUS_API_BASE.to_owned() + "/questions/" + id;
    if verbose {
        println!("Metaculus: Connecting to API at {}", api_url)
    }
    let market_single = client
        .get(&api_url)
        .send()
        .await
        .expect("HTTP call failed to execute")
        .json::<MarketInfo>()
        .await
        .expect("Market failed to deserialize");
    if !is_valid(&market_single) {
        println!("Metaculus: Market is not valid for processing, this may fail.")
    }
    let market_data = get_extended_data(&market_single)
        .expect("Error getting extended market data")
        .try_into()
        .expect("Error converting market into standard fields");
    if verbose {
        println!(
            "Metaculus: Saving processed market to {:?}...",
            output_method
        )
    }
    save_markets(Vec::from([market_data]), output_method);
}
