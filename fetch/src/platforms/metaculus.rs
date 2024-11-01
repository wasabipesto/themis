//! Tools to download and process markets from the Metaculus API.

use super::*;

const METACULUS_API_BASE: &str = "https://www.metaculus.com/api2";
const METACULUS_SITE_BASE: &str = "https://www.metaculus.com";
const METACULUS_USD_PER_FORECAST: f32 = 0.10;
const METACULUS_RATELIMIT: usize = 15;
const METACULUS_RATELIMIT_MS: u64 = 60_000;

#[derive(Deserialize, Debug, Clone)]
struct BulkMarketResponse {
    //next: String, // just use offset instead
    results: Vec<MarketInfo>,
}

#[derive(Deserialize, Debug, Clone)]
struct MarketInfo {
    id: u32,
    title: String,
    //status: String,
    resolved: bool,
    nr_forecasters: i32,
    forecasts_count: u32,
    created_at: DateTime<Utc>,
    published_at: Option<DateTime<Utc>>,
    actual_close_time: Option<DateTime<Utc>>,
    projects: MarketProjects,
    question: MarketQuestionDetails,
}

#[derive(Deserialize, Debug, Clone)]
struct MarketProjects {
    category: Option<Vec<Category>>,
}

#[derive(Deserialize, Debug, Clone)]
struct Category {
    //name: String,
    slug: String,
}

#[derive(Deserialize, Debug, Clone)]
struct MarketQuestionDetails {
    //description: String,
    r#type: Option<String>,
    resolution: Option<String>,
    aggregations: PredictionAggregations,
}

#[derive(Deserialize, Debug, Clone)]
struct PredictionAggregations {
    /// Formerly referred to as "community prediction"
    recency_weighted: AggregationData,
    //unweighted: AggregationData,
    //single_aggregation: AggregationData,
    //metaculus_prediction: AggregationData,
}

#[derive(Deserialize, Debug, Clone)]
struct AggregationData {
    history: Vec<PredictionPoint>,
    //latest: PredictionPoint,
}

#[derive(Deserialize, Debug, Clone)]
struct PredictionPoint {
    start_time: f32,
    end_time: Option<f32>,
    means: Vec<f32>,
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
        METACULUS_SITE_BASE.to_owned() + "/questions/" + &self.market.id.to_string() + "/"
    }
    fn open_dt(&self) -> Result<DateTime<Utc>, MarketConvertError> {
        match self.market.published_at {
            Some(published_at) => Ok(published_at),
            None => Ok(self.market.created_at),
        }
    }
    fn close_dt(&self) -> Result<DateTime<Utc>, MarketConvertError> {
        if let Some(close_time) = self.market.actual_close_time {
            Ok(close_time)
        } else {
            Err(MarketConvertError {
                data: self.debug(),
                message: "Metaculus: actual_close_time is missing from closed market".to_string(),
                level: 3,
            })
        }
    }
    fn volume_usd(&self) -> f32 {
        self.market.forecasts_count as f32 * METACULUS_USD_PER_FORECAST
    }
    fn num_traders(&self) -> i32 {
        self.market.nr_forecasters
    }
    fn category(&self) -> String {
        if let Some(categories) = &self.market.projects.category {
            for category in categories {
                match category.slug.as_str() {
                    // TODO: need to re-do these categories, seem to have changed
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
        } else {
            "None".to_string()
        }
    }
    fn events(&self) -> Vec<ProbUpdate> {
        self.events.to_owned()
    }
    fn resolution(&self) -> Result<f32, MarketConvertError> {
        if let Some(res_text) = &self.market.question.resolution {
            match res_text.as_str() {
                "yes" => Ok(1.0),
                "no" => Ok(0.0),
                _ => Err(MarketConvertError {
                    data: self.debug(),
                    message: "Metaculus: Market resolution value is not \"yes\" or \"no\""
                        .to_string(),
                    level: 3,
                }),
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
            prob_at_pct: self.prob_at_pct_list()?,
            prob_time_avg: self.prob_time_avg_whole()?,
            resolution: self.resolution()?,
        })
    }
}

/// Test if a market is suitable for analysis.
fn is_valid(market: &MarketInfo) -> bool {
    market.resolved == true && market.question.r#type == Some("binary".to_string())
}

/// Convert API events into standard events.
fn get_prob_updates(
    mut points: Vec<PredictionPoint>,
) -> Result<Vec<ProbUpdate>, MarketConvertError> {
    let mut result = Vec::new();
    // not ideal to sort by start_time as i64 but they're always more than 1m apart anyways
    points.sort_unstable_by_key(|point| point.start_time as i64);
    for point in points {
        // get timestamp, ideally midpoint between start_time and end_time
        let center_time = if let Some(end_time) = point.end_time {
            ((point.start_time + end_time) / 2.0) as i64
        } else {
            // on live markets there is no end_time for the last bucket
            // we shouldn't hit this in practice
            point.start_time as i64
        };
        let dt_opt = DateTime::from_timestamp(center_time, 0);
        if let Some(time) = dt_opt {
            if let Some(prob) = point.means.first() {
                result.push(ProbUpdate {
                    time,
                    prob: prob.clone(),
                });
            } else {
                return Err(MarketConvertError {
                    data: format!("{:?}", point),
                    message: "Metaculus: History event point \"means\" list is empty".to_string(),
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
/// TODO remove this?
async fn get_extended_data(
    _client: &ClientWithMiddleware,
    market: &MarketInfo,
) -> Result<MarketFull, MarketConvertError> {
    //let api_url = METACULUS_API_BASE.to_owned() + "/questions/" + &market.id.to_string();
    //let market_extra: MarketInfoExtra = send_request(client.get(&api_url)).await?;
    Ok(MarketFull {
        market: market.clone(),
        //market_extra,
        events: get_prob_updates(
            market
                .question
                .aggregations
                .recency_weighted
                .history
                .clone(),
        )?,
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
