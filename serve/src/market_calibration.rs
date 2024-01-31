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

/// A quick and dirty f32 mask into u32 for key lookup.
/// Only needs to work for values between 0 and 1 at increaments of 0.01.
fn prob_to_k(f: &f32) -> i32 {
    (f * 1000.0) as i32
}

/// Inverse of `prob_to_k`.
fn k_to_prob(k: &i32) -> f32 {
    *k as f32 / 1000.0
}

/*/
/// Data for each bin and the markets included.
struct XAxisBin {
    start: f32,
    end: f32,
    weighted_y_values: f32,
    weighted_counts: f32,
}
*/

/// Generates a set of equally-spaced bins between 0 and 1.
/// The key corresponds to the middle of each bin (where it will be plotted on the chart).
/// All values here are given in "k", the value given by `prob_to_k`, so they can be used as keys.
fn generate_xaxis_bins(bin_size: &i32) -> Vec<i32> {
    // bin_size is the distance from the start to the end of the bin
    let mut bins: Vec<i32> = Vec::new();
    // we place the middle of the first bin such that it will start at 0
    // e.g. if the bin size is 0.05, the first bin should be 0.00 -> 0.05, which means the center is at 0.025
    let mut x = bin_size / 2;
    while x <= prob_to_k(&1.0) {
        bins.push(x);
        x += bin_size;
    }
    bins
}

/// Determine which bin the market belongs in based on the given x-value.
/// If a markets is on the line between two bins it will be sorted into the lower one.
fn find_xaxis_bin(x_value: f32, bins: &Vec<i32>, bin_size: i32) -> Result<i32, ApiError> {
    // convert the x-axis value into "k" to match the keys in the list
    let k_value = prob_to_k(&x_value);
    bins.iter()
        .find(|&bin_middle| {
            bin_middle - bin_size / 2 <= k_value && k_value <= bin_middle + bin_size / 2
        })
        .ok_or(ApiError::new(
            500,
            format!(
                "failed to find correct bin for {} in {:?} with lookaround {}",
                k_value,
                bins,
                bin_size / 2
            ),
        ))
        .copied()
}

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
    // generate x-axis bins
    let bin_size: i32 = prob_to_k(&query.calibration_bin_size);
    let bins = generate_xaxis_bins(&bin_size);
    let bin_count = bins.len();

    let mut traces = Vec::new();
    for (platform, market_list) in markets_by_platform {
        // build sums and counts to use as rolling averages
        let mut weighted_resolution_sums = HashMap::with_capacity(bin_count);
        let mut weighted_counts = HashMap::with_capacity(bin_count);
        // populate each map with x-values from bins
        for bin in bins.clone() {
            let hash_value = bin;
            weighted_resolution_sums.insert(hash_value, 0.0);
            weighted_counts.insert(hash_value, 0.0);
        }

        // get weighted average values for all markets
        // this is a hot loop since we iterate over all markets
        for market in market_list.iter() {
            // get specified market values
            let market_x_value = get_market_x_value(&market, &query.calibration_bin_method)?;
            let market_y_value = get_market_y_value(&market)?;
            let market_weight_value =
                get_market_weight_value(&market, &query.calibration_weight_attribute)?;

            // find the closest bin based on the market's selected x value
            let bin = find_xaxis_bin(market_x_value, &bins, bin_size)?;

            // add the market data to each counter
            *weighted_resolution_sums.get_mut(&bin).unwrap() +=
                market_weight_value * market_y_value;
            *weighted_counts.get_mut(&bin).unwrap() += market_weight_value;
        }

        // convert "k"s back into standard probabilities for the x-axis
        let x_series = bins.iter().map(k_to_prob).collect();
        // divide out the weighted values in each bin to get average y-values
        let y_series = bins
            .iter()
            .map(|bin| {
                // note that NaN is serialized as None, so if `count` is 0 the point won't be shown
                let sum = weighted_resolution_sums.get(bin).unwrap();
                let count = weighted_counts.get(bin).unwrap();
                sum / count
            })
            .collect();
        // get weighted value for the point size and scale it to fit the plot
        let point_weights = bins
            .iter()
            .map(|bin| *weighted_counts.get(bin).unwrap())
            .collect();
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
        x_title: match query.calibration_bin_method.as_str() {
            "prob_at_midpoint" => format!("Probability at Market Midpoint"),
            "prob_at_close" => format!("Probability at Market Close"),
            "prob_time_weighted" => format!("Market Time-Averaged Probability"),
            _ => panic!("given bin_method not in x_title map"),
        },
        y_title: match query.calibration_weight_attribute.as_str() {
            "open_days" => format!("Resolution, Weighted by Duration"),
            "num_traders" => format!("Resolution, Weighted by Traders"),
            "volume_usd" => format!("Resolution, Weighted by Volume"),
            "none" => format!("Resolution, Unweighted"),
            _ => panic!("given weight_attribute not in y_title map"),
        },
    };

    let response = PlotData { metadata, traces };

    Ok(HttpResponse::Ok().json(response))
}
