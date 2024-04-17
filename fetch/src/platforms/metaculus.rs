//! Tools to download and process markets from the Metaculus API.

use super::*;

const METACULUS_API_BASE: &str = "https://www.metaculus.com/api2";
const METACULUS_SITE_BASE: &str = "https://www.metaculus.com";
const METACULUS_USD_PER_FORECAST: f32 = 0.10;
const METACULUS_RATELIMIT: usize = 15;
const METACULUS_RATELIMIT_MS: u64 = 60_000;

#[derive(Deserialize, Debug, Clone)]
struct BulkMarketResponse {
    //next: String,
    results: Vec<MarketInfo>,
}

#[derive(Deserialize, Debug, Clone)]
struct MarketInfo {
    id: u32,
    title: String,
    active_state: String,
    page_url: String,
    number_of_forecasters: i32,
    prediction_count: u32,
    created_time: DateTime<Utc>,
    effected_close_time: Option<DateTime<Utc>>,
    possibilities: MarketTypePossibilities,
    community_prediction: PredictionHistory,
    resolution: Option<f32>,
}

#[derive(Deserialize, Debug, Clone)]
struct MarketInfoExtra {
    categories: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct MarketTypePossibilities {
    r#type: Option<String>,
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
    avg: Option<f32>,
    //var: f32,
    //weighted_avg: f32,
}

/// Container for market data and events, used to hold data for conversion.
#[derive(Debug)]
struct MarketFull {
    market: MarketInfo,
    market_extra: MarketInfoExtra,
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
                message: "Metaculus: effected_close_time is missing from closed market".to_string(),
                level: 3,
            })
        }
    }
    fn volume_usd(&self) -> f32 {
        self.market.prediction_count as f32 * METACULUS_USD_PER_FORECAST
    }
    fn num_traders(&self) -> i32 {
        self.market.number_of_forecasters
    }
    fn category(&self) -> String {
        for category in &self.market_extra.categories {
            match category.as_str() {
                "bio--bioengineering" => return "Science".to_string(),
                "bio--infectious-disease" => return "Science".to_string(),
                "bio--medicine" => return "Science".to_string(),
                "business" => return "Economics".to_string(),
                "category--scientific-discoveries" => return "Science".to_string(),
                "category--technological-advances" => return "Technology".to_string(),
                "comp-sci--ai-and-machinelearning" => return "AI".to_string(),
                "computing--ai" => return "AI".to_string(),
                "computing--blockchain" => return "Crypto".to_string(),
                "contests--cryptocurrency" => return "Crypto".to_string(),
                "economy" => return "Economics".to_string(),
                "elections--us--president" => return "Politics".to_string(),
                "environment--climate" => return "Climate".to_string(),
                "finance" => return "Economics".to_string(),
                "finance--cryptocurrencies" => return "Crypto".to_string(),
                "finance--market" => return "Economics".to_string(),
                "geopolitics" => return "Politics".to_string(),
                "geopolitics--armedconflict" => return "Politics".to_string(),
                "industry--space" => return "Science".to_string(),
                "industry--transportation" => return "Technology".to_string(),
                "phys-sci--astro-and-cosmo" => return "Science".to_string(),
                "politics" => return "Politics".to_string(),
                "politics--europe" => return "Politics".to_string(),
                "politics--us" => return "Politics".to_string(),
                "series--aimilestones" => return "AI".to_string(),
                "series--spacex" => return "Technology".to_string(),
                "sports" => return "Sports".to_string(),
                "tech--automotive" => return "Technology".to_string(),
                "tech--energy" => return "Technology".to_string(),
                "tech--general" => return "Technology".to_string(),
                "tech--space" => return "Technology".to_string(),
                _ => continue,
            }
        }
        "None".to_string()
    }
    fn events(&self) -> Vec<ProbUpdate> {
        self.events.to_owned()
    }
    fn resolution(&self) -> Result<f32, MarketConvertError> {
        if let Some(resolution) = self.market.resolution {
            if (0.0..=1.0).contains(&resolution) {
                Ok(resolution)
            } else {
                Err(MarketConvertError {
                    data: self.debug(),
                    message: "Metaculus: Market resolution value out of bounds".to_string(),
                    level: 3,
                })
            }
        } else {
            Err(MarketConvertError {
                data: self.debug(),
                message: "Metaculus: Market resolution value is null".to_string(),
                level: 3,
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
            open_dt: self.open_dt()?,
            close_dt: self.close_dt()?,
            open_days: self.open_days()?,
            volume_usd: self.volume_usd(),
            num_traders: self.num_traders(),
            category: self.category(),
            prob_at_midpoint: self.prob_at_percent(0.5)?,
            prob_at_close: self.prob_at_percent(1.0)?,
            prob_each_pct: self.prob_each_pct_list()?,
            prob_each_date: self.prob_each_date_map()?,
            prob_time_avg: self.prob_time_avg_whole()?,
            resolution: self.resolution()?,
        })
    }
}

