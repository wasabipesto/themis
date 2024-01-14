//! Tools to download and process markets from the Polymarket API.

use super::*;

const POLYMARKET_GAMMA_API_BASE: &str = "https://gamma-api.polymarket.com/query";
const POLYMARKET_CLOB_API_BASE: &str = "https://clob.polymarket.com";
const POLYMARKET_SITE_BASE: &str = "https://polymarket.com";
const POLYMARKET_RATELIMIT: usize = 100;
const POLYMARKET_EPSILON: f32 = 0.0001;

/// (Indirect) API response with standard market info.
#[allow(non_snake_case)]
#[derive(Deserialize, Debug, Clone)]
struct MarketInfo {
    id: String,
    question: String,
    slug: String,
    createdAt: DateTime<Utc>,
    //startDate: Option<DateTime<Utc>>,
    endDate: Option<DateTime<Utc>>,
    //category: String,
    #[serde(deserialize_with = "deserialize_f32_remove_quotes")]
    volume: f32,
    #[serde(deserialize_with = "deserialize_vec_f32_remove_quotes")]
    outcomePrices: Vec<f32>,
    #[serde(deserialize_with = "deserialize_vec_string_remove_quotes")]
    clobTokenIds: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct GammaResposneLv1 {
    data: GammaResposneLv2,
}

#[derive(Deserialize, Debug, Clone)]
struct GammaResposneLv2 {
    markets: Vec<MarketInfo>,
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

/// Deserialize f32 from a string wrapped in quotes.
pub fn deserialize_f32_remove_quotes<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.trim_matches('"')
        .parse()
        .map_err(|_| serde::de::Error::custom(format!("invalid f32: {}", s)))
}

/// Deserialize Vec<f32> from a string wrapped in quotes.
pub fn deserialize_vec_f32_remove_quotes<'de, D>(deserializer: D) -> Result<Vec<f32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let values: Result<Vec<f32>, _> = s
        .trim_matches(|c| char::is_ascii_punctuation(&c))
        .split(',')
        .map(|value| {
            let cleaned_value = value
                .trim()
                .trim_matches(|c| char::is_ascii_punctuation(&c));
            cleaned_value.parse().map_err(|_| {
                <D::Error as serde::de::Error>::custom(format!("invalid value: {}", cleaned_value))
            })
        })
        .collect();

    values.map_err(|e| serde::de::Error::custom(format!("invalid Vec<f32>: {} from {}", e, s)))
}

/// Deserialize Vec<String> from a string wrapped in quotes.
pub fn deserialize_vec_string_remove_quotes<'de, D>(
    deserializer: D,
) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let values: Result<Vec<String>, _> = s
        .trim_matches(|c| char::is_ascii_punctuation(&c))
        .split(',')
        .map(|value| {
            let cleaned_value = value
                .trim()
                .trim_matches(|c| char::is_ascii_punctuation(&c));
            cleaned_value.parse().map_err(|_| {
                <D::Error as serde::de::Error>::custom(format!("invalid value: {}", cleaned_value))
            })
        })
        .collect();

    values.map_err(|e| serde::de::Error::custom(format!("invalid Vec<String>: {} from {}", e, s)))
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
        self.market.id.to_owned()
    }
    fn url(&self) -> String {
        POLYMARKET_SITE_BASE.to_owned() + "/event/" + &self.market.slug
    }
    fn open_dt(&self) -> Result<DateTime<Utc>, MarketConvertError> {
        /*
        if let Some(open_dt) = self.market.startDate {
            Ok(open_dt)
        } else {
            Ok(self.market.createdAt)
        }
        */
        Ok(self.market.createdAt)
    }
    fn close_dt(&self) -> Result<DateTime<Utc>, MarketConvertError> {
        if let Some(close_dt) = self.market.endDate {
            Ok(close_dt)
        } else {
            Err(MarketConvertError {
                data: self.debug(),
                message: format!("Polymarket: Market field endDate is empty."),
                level: 3,
            })
        }
    }
    fn volume_usd(&self) -> f32 {
        self.market.volume
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
        if let Some(price) = self.market.outcomePrices.first() {
            if price < &POLYMARKET_EPSILON {
                Ok(0.0)
            } else if price > &(1.0 - POLYMARKET_EPSILON) {
                Ok(1.0)
            } else {
                Err(MarketConvertError {
                data: self.debug(),
                message: format!("Polymarket: Current prices are not close enough to bounds to guarantee resolution status."),
                level: 1,
            })
            }
        } else {
            Err(MarketConvertError {
                data: self.debug(),
                message: format!("Polymarket: Market field outcomePrices is empty."),
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
            prob_time_weighted: self.prob_time_weighted()?,
            resolution: self.resolution()?,
        })
    }
}

