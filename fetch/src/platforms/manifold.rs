//! Tools to download and process markets from the Manifold API.

use super::*;
use std::cmp;

const MANIFOLD_API_BASE: &str = "https://api.manifold.markets/v0";
const MANIFOLD_SITE_BASE: &str = "https://manifold.markets/";
const MANIFOLD_EXCHANGE_RATE: f32 = 100.0;
const MANIFOLD_RATELIMIT: usize = 15;

/// API response with standard market info from `/markets`.
#[allow(non_snake_case)]
#[derive(Deserialize, Debug, Clone)]
struct MarketInfo {
    id: String,
    question: String,
    slug: String,
    creatorUsername: String,
    mechanism: String,
    volume: f32,
    outcomeType: String,
    isResolved: bool,
    resolution: Option<String>,
    resolutionProbability: Option<f32>,
    createdTime: i64,
    closeTime: Option<i64>, // polls and bounties lack close times
    resolutionTime: Option<i64>,
}

/// API response with extended info from `/market`.
#[allow(non_snake_case)]
#[derive(Deserialize, Debug, Clone)]
struct MarketInfoExtra {
    groupSlugs: Option<Vec<String>>,
}

/// API response with standard bet info from `/bets`.
#[allow(non_snake_case)]
#[derive(Deserialize, Debug, Clone)]
struct Bet {
    id: String,
    userId: String,
    createdTime: i64,
    //probBefore: Option<f32>,
    probAfter: Option<f32>,
    //amount: f32,
    //shares: f32,
    //outcome: f32,
}

