use super::*;
use std::cmp;

const MANIFOLD_API_BASE: &str = "https://api.manifold.markets/v0";
const MANIFOLD_SITE_BASE: &str = "https://manifold.markets/";
const MANIFOLD_EXCHANGE_RATE: f32 = 100.0;

#[allow(non_snake_case)]
#[derive(Deserialize, Debug, Clone)]
struct MarketInfo {
    id: String,
    question: String,
    slug: String,
    creatorUsername: String,
    isResolved: bool,
    createdTime: i64,
    closeTime: Option<i64>, // polls and bounties lack close times
    resolutionTime: Option<i64>,
    volume: f32,
}

impl MarketInfoDetails for MarketInfo {
    fn is_valid(&self) -> bool {
        self.isResolved
    }
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

impl MarketFullDetails for MarketFull {
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
            None => Err(MarketConvertError::new(
                self.debug(),
                "Manifold Market createdTime could not be converted into DateTime",
            )),
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
            (None, None) => Err(MarketConvertError::new(
                format!("{:?}", self),
                "Manifold Market response did not include closeTime or resolutionTime",
            )),
        }?;
        match get_datetime_from_millis(ts) {
            Ok(time) => Ok(time),
            Err(_) => Err(MarketConvertError::new(
                format!("{:?}", &self),
                "Manifold Market closeTime or resolveTime could not be converted into DateTime",
            )),
        }
    }
    fn volume_usd(&self) -> f32 {
        self.market.volume / MANIFOLD_EXCHANGE_RATE
    }
    fn events(&self) -> Vec<MarketEvent> {
        self.events.to_owned()
    }
}

impl TryInto<MarketForDB> for MarketFull {
    type Error = MarketConvertError;
    fn try_into(self) -> Result<MarketForDB, MarketConvertError> {
        Ok(MarketForDB {
            title: self.title(),
            platform: self.platform(),
            platform_id: self.platform_id(),
            url: self.url(),
            open_days: self.open_days()?,
            volume_usd: self.volume_usd(),
            prob_at_midpoint: self.prob_at_percent(0.5)?,
            prob_at_close: self.prob_at_percent(1.0)?,
        })
    }
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
                return Err(MarketConvertError::new(
                    format!("{:?}", bet),
                    "Manifold Bet createdTime timestamp could not be converted into DateTime",
                ));
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
        let response: Vec<Bet> = client
            .get(&api_url)
            .query(&[("contractId", &market.id)])
            .query(&[("limit", limit)])
            .query(&[("before", before)])
            .send()
            .await
            .unwrap()
            .json::<Vec<Bet>>()
            .await
            .unwrap();
        let response_len = response.len();
        all_bet_data.extend(response);
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

pub async fn get_markets_all() -> Vec<MarketForDB> {
    let client = get_default_client();
    let api_url = MANIFOLD_API_BASE.to_owned() + "/markets";
    let limit = 1000;
    let mut before: Option<String> = None;
    let mut all_market_data: Vec<MarketFull> = Vec::new();
    loop {
        let response: Vec<MarketInfo> = client
            .get(&api_url)
            .query(&[("limit", limit)])
            .query(&[("before", before)])
            .send()
            .await
            .unwrap()
            .json::<Vec<MarketInfo>>()
            .await
            .unwrap();
        let market_data_futures: Vec<_> = response
            .iter()
            .filter(|market| market.is_valid())
            .map(|market| get_extended_data(&client, market))
            .collect();
        let market_data: Vec<MarketFull> = join_all(market_data_futures)
            .await
            .into_iter()
            .map(|i| i.expect("Error processing market"))
            .collect();
        all_market_data.extend(market_data);
        if response.len() == limit {
            before = Some(all_market_data.last().unwrap().market.id.clone());
        } else {
            break;
        }
    }
    all_market_data
        .into_iter()
        .map(|market| {
            market
                .try_into()
                .expect("Error converting market into standard fields.")
        })
        .collect()
}

pub async fn get_market_by_id(id: &String) -> Vec<MarketForDB> {
    let client = get_default_client();
    let api_url = MANIFOLD_API_BASE.to_owned() + "/market/" + &id;
    let response = client
        .get(&api_url)
        .send()
        .await
        .unwrap()
        .json::<MarketInfo>()
        .await
        .unwrap();
    let market_data = get_extended_data(&client, &response).await.unwrap();
    Vec::from([market_data.try_into().expect("Error processing market")])
}