/// Test if a market is suitable for analysis.
fn is_valid(market: &MarketInfo) -> bool {
    match (market.outcomePrices.first(), market.outcomePrices.last()) {
        (Some(price_a), Some(price_b)) => 1.0 - price_a - price_b < POLYMARKET_EPSILON,
        (Some(_), None) => false,
        (None, Some(_)) => false,
        (None, None) => false,
    }
}

/// Convert API events into standard events.
fn get_prob_updates(
    mut points: Vec<PricesHistoryPoint>,
) -> Result<Vec<ProbUpdate>, MarketConvertError> {
    let mut result = Vec::new();
    points.sort_unstable_by_key(|point| point.t);
    for point in points {
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
    let clob_id = match market.clobTokenIds.first() {
        Some(id) => Ok(id),
        None => Err(MarketConvertError {
            data: format!("{:?}", market),
            message: format!("Polymarket: Market field clobTokenIds is empty."),
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
        let response = client
            .get(&api_url)
            .query(&[("interval", "all")])
            .query(&[("market", clob_id)])
            .query(&[("fidelity", fidelity)])
            .send()
            .await
            .expect("HTTP call failed to execute")
            .error_for_status()
            .unwrap_or_else(|e| panic!("Query failed: {:?}", e))
            .json::<PricesHistoryResponse>()
            .await
            .unwrap();

        // break out if we get data
        if response.history.len() > 0 {
            events.extend(response.history);
            break;
        } else if i >= 5 {
            return Err(MarketConvertError {
                data: format!("{:?}", market),
                message: format!("Polymarket: CLOB returned empty list for price history, even at fidelity = {fidelity}."),
                level: 3,
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
    let client = get_reqwest_client_ratelimited(POLYMARKET_RATELIMIT);
    let api_url = POLYMARKET_GAMMA_API_BASE.to_owned();
    if verbose {
        println!("Polymarket: Connecting to API at {}", api_url)
    }
    let limit: usize = 1000;
    let mut offset: usize = 0;
    loop {
        if verbose {
            println!("Polymarket: Getting markets starting at {:?}...", offset)
        }
        let query = format!("query {{
            markets(
                limit: {limit},
                offset: {offset}
                where: \"active = true and closed = true and end_date < now() and volume_num > 0 and enable_order_book = true and jsonb_array_length(clob_token_ids) = 2\"
            ) {{
                id,
                question,
                slug,
                createdAt,
                startDate,
                endDate,
                category,
                liquidity,
                volume,
                outcomePrices,
                clobTokenIds,
            }}
        }}");
        let response = client
            .get(&api_url)
            .query(&[("query", query)])
            .send()
            .await
            .expect("HTTP call failed to execute")
            .error_for_status()
            .unwrap_or_else(|e| panic!("Query failed: {:?}", e))
            .json::<GammaResposneLv1>()
            .await
            .unwrap();
        if verbose {
            println!(
                "Polymarket: Processing {} markets...",
                response.data.markets.len()
            )
        }
        let market_data_futures: Vec<_> = response
            .data
            .markets
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
        if response.data.markets.len() == limit {
            offset += limit;
        } else {
            break;
        }
    }
    println!("Polymarket: Processing complete.");
}

/// Download, process and store one market from the platform.
pub async fn get_market_by_id(id: &String, output_method: OutputMethod, verbose: bool) {
    let client = get_reqwest_client_ratelimited(POLYMARKET_RATELIMIT);
    let api_url = POLYMARKET_GAMMA_API_BASE.to_owned();
    if verbose {
        println!("Polymarket: Connecting to API at {}", api_url)
    }
    let query = format!(
        "query {{
            markets(
                where: \"id = {id}\"
            ) {{
                id,
                question,
                slug,
                createdAt,
                startDate,
                endDate,
                category,
                liquidity,
                volume,
                outcomePrices,
                clobTokenIds,
            }}
        }}"
    );
    let response = client
        .get(&api_url)
        .query(&[("query", query)])
        .send()
        .await
        .expect("HTTP call failed to execute")
        .error_for_status()
        .unwrap_or_else(|e| panic!("Query failed: {:?}", e))
        .json::<GammaResposneLv1>()
        .await
        .unwrap();
    let single_market = response
        .data
        .markets
        .first()
        .expect("Polymarket: Gamma market query response was empty.");
    if !is_valid(single_market) {
        println!("Polymarket: Market is not valid for processing, this may fail.")
    }
    let market_data = get_extended_data(&client, single_market)
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
