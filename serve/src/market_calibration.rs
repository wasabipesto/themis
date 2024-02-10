use super::*;

/// Parameters passed to the calibration function.
/// If the parameter is not supplied, the default values are used.
/// TODO: Change bin_attribute and weight_attribute to enums
#[derive(Debug, Deserialize, Serialize)]
pub struct CalibrationQueryParams {
    #[serde(default = "default_bin_attribute")]
    bin_attribute: String,
    #[serde(default = "default_bin_size")]
    bin_size: f32,
    #[serde(default = "default_weight_attribute")]
    weight_attribute: String,
    #[serde(flatten)]
    pub filters: CommonFilterParams,
}
fn default_bin_attribute() -> String {
    "prob_at_midpoint".to_string()
}
fn default_bin_size() -> f32 {
    0.05
}
fn default_weight_attribute() -> String {
    "none".to_string()
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
    r: f32,
    //desc: String,
}

/// Data sent to the client to render a plot, one plot per platform.
#[derive(Debug, Serialize)]
struct Trace {
    platform: Platform,
    points: Vec<Point>,
}

/// Metadata to help label a plot.
#[derive(Debug, Serialize)]
struct PlotMetadata {
    title: String,
    x_title: String,
    y_title: String,
}

/// Full response for a calibration plot.
#[derive(Debug, Serialize)]
struct CalibrationPlotResponse {
    query: CalibrationQueryParams,
    metadata: PlotMetadata,
    traces: Vec<Trace>,
}

/// Generates a set of equally-spaced bins between 0 and 1, where `bin_size` is the width of each bin.
fn generate_xaxis_bins(bin_size: &f32) -> Result<Vec<XAxisBin>, ApiError> {
    let mut bins: Vec<XAxisBin> = Vec::new();
    let mut x: f32 = 0.0;
    while x <= 1.0 {
        bins.push(XAxisBin {
            start: x,
            middle: x + bin_size / 2.0,
            end: x + bin_size,
            y_axis_numerator: 0.0,
            y_axis_denominator: 0.0,
        });
        x += bin_size;
    }
    Ok(bins)
}

/// Get the x-value of the market, based on the user-defined bin attribute.
fn get_market_x_value(market: &Market, bin_attribute: &String) -> Result<f32, ApiError> {
    match bin_attribute.as_str() {
        "prob_at_midpoint" => Ok(market.prob_at_midpoint),
        "prob_at_close" => Ok(market.prob_at_close),
        "prob_time_avg" => Ok(market.prob_time_avg),
        _ => {
            return Err(ApiError::new(
                400,
                "the value provided for `bin_attribute` is not a valid option".to_string(),
            ))
        }
    }
}

/// Get the y-value of the market, which is always the resolution value.
fn get_market_y_value(market: &Market) -> Result<f32, ApiError> {
    Ok(market.resolution)
}

/// Get the weighting value of the market, based on the user-defined weighting attribute.
fn get_market_weight_value(market: &Market, weight_attribute: &String) -> Result<f32, ApiError> {
    match weight_attribute.as_str() {
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
fn get_x_axis_title(bin_attribute: &String) -> Result<String, ApiError> {
    match bin_attribute.as_str() {
        "prob_at_midpoint" => Ok(format!("Probability at Market Midpoint")),
        "prob_at_close" => Ok(format!("Probability at Market Close")),
        "prob_time_avg" => Ok(format!("Market Time-Averaged Probability")),
        _ => Err(ApiError {
            status_code: 500,
            message: format!("given bin_attribute not in x_title map"),
        }),
    }
}

/// Get the y-axis title of the plot, based on the user-defined weight attribute.
fn get_y_axis_title(weight_attribute: &String) -> Result<String, ApiError> {
    match weight_attribute.as_str() {
        "open_days" => Ok(format!("Resolution, Weighted by Duration")),
        "num_traders" => Ok(format!("Resolution, Weighted by Traders")),
        "volume_usd" => Ok(format!("Resolution, Weighted by Volume")),
        "none" => Ok(format!("Resolution, Unweighted")),
        _ => Err(ApiError {
            status_code: 500,
            message: format!("given weight_attribute not in y_title map"),
        }),
    }
}

/// Takes a set of markets and generates calibration plots for each.
pub fn build_calibration_plot(
    query: Query<CalibrationQueryParams>,
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
) -> Result<HttpResponse, ApiError> {
    // get markets from database
    let markets = get_markets_filtered(conn, Some(&query.filters), None)?;
    // sort by platform
    let markets_by_platform = categorize_markets_by_platform(markets);

    let mut traces = Vec::new();
    for (platform, market_list) in markets_by_platform {
        // generate x-axis bins
        let mut bins = generate_xaxis_bins(&query.bin_size)?;

        // get weighted average values for all markets
        // this is a hot loop since we iterate over all markets
        for market in market_list.iter() {
            // get specified market values
            let market_x_value = get_market_x_value(&market, &query.bin_attribute)?;
            let market_y_value = get_market_y_value(&market)?;
            let market_weight_value = get_market_weight_value(&market, &query.weight_attribute)?;

            // find the closest bin based on the market's selected x value
            let bin = bins
                .iter_mut()
                .find(|bin| bin.start <= market_x_value && market_x_value <= bin.end)
                .ok_or(ApiError::new(
                    500,
                    format!(
                        "failed to find correct bin for {market_x_value} with bin size {}",
                        &query.bin_size
                    ),
                ))?;

            // add the market data to each counter
            bin.y_axis_numerator += market_weight_value * market_y_value;
            bin.y_axis_denominator += market_weight_value;
        }

        let denominator_list = bins.iter().map(|bin| bin.y_axis_denominator).collect();
        let scale_params = get_scale_params(
            denominator_list,
            POINT_SIZE_MIN,
            POINT_SIZE_MAX,
            POINT_SIZE_DEFAULT,
        );
        let points = bins
            .iter()
            .map(|bin| Point {
                x: bin.middle,
                y: bin.y_axis_numerator / bin.y_axis_denominator,
                r: scale_data_point(bin.y_axis_denominator, scale_params.clone()),
                //desc: format!(),
            })
            .collect();

        // save it all to the trace and push it to result
        traces.push(Trace {
            platform: get_platform_by_name(conn, &platform)?,
            points,
        })
    }

    // sort the market lists by platform name so it's consistent
    traces.sort_unstable_by_key(|t| t.platform.name.clone());

    // get plot and axis titles
    let metadata = PlotMetadata {
        title: format!("Calibration Plot"),
        x_title: get_x_axis_title(&query.bin_attribute)?,
        y_title: get_y_axis_title(&query.weight_attribute)?,
    };

    let response = CalibrationPlotResponse {
        query: query.into_inner(),
        metadata,
        traces,
    };

    Ok(HttpResponse::Ok().json(response))
}
