use super::*;
use std::cmp;

const MANIFOLD_API_BASE: &str = "https://api.manifold.markets/v0";
const MANIFOLD_SITE_BASE: &str = "https://manifold.markets/";
const MANIFOLD_EXCHANGE_RATE: f32 = 100.0;
const MANIFOLD_RATELIMIT: usize = 20;

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
    resolution: String,
    resolutionProbability: Option<f32>,
    createdTime: i64,
    closeTime: Option<i64>, // polls and bounties lack close times
    resolutionTime: Option<i64>,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct Bet {
    id: String,
    createdTime: i64,
    //probBefore: Option<f32>,
    probAfter: Option<f32>,
    //amount: f32,
    //shares: f32,
    //outcome: f32,
}

#[derive(Debug)]
struct MarketFull {
    market: MarketInfo,
    events: Vec<MarketEvent>,
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
                message: "Manifold Market createdTime could not be converted into DateTime"
                    .to_string(),
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
                message: "Manifold Market response did not include closeTime or resolutionTime"
                    .to_string(),
            }),
        }?;
        match get_datetime_from_millis(ts) {
            Ok(time) => Ok(time),
            Err(_) => Err(MarketConvertError {
                data: format!("{:?}", &self),
                message:
                    "Manifold Market closeTime or resolveTime could not be converted into DateTime"
                        .to_string(),
            }),
        }
    }
    fn volume_usd(&self) -> f32 {
        self.market.volume / MANIFOLD_EXCHANGE_RATE
    }
    fn events(&self) -> Vec<MarketEvent> {
        self.events.to_owned()
    }
    fn resolution(&self) -> Result<f32, MarketConvertError> {
        match self.market.resolution.as_str() {
            "YES" => Ok(1.0),
            "NO" => Ok(0.0),
            "MKT" => {
                if let Some(res) = self.market.resolutionProbability {
                    Ok(res)
                } else {
                    Err(MarketConvertError {
                        data: self.debug(),
                        message: "Market resolved to MKT but is missing resolutionProbability"
                            .to_string(),
                    })
                }
            }
            _ => Err(MarketConvertError {
                data: self.debug(),
                message: "Market resolved to something besides YES, NO, or MKT".to_string(),
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
    market.isResolved
        && market.mechanism == "cpmm-1"
        && market.outcomeType == "BINARY"
        && market.resolution != "CANCEL"
}

fn get_datetime_from_millis(ts: i64) -> Result<DateTime<Utc>, ()> {
    let dt = NaiveDateTime::from_timestamp_millis(ts);
    match dt {
        Some(dt) => Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc)),
        None => Err(()),
    }
}

fn get_events_from_bets(mut bets: Vec<Bet>) -> Result<Vec<MarketEvent>, MarketConvertError> {
    let mut result = Vec::new();
    bets.sort_unstable_by_key(|b| b.createdTime);
    for bet in bets {
        if let Some(prob) = bet.probAfter {
            if let Ok(time) = get_datetime_from_millis(bet.createdTime) {
                result.push(MarketEvent { time, prob });
            } else {
                return Err(MarketConvertError {
                    data: format!("{:?}", bet),
                    message:
                        "Manifold Bet createdTime timestamp could not be converted into DateTime"
                            .to_string(),
                });
            }
        }
    }

    Ok(result)
}

async fn get_extended_data(
    client: &ClientWithMiddleware,
    market: &MarketInfo,
) -> Result<MarketFull, MarketConvertError> {
    let api_url = MANIFOLD_API_BASE.to_owned() + "/bets";
    let limit = 1000;
    let mut before: Option<String> = None;
    let mut all_bet_data: Vec<Bet> = Vec::new();
    loop {
        let response_text = client
            .get(&api_url)
            .query(&[("contractId", &market.id)])
            .query(&[("limit", limit)])
            .query(&[("before", before)])
            .send()
            .await
            .expect("HTTP call failed to execute")
            .text()
            .await
            .expect("Failed to get response text");
        let bet_data: Vec<Bet> =
            serde_json::from_str(&response_text).map_err(|e| MarketConvertError {
                data: format!("{:?}", response_text),
                message: format!("Bet failed to deserialize: {:?}", e),
            })?;
        let response_len = bet_data.len();
        all_bet_data.extend(bet_data);
        if response_len == limit {
            before = Some(all_bet_data.last().unwrap().id.clone());
        } else {
            break;
        }
    }

    Ok(MarketFull {
        market: market.clone(),
        events: get_events_from_bets(all_bet_data)?,
    })
}

pub async fn get_markets_all() -> Vec<MarketStandard> {
    let client = get_reqwest_client_ratelimited(MANIFOLD_RATELIMIT);
    let api_url = MANIFOLD_API_BASE.to_owned() + "/markets";
    let limit = 1000;
    let mut before: Option<String> = None;
    let mut all_market_data = Vec::new();
    loop {
        let market_response: Vec<MarketInfo> = client
            .get(&api_url)
            .query(&[("limit", limit)])
            .query(&[("before", before)])
            .send()
            .await
            .expect("HTTP call failed to execute")
            .json::<Vec<MarketInfo>>()
            .await
            .expect("Market failed to deserialize");
        let market_data_futures: Vec<_> = market_response
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
                    .expect("Error converting market into standard fields.")
            })
            .collect();
        all_market_data.extend(market_data);
        if market_response.len() == limit {
            before = Some(market_response.last().unwrap().id.clone());
        } else {
            break;
        }
    }
    all_market_data
}

pub async fn get_market_by_id(id: &String) -> Vec<MarketStandard> {
    let client = get_reqwest_client_ratelimited(MANIFOLD_RATELIMIT);
    let api_url = MANIFOLD_API_BASE.to_owned() + "/market/" + id;
    let market_single = client
        .get(&api_url)
        .send()
        .await
        .expect("HTTP call failed to execute")
        .json::<MarketInfo>()
        .await
        .expect("Market failed to deserialize");
    if !is_valid(&market_single) {
        println!("Market is not valid for processing, this may fail.")
    }
    let market_data = get_extended_data(&client, &market_single)
        .await
        .expect("Error getting extended market data")
        .try_into()
        .expect("Error converting market into standard fields");
    Vec::from([market_data])
}
