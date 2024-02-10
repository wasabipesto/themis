use super::*;

/// Parameters passed to the accuracy function.
/// If the parameter is not supplied, the default values are used.
/// TODO: Change to enums
#[derive(Debug, Deserialize, Serialize)]
pub struct AccuracyQueryParams {
    #[serde(default = "default_scoring_attribute")]
    scoring_attribute: String,
    #[serde(default = "default_weight_attribute")]
    weight_attribute: String,
    #[serde(default = "default_xaxis_attribute")]
    xaxis_attribute: String,
    #[serde(default = "default_num_market_points")]
    num_market_points: usize,
    #[serde(flatten)]
    pub filters: CommonFilterParams,
}
fn default_scoring_attribute() -> String {
    "prob_at_midpoint".to_string()
}
fn default_weight_attribute() -> String {
    "none".to_string()
}
fn default_xaxis_attribute() -> String {
    "open_days".to_string()
}
fn default_num_market_points() -> usize {
    1000
}

/// Data for each bin and the markets included.
struct XAxisBin {
    start: f32,
    middle: f32,
    end: f32,
    y_axis_numerator: f32,
    y_axis_denominator: f32,
}

/// An individual datapoint to be plotted.
#[derive(Debug, Serialize)]
struct Point {
    x: f32,
    y: f32,
    desc: String,
}

/// Data sent to the client to render a plot, one plot per platform.
#[derive(Debug, Serialize)]
struct Trace {
    platform: Platform,
    market_points: Vec<Point>,
    //accuracy_line: Vec<Point>,
}

/// Metadata to help label a plot.
#[derive(Debug, Serialize)]
struct PlotMetadata {
    title: String,
    x_title: String,
    y_title: String,
}

/// Full response for a plot.
#[derive(Debug, Serialize)]
struct AccuracyPlotResponse {
    query: AccuracyQueryParams,
    metadata: PlotMetadata,
    traces: Vec<Trace>,
}

/// TODO
fn generate_xaxis_bins(bin_size: &f32) -> Result<Vec<XAxisBin>, ApiError> {
    let mut bins: Vec<XAxisBin> = Vec::new();
    Ok(bins)
}

/// Get the predicted value of the market, based on the user-defined scoring attribute.
fn get_market_scoring_value(market: &Market, query: &AccuracyQueryParams) -> Result<f32, ApiError> {
    match query.scoring_attribute.as_str() {
        "prob_at_midpoint" => Ok(market.prob_at_midpoint),
        "prob_at_close" => Ok(market.prob_at_close),
        "prob_time_avg" => Ok(market.prob_time_avg),
        _ => Err(ApiError::new(
            400,
            "the value provided for `scoring_attribute` is not a valid option".to_string(),
        )),
    }
}

/// Get the x-value of the market, based on the user-defined attribute.
fn get_market_xaxis_value(market: &Market, query: &AccuracyQueryParams) -> Result<f32, ApiError> {
    match query.xaxis_attribute.as_str() {
        //"open_dt" => Ok(market.open_dt),
        //"close_dt" => Ok(market.close_dt),
        "open_days" => Ok(market.open_days),
        "volume_usd" => Ok(market.volume_usd),
        "num_traders" => Ok(market.num_traders as f32),
        _ => Err(ApiError::new(
            400,
            "the value provided for `xaxis_attribute` is not a valid option".to_string(),
        )),
    }
}

/// Get the weighting value of the market, based on the user-defined weighting attribute.
fn get_market_weight_value(market: &Market, query: &AccuracyQueryParams) -> Result<f32, ApiError> {
    match query.weight_attribute.as_str() {
        "open_days" => Ok(market.open_days),
        "num_traders" => Ok(market.num_traders as f32),
        "volume_usd" => Ok(market.volume_usd),
        "none" => Ok(1.0),
        _ => Err(ApiError::new(
            400,
            "the value provided for `weight_attribute` is not a valid option".to_string(),
        )),
    }
}

