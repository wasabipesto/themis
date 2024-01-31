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

/// Sort all markets into Vecs based on the platform name.
fn categorize_markets_by_platform(markets: Vec<Market>) -> HashMap<String, Vec<Market>> {
    let mut markets_by_platform: HashMap<String, Vec<Market>> = HashMap::new();
    for market in markets {
        // this is a hot loop since we iterate over all markets
        if let Some(market_list) = markets_by_platform.get_mut(&market.platform) {
            market_list.push(market);
        } else {
            markets_by_platform.insert(market.platform.clone(), Vec::from([market]));
        }
    }
    markets_by_platform
}

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

/// Converts
pub fn build_calibration_plot(
    query: Query<CalibrationQueryParams>,
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    markets: Vec<Market>,
) -> Result<HttpResponse, ApiError> {
    // sort all markets based on the platform
    let markets_by_platform = categorize_markets_by_platform(markets);

    // generate x-axis bins
    let bin_size: i32 = prob_to_k(&query.calibration_bin_size);
    let bins = generate_xaxis_bins(&bin_size);

    let mut traces = Vec::new();
    for (platform, market_list) in markets_by_platform {
        // build sums and counts to use as rolling averages
        let mut weighted_resolution_sums = HashMap::with_capacity(bins.len());
        let mut weighted_counts = HashMap::with_capacity(bins.len());
        // populate each map with x-values from bins
        for bin in bins.clone() {
            let hash_value = bin;
            weighted_resolution_sums.insert(hash_value, 0.0);
            weighted_counts.insert(hash_value, 0.0);
        }

        // set up brier counters
        let mut weighted_brier_sum: f32 = 0.0;
        let mut weighted_brier_count: f32 = 0.0;

        // get weighted average values for all markets
        // this is a hot loop since we iterate over all markets
        for market in market_list.iter() {
            // find the closest bin based on the market's selected x value
            let market_x_value = match query.calibration_bin_method.as_str() {
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
            if market_x_value.is_nan() {
                return Err(ApiError {
                    status_code: 500,
                    message: format!("Market X-Value is NaN: {:?}", market),
                });
            };
            let market_k = prob_to_k(&market_x_value);
            let bin_opt = bins
                .iter()
                .find(|&x| x - bin_size / 2 <= market_k && market_k <= x + bin_size / 2);
            let bin = match bin_opt {
                Some(bin) => bin,
                None => {
                    eprintln!(
                        "failed to find correct bin for {} in {:?} with lookaround {}",
                        market_k,
                        bins,
                        bin_size / 2
                    );
                    continue;
                }
            };

            // get the weighting value
            let market_y_value = market.resolution;
            if market_y_value.is_nan() {
                return Err(ApiError {
                    status_code: 500,
                    message: format!("Market Y-Value is NaN: {:?}", market),
                });
            };
            let weight: f32 = match query.calibration_weight_attribute.as_str() {
                "open_days" => market.open_days,
                "num_traders" => market.num_traders as f32,
                "volume_usd" => market.volume_usd,
                "none" => 1.0,
                _ => {
                    return Err(ApiError::new(
                        400,
                        "the value provided for `weight_attribute` is not a valid option"
                            .to_string(),
                    ))
                }
            };

            // add the market data to each counter
            *weighted_resolution_sums.get_mut(bin).unwrap() += weight * market_y_value;
            *weighted_counts.get_mut(bin).unwrap() += weight;
            weighted_brier_sum += weight * (market_y_value - market_x_value).powf(2.0);
            weighted_brier_count += weight;
        }

        // divide out rolling averages into a single average value
        let x_series = bins.iter().map(k_to_prob).collect();
        let y_series = bins
            .iter()
            .map(|bin| {
                // note that NaN is serialized as None, so if `count` is 0 the point won't be shown
                let sum = weighted_resolution_sums.get(bin).unwrap();
                let count = weighted_counts.get(bin).unwrap();
                sum / count
            })
            .collect();
        let point_weights = bins
            .iter()
            .map(|bin| *weighted_counts.get(bin).unwrap())
            .collect();
        let point_sizes = scale_list(point_weights, 8.0, 32.0, 10.0);

        // get cached platform info from database
        let platform_info = get_platform_by_name(conn, &platform)?;

        // save it all to the trace and push it to result
        traces.push(Trace {
            platform_name_fmt: platform_info.platform_name_fmt,
            platform_description: platform_info.platform_description,
            platform_avatar_url: platform_info.platform_avatar_url,
            platform_color: platform_info.platform_color,
            num_markets: market_list.len(),
            brier_score: weighted_brier_sum / weighted_brier_count,
            x_series,
            y_series,
            point_sizes,
        })
    }

    traces.sort_unstable_by_key(|t| t.platform_name_fmt.clone());

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