/// Test if a market is suitable for analysis.
fn is_valid(market: &MarketInfo) -> bool {
    market.active_state == "RESOLVED"
        && market.possibilities.r#type == Some("binary".to_string())
        && market.resolution >= Some(0.0)
}

/// Convert API events into standard events.
fn get_prob_updates(
    mut points: Vec<PredictionPoint>,
) -> Result<Vec<ProbUpdate>, MarketConvertError> {
    let mut result = Vec::new();
    points.sort_unstable_by_key(|point| point.t as i64);
    for point in points {
        let dt_opt = NaiveDateTime::from_timestamp_opt(point.t as i64, 0);
        if let Some(dt) = dt_opt {
            let time = DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc);
            if let Some(prob) = point.x2.avg {
                result.push(ProbUpdate { time, prob });
            } else {
                return Err(MarketConvertError {
                    data: format!("{:?}", point),
                    message: "Metaculus: History event point.x2.avg is missing".to_string(),
                    level: 3,
                });
            }
        } else {
            return Err(MarketConvertError {
                data: format!("{:?}", point),
                message: "Metaculus: History event timestamp could not be converted into DateTime"
                    .to_string(),
                level: 4,
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
    let api_url = METACULUS_API_BASE.to_owned() + "/questions/" + &market.id.to_string();
    let market_extra: MarketInfoExtra = send_request(client.get(&api_url)).await?;
    Ok(MarketFull {
        market: market.clone(),
        market_extra,
        events: get_prob_updates(market.community_prediction.history.clone())?,
    })
}

/// Download, process and store all valid markets from the platform.
pub async fn get_markets_all(output_method: OutputMethod, verbose: bool) {
    log_to_stdout("Metaculus: Processing started...");
    let client = get_reqwest_client_ratelimited(METACULUS_RATELIMIT, Some(METACULUS_RATELIMIT_MS));
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
        let market_response: BulkMarketResponse = send_request(
            client
                .get(&api_url)
                .query(&[("limit", limit)])
                .query(&[("offset", offset)]),
        )
        .await
        .expect("Metaculus: API query error.");
        if verbose {
            println!(
                "Metaculus: Processing {} markets...",
                market_response.results.len()
            )
        }
        let market_data_futures: Vec<_> = market_response
            .results
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
    log_to_stdout("Metaculus: Processing complete.");
}

/// Download, process and store one market from the platform.
pub async fn get_market_by_id(id: &str, output_method: OutputMethod, verbose: bool) {
    let client = get_reqwest_client_ratelimited(METACULUS_RATELIMIT, Some(METACULUS_RATELIMIT_MS));
    let api_url = METACULUS_API_BASE.to_owned() + "/questions/" + id;
    if verbose {
        println!("Metaculus: Connecting to API at {}", api_url)
    }
    let market_single: MarketInfo = send_request(client.get(&api_url))
        .await
        .expect("Metaculus: API query error.");
    if !is_valid(&market_single) {
        println!("Metaculus: Market is not valid for processing, this may fail.")
    }
    let market_data = get_extended_data(&client, &market_single)
        .await
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