/// Get the x-axis title of the plot, based on the user-defined bin attribute.
fn get_x_axis_title(query: &AccuracyQueryParams) -> Result<String, ApiError> {
    match query.xaxis_attribute.as_str() {
        //"open_dt" => Ok(format!("Market Open Date")),
        //"close_dt" => Ok(format!("Market Close Date")),
        "open_days" => Ok(format!("Market Open Length (days)")),
        "volume_usd" => Ok(format!("Market Volume (USD)")),
        "num_traders" => Ok(format!("Number of Unique Traders")),
        _ => Err(ApiError {
            status_code: 500,
            message: format!("given xaxis_attribute not in x_title map"),
        }),
    }
}

/// Get the y-axis title of the plot, based on the user-defined weight attribute.
fn get_y_axis_title(query: &AccuracyQueryParams) -> Result<String, ApiError> {
    match query.scoring_attribute.as_str() {
        "prob_at_midpoint" => Ok(format!("Brier Score from Midpoint Probability")),
        "prob_at_close" => Ok(format!("Brier Score from Closing Probability")),
        "prob_time_avg" => Ok(format!("Brier Score from Time-Averaged Probability")),
        _ => Err(ApiError {
            status_code: 500,
            message: format!("given scoring_attribute not in y_title map"),
        }),
    }
}

/// Takes a markets and generates its brier score.
fn get_market_brier_score(
    market: &Market,
    query: &Query<AccuracyQueryParams>,
) -> Result<f32, ApiError> {
    let market_resolved_value = market.resolution;
    let market_predicted_value = get_market_scoring_value(&market, &query)?;
    let brier_score = (market_resolved_value - market_predicted_value).powf(2.0);
    Ok(brier_score)
}

/// Takes a set of markets and generates a brier score.
fn get_list_brier_score(
    markets: &Vec<Market>,
    query: &Query<AccuracyQueryParams>,
) -> Result<f32, ApiError> {
    // set up brier counters
    let mut weighted_brier_sum: f32 = 0.0;
    let mut weighted_brier_count: f32 = 0.0;

    // this is a hot loop since we iterate over all markets
    for market in markets {
        let market_predicted_value = get_market_scoring_value(&market, &query)?;
        let market_resolved_value = market.resolution;
        let market_weight_value = get_market_weight_value(&market, &query)?;
        weighted_brier_sum +=
            market_weight_value * (market_resolved_value - market_predicted_value).powf(2.0);
        weighted_brier_count += market_weight_value;
    }
    Ok(weighted_brier_sum / weighted_brier_count)
}

/// Takes a set of markets and generates calibration plots for each.
pub fn build_accuracy_plot(
    query: Query<AccuracyQueryParams>,
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
) -> Result<HttpResponse, ApiError> {
    // get rng thread
    let mut rng = rand::thread_rng();
    // get markets from database
    let markets = get_markets_filtered(conn, Some(&query.filters), None)?;
    // sort by platform
    let markets_by_platform = categorize_markets_by_platform(markets);

    let mut traces = Vec::new();
    for (platform, market_list) in markets_by_platform {
        // get a set of random markets for the scatterplot
        let mut market_points = Vec::with_capacity(query.num_market_points);
        for market in market_list.choose_multiple(&mut rng, query.num_market_points) {
            market_points.push(Point {
                x: get_market_xaxis_value(&market, &query)?,
                y: get_market_brier_score(&market, &query)?,
                desc: market.title.clone(),
            })
        }

        // save it all to the trace and push it to result
        traces.push(Trace {
            platform: get_platform_by_name(conn, &platform)?,
            market_points,
        })
    }

    // sort the market lists by platform name so it's consistent
    traces.sort_unstable_by_key(|t| t.platform.name.clone());

    // get plot and axis titles
    let metadata = PlotMetadata {
        title: format!("Accuracy Plot"),
        x_title: get_x_axis_title(&query)?,
        y_title: get_y_axis_title(&query)?,
    };

    let response = AccuracyPlotResponse {
        query: query.into_inner(),
        metadata,
        traces,
    };

    Ok(HttpResponse::Ok().json(response))
}
