use super::*;

/// Parameters passed to the calibration function.
/// If the parameter is not supplied, the default values are used.
/// TODO: Change calibration_bin_method and calibration_weight_attribute to enums
#[derive(Debug, Deserialize)]
pub struct CalibrationQueryParams {
    #[serde(default = "default_calibration_bin_method")]
    pub calibration_bin_method: String,
    #[serde(default = "default_calibration_bin_size")]
    pub calibration_bin_size: f32,
    #[serde(default = "default_calibration_weight_attribute")]
    pub calibration_weight_attribute: String,
    #[serde(flatten)]
    pub filters: CommonFilterParams,
}
fn default_calibration_bin_method() -> String {
    "prob_time_weighted".to_string()
}
fn default_calibration_bin_size() -> f32 {
    0.05
}
fn default_calibration_weight_attribute() -> String {
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

/// Generates a set of equally-spaced bins between 0 and 1, where `bin_size` is the width of each bin.
fn generate_xaxis_bins(bin_size: &f32) -> Vec<XAxisBin> {
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
    bins
}

/// Get the x-value of the market, based on the user-defined bin method.
/// Also checks to make sure the value is not NaN.
fn get_market_x_value(market: &Market, bin_method: &String) -> Result<f32, ApiError> {
    let value = match bin_method.as_str() {
        "prob_at_midpoint" => market.prob_at_midpoint,
        "prob_at_close" => market.prob_at_close,
        "prob_time_weighted" => market.prob_time_weighted,
        _ => {
            return Err(ApiError::new(
                400,
                "the value provided for `bin_method` is not a valid option".to_string(),
            ))
        }
    };
    if value.is_nan() {
        Err(ApiError {
            status_code: 500,
            message: format!("Market X-Value ({bin_method}) is NaN: {:?}", market),
        })
    } else {
        Ok(value)
    }
}

/// Get the x-axis title of the plot, based on the user-defined bin method.
fn get_x_axis_title(bin_method: &String) -> Result<String, ApiError> {
    match bin_method.as_str() {
        "prob_at_midpoint" => Ok(format!("Probability at Market Midpoint")),
        "prob_at_close" => Ok(format!("Probability at Market Close")),
        "prob_time_weighted" => Ok(format!("Market Time-Averaged Probability")),
        _ => Err(ApiError {
            status_code: 500,
            message: format!("given bin_method not in x_title map"),
        }),
    }
}

/// Get the x-axis title of the plot, based on the user-defined bin method.
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
/// Get the y-value of the market, which is always the resolution value.
/// Also checks to make sure the value is not NaN.
fn get_market_y_value(market: &Market) -> Result<f32, ApiError> {
    let value = market.resolution;
    if value.is_nan() {
        Err(ApiError {
            status_code: 500,
            message: format!("Market Y-Value is NaN: {:?}", market),
        })
    } else {
        Ok(value)
    }
}

/// Get the weighting value of the market, based on the user-defined weighting method.
/// Also checks to make sure the value is not NaN.
fn get_market_weight_value(market: &Market, weight_attribute: &String) -> Result<f32, ApiError> {
    let value = match weight_attribute.as_str() {
        "open_days" => market.open_days,
        "num_traders" => market.num_traders as f32,
        "volume_usd" => market.volume_usd,
        "none" => 1.0,
        _ => {
            return Err(ApiError::new(
                400,
                "the value provided for `weight_attribute` is not a valid option".to_string(),
            ))
        }
    };
    if value.is_nan() {
        Err(ApiError {
            status_code: 500,
            message: format!("Market Weight ({weight_attribute}) is NaN: {:?}", market),
        })
    } else {
        Ok(value)
    }
}

/// Takes a set of markets and generates a brier score.
#[allow(dead_code)]
pub fn calculate_brier_score(
    query: Query<CalibrationQueryParams>,
    markets: Vec<Market>,
) -> Result<f32, ApiError> {
    // set up brier counters
    let mut weighted_brier_sum: f32 = 0.0;
    let mut weighted_brier_count: f32 = 0.0;

    // this is a hot loop since we iterate over all markets
    for market in markets {
        let market_x_value = get_market_x_value(&market, &query.calibration_bin_method)?;
        let market_y_value = get_market_y_value(&market)?;
        let market_weight_value =
            get_market_weight_value(&market, &query.calibration_weight_attribute)?;
        weighted_brier_sum += market_weight_value * (market_y_value - market_x_value).powf(2.0);
        weighted_brier_count += market_weight_value;
    }
    Ok(weighted_brier_sum / weighted_brier_count)
}

/// Takes a set of markets and generates calibration plots for each.
pub fn build_calibration_plot(
    query: Query<CalibrationQueryParams>,
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    markets_by_platform: HashMap<String, Vec<Market>>,
) -> Result<HttpResponse, ApiError> {
    let mut traces = Vec::new();
    for (platform, market_list) in markets_by_platform {
        // generate x-axis bins
        let mut bins = generate_xaxis_bins(&query.calibration_bin_size);

        // get weighted average values for all markets
        // this is a hot loop since we iterate over all markets
        for market in market_list.iter() {
            // get specified market values
            let market_x_value = get_market_x_value(&market, &query.calibration_bin_method)?;
            let market_y_value = get_market_y_value(&market)?;
            let market_weight_value =
                get_market_weight_value(&market, &query.calibration_weight_attribute)?;

            // find the closest bin based on the market's selected x value
            let bin = bins
                .iter_mut()
                .find(|bin| bin.start <= market_x_value && market_x_value <= bin.end)
                .ok_or(ApiError::new(
                    500,
                    format!(
                        "failed to find correct bin for {market_x_value} with bin size {}",
                        &query.calibration_bin_size
                    ),
                ))?;

            // add the market data to each counter
            bin.y_axis_numerator += market_weight_value * market_y_value;
            bin.y_axis_denominator += market_weight_value;
        }

        // convert "k"s back into standard probabilities for the x-axis
        let x_series = bins.iter().map(|bin| bin.middle).collect();
        // divide out the weighted values in each bin to get average y-values
        // note that NaN is serialized as None, so if `denominator` is 0 the point won't be shown
        let y_series = bins
            .iter()
            .map(|bin| bin.y_axis_numerator / bin.y_axis_denominator)
            .collect();
        // get weighted value for the point size and scale it to fit the plot
        let point_weights = bins.iter().map(|bin| bin.y_axis_denominator).collect();
        let point_sizes = scale_list(point_weights, 8.0, 32.0, 10.0);

        // get cached platform info from database
        let platform_info = get_platform_by_name(conn, &platform)?;

        // save it all to the trace and push it to result
        traces.push(Trace {
            platform: platform_info,
            num_markets: market_list.len(),
            x_series,
            y_series,
            point_sizes,
        })
    }

    // sort the market lists by platform name so it's consistent
    traces.sort_unstable_by_key(|t| t.platform.platform_name_fmt.clone());

    // get plot and axis titles
    let metadata = PlotMetadata {
        title: format!("Calibration Plot"),
        x_title: get_x_axis_title(&query.calibration_bin_method)?,
        y_title: get_y_axis_title(&query.calibration_weight_attribute)?,
    };

    let response = PlotData { metadata, traces };

    Ok(HttpResponse::Ok().json(response))
}