/// Container for market data and events, used to hold data for conversion.
#[derive(Debug)]
struct MarketFull {
    market: MarketInfo,
    market_extra: MarketInfoExtra,
    bets: Vec<Bet>,
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
        "manifold".to_string()
    }
    fn platform_id(&self) -> String {
        self.market.id.to_owned()
    }
    fn url(&self) -> String {
        MANIFOLD_SITE_BASE.to_owned() + &self.market.creatorUsername + "/" + &self.market.slug
    }
    fn open_dt(&self) -> Result<DateTime<Utc>, MarketConvertError> {
        let ts = self.market.createdTime;
        let dt = NaiveDateTime::from_timestamp_millis(ts);
        match dt {
            Some(dt) => Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc)),
            None => Err(MarketConvertError {
                data: self.debug(),
                message: "Manifold: Market createdTime could not be converted into DateTime"
                    .to_string(),
                level: 3,
            }),
        }
    }
    fn close_dt(&self) -> Result<DateTime<Utc>, MarketConvertError> {
        let ts = match (self.market.closeTime, self.market.resolutionTime) {
            // both close and resolution times are present
            (Some(close_time), Some(resolution_time)) => {
                if close_time < self.market.createdTime {
                    // close time was set in the past, use resolution time instead
                    Ok(resolution_time)
                } else {
                    // close time and resolution time were both after created time, take whichever came first
                    Ok(cmp::min(close_time, resolution_time))
                }
            }
            // only resolution time is present
            (Some(close_time), None) => Ok(close_time),
            // only close time is present
            (None, Some(resolution_time)) => Ok(resolution_time),
            // neither is present
            (None, None) => Err(MarketConvertError {
                data: format!("{:?}", self),
                message: "Manifold: Market response did not include closeTime or resolutionTime"
                    .to_string(),
                level: 3,
            }),
        }?;
        match get_datetime_from_millis(ts) {
            Ok(time) => Ok(time),
            Err(_) => Err(MarketConvertError {
                data: format!("{:?}", &self),
                message:
                    "Manifold: Market closeTime or resolveTime could not be converted into DateTime"
                        .to_string(),
                level: 3,
            }),
        }
    }
    fn volume_usd(&self) -> f32 {
        self.market.volume / MANIFOLD_EXCHANGE_RATE
    }
    fn num_traders(&self) -> i32 {
        self.bets
            .iter()
            .map(|bet| bet.userId.clone())
            .collect::<std::collections::HashSet<_>>()
            .len() as i32
    }
    fn category(&self) -> String {
        if let Some(categories) = &self.market_extra.groupSlugs {
            for category in categories {
                match category.as_str() {
                    "118th-congress" => return "Politics".to_string(),
                    "2024-us-presidential-election" => return "Politics".to_string(),
                    //"africa" => return "Other".to_string(),
                    "ai" => return "AI".to_string(),
                    "ai-alignment" => return "AI".to_string(),
                    "ai-safety" => return "AI".to_string(),
                    "arabisraeli-conflict" => return "Politics".to_string(),
                    "apple" => return "Technology".to_string(),
                    "baseball" => return "Sports".to_string(),
                    "basketball" => return "Sports".to_string(),
                    "biotech" => return "Science".to_string(),
                    "bitcoin" => return "Crypto".to_string(),
                    "celebrities" => return "Culture".to_string(),
                    "chatgpt" => return "AI".to_string(),
                    "chess" => return "Sports".to_string(),
                    //"china" => return "Other".to_string(),
                    "climate" => return "Climate".to_string(),
                    "crypto-speculation" => return "Crypto".to_string(),
                    "culture-default" => return "Culture".to_string(),
                    //"daliban-hq" => return "Other".to_string(),
                    //"destinygg" => return "Other".to_string(),
                    //"destinygg-stocks" => return "Other".to_string(),
                    "donald-trump" => return "Politics".to_string(),
                    "economics-default" => return "Economics".to_string(),
                    //"effective-altruism" => return "Other".to_string(),
                    //"elon-musk-14d9d9498c7e" => return "Other".to_string(),
                    //"europe" => return "Other".to_string(),
                    "f1" => return "Sports".to_string(),
                    "finance" => return "Economics".to_string(),
                    "football" => return "Sports".to_string(),
                    "formula-1" => return "Sports".to_string(),
                    //"fun" => return "Other".to_string(),
                    "gaming" => return "Culture".to_string(),
                    "gpt4-speculation" => return "AI".to_string(),
                    //"health" => return "Other".to_string(),
                    //"india" => return "Other".to_string(),
                    "internet" => return "Technology".to_string(),
                    //"israel" => return "Other".to_string(),
                    "israelhamas-conflict-2023" => return "Politics".to_string(),
                    "israeli-politics" => return "Politics".to_string(),
                    //"latin-america" => return "Other".to_string(),
                    //"lgbtqia" => return "Other".to_string(),
                    //"mathematics" => return "Other".to_string(),
                    "medicine" => return "Science".to_string(),
                    //"middle-east" => return "Other".to_string(),
                    "movies" => return "Culture".to_string(),
                    "music-f213cbf1eab5" => return "Culture".to_string(),
                    "nfl" => return "Sports".to_string(),
                    "nuclear" => return "Science".to_string(),
                    "nuclear-risk" => return "Politics".to_string(),
                    //"one-piece-stocks" => return "Other".to_string(),
                    "openai" => return "AI".to_string(),
                    "openai-9e1c42b2bb1e" => return "AI".to_string(),
                    "openai-crisis" => return "AI".to_string(),
                    //"personal-goals" => return "Other".to_string(),
                    "physics" => return "Science".to_string(),
                    "politics-default" => return "Politics".to_string(),
                    "programming" => return "Technology".to_string(),
                    //"russia" => return "Other".to_string(),
                    //"sam-altman" => return "Other".to_string(),
                    "science-default" => return "Science".to_string(),
                    //"sex-and-love" => return "Other".to_string(),
                    "soccer" => return "Sports".to_string(),
                    "space" => return "Science".to_string(),
                    "speaker-of-the-house-election" => return "Politics".to_string(),
                    "sports-default" => return "Sports".to_string(),
                    "startups" => return "Economics".to_string(),
                    "stocks" => return "Economics".to_string(),
                    "technical-ai-timelines" => return "AI".to_string(),
                    "technology-default" => return "Technology".to_string(),
                    "tennis" => return "Sports".to_string(),
                    //"the-life-of-biden" => return "Other".to_string(),
                    "time-person-of-the-year" => return "Culture".to_string(),
                    "tv" => return "Culture".to_string(),
                    //"twitter" => return "Technology".to_string(),
                    "uk-politics" => return "Politics".to_string(),
                    "ukraine" => return "Politics".to_string(),
                    "ukrainerussia-war" => return "Politics".to_string(),
                    "us-politics" => return "Politics".to_string(),
                    "wars" => return "Politics".to_string(),
                    "world-default" => return "Politics".to_string(),
                    _ => continue,
                }
            }
        }
        "None".to_string()
    }
    fn events(&self) -> Vec<ProbUpdate> {
        self.events.to_owned()
    }
    fn resolution(&self) -> Result<f32, MarketConvertError> {
        match &self.market.resolution {
            Some(resolution_text) => match resolution_text.as_str() {
                "YES" => Ok(1.0),
                "NO" => Ok(0.0),
                "MKT" => {
                    if let Some(res) = self.market.resolutionProbability {
                        Ok(res)
                    } else {
                        Err(MarketConvertError {
                            data: self.debug(),
                            message: "Manifold: Market resolved to MKT but is missing resolutionProbability"
                                .to_string(),
                                level: 3,
                        })
                    }
                }
                _ => Err(MarketConvertError {
                    data: self.debug(),
                    message: "Manifold: Market resolved to something besides YES, NO, or MKT"
                        .to_string(),
                    level: 3,
                }),
            },
            _ => Err(MarketConvertError {
                data: self.debug(),
                message: "Manifold: Market resolved without `resolution` value".to_string(),
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
    market.isResolved
        && market.mechanism == "cpmm-1"
        && market.outcomeType == "BINARY"
        && market.volume > 0.0
        && market.resolution != Some("CANCEL".to_string())
}

/// Convert API events into standard events.
fn get_prob_updates(mut bets: Vec<Bet>) -> Result<Vec<ProbUpdate>, MarketConvertError> {
    let mut result = Vec::new();
    bets.sort_unstable_by_key(|b| b.createdTime);
    for bet in bets {
        if let Some(prob) = bet.probAfter {
            if let Ok(time) = get_datetime_from_millis(bet.createdTime) {
                result.push(ProbUpdate { time, prob });
            } else {
                return Err(MarketConvertError {
                    data: format!("{:?}", bet),
                    message:
                        "Manifold: Bet createdTime timestamp could not be converted into DateTime"
                            .to_string(),
                    level: 3,
                });
            }
        }
    }

    Ok(result)
}

/// Download full market history and store events in the container.
async fn get_extended_data(
    client: &ClientWithMiddleware,
    market: &MarketInfo,
) -> Result<MarketFull, MarketConvertError> {
    // get trade info from /bets
    let api_url = MANIFOLD_API_BASE.to_owned() + "/bets";
    let limit = 1000;
    let mut before: Option<String> = None;
    let mut all_bet_data: Vec<Bet> = Vec::new();
    loop {
        let bet_data: Vec<Bet> = send_request(
            client
                .get(&api_url)
                .query(&[("contractId", &market.id)])
                .query(&[("limit", &limit)])
                .query(&[("before", &before)]),
        )
        .await?;
        if bet_data.len() == limit {
            all_bet_data.extend(bet_data);
            before = Some(all_bet_data.last().unwrap().id.clone());
        } else {
            all_bet_data.extend(bet_data);
            break;
        }
    }

    // get extra data from /market
    let api_url = MANIFOLD_API_BASE.to_owned() + "/market/" + &market.id;
    let market_extra: MarketInfoExtra = send_request(client.get(&api_url)).await?;

    // save
    Ok(MarketFull {
        market: market.clone(),
        market_extra,
        bets: all_bet_data.clone(),
        events: get_prob_updates(all_bet_data)?,
    })
}

/// Download, process and store all valid markets from the platform.
pub async fn get_markets_all(output_method: OutputMethod, verbose: bool) {
    println!("Manifold: Processing started...");
    let client = get_reqwest_client_ratelimited(MANIFOLD_RATELIMIT);
    let api_url = MANIFOLD_API_BASE.to_owned() + "/markets";
    if verbose {
        println!("Manifold: Connecting to API at {}", api_url)
    }
    let limit = 1000;
    let mut before: Option<String> = None;
    loop {
        if verbose {
            println!("Manifold: Getting markets starting at {:?}...", before)
        }
        let market_response: Vec<MarketInfo> = send_request(
            client
                .get(&api_url)
                .query(&[("limit", limit)])
                .query(&[("before", before)]),
        )
        .await
        .expect("Manifold: API query error.");
        if verbose {
            println!("Manifold: Processing {} markets...", market_response.len())
        }
        let market_data_futures: Vec<_> = market_response
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
                "Manifold: Saving {} processed markets to {:?}...",
                market_data.len(),
                output_method
            )
        }
        save_markets(market_data, output_method);
        if market_response.len() == limit {
            before = Some(market_response.last().unwrap().id.clone());
        } else {
            break;
        }
    }
    println!("Manifold: Processing complete.");
}

/// Download, process and store one market from the platform.
pub async fn get_market_by_id(id: &str, output_method: OutputMethod, verbose: bool) {
    let client = get_reqwest_client_ratelimited(MANIFOLD_RATELIMIT);
    let api_url = MANIFOLD_API_BASE.to_owned() + "/market/" + id;
    if verbose {
        println!("Manifold: Connecting to API at {}", api_url)
    }
    let market_single: MarketInfo = send_request(client.get(&api_url))
        .await
        .expect("Manifold: API query error.");
    if !is_valid(&market_single) {
        println!("Manifold: Market is not valid for processing, this may fail.")
    }
    let market_data = get_extended_data(&client, &market_single)
        .await
        .expect("Error getting extended market data")
        .try_into()
        .expect("Error converting market into standard fields");
    if verbose {
        println!(
            "Manifold: Saving processed market to {:?}...",
            output_method
        )
    }
    save_markets(Vec::from([market_data]), output_method);
}
