use super::*;

const POINT_SIZE_MIN: f32 = 6.0;
const POINT_SIZE_MAX: f32 = 28.0;
const POINT_SIZE_DEFAULT: f32 = 8.0;

/// Parameters passed to the calibration function.
/// If the parameter is not supplied, the default values are used.
#[derive(Debug, Deserialize, Serialize)]
pub struct CalibrationQueryParams {
    #[serde(default = "default_bin_attribute")]
    bin_attribute: BinAttribute,
    #[serde(default = "default_bin_size")]
    bin_size: f32,
    #[serde(default = "default_weight_attribute")]
    weight_attribute: WeightAttribute,
    #[serde(flatten)]
    pub filters: CommonFilterParams,
}
fn default_bin_attribute() -> BinAttribute {
    BinAttribute::ProbAtMidpoint
}
fn default_bin_size() -> f32 {
    0.05
}
fn default_weight_attribute() -> WeightAttribute {
    WeightAttribute::None
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
    point_title: String,
    point_label: String,
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

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BinAttribute {
    ProbAtMidpoint,
    ProbAtClose,
    ProbTimeAvg,
}

pub trait XAxisMethods {
    /// Get the value to use for the x-axis (bin).
    fn get_x_value(&self, market: &Market) -> f32;
    /// Get the title to use for the y-axis.
    fn get_title(&self) -> String;
}
impl XAxisMethods for BinAttribute {
    fn get_x_value(&self, market: &Market) -> f32 {
        match self {
            BinAttribute::ProbAtMidpoint => market.prob_at_midpoint,
            BinAttribute::ProbAtClose => market.prob_at_close,
            BinAttribute::ProbTimeAvg => market.prob_time_avg,
        }
    }
    fn get_title(&self) -> String {
        match self {
            BinAttribute::ProbAtMidpoint => "Probability at Market Midpoint".to_string(),
            BinAttribute::ProbAtClose => "Probability at Market Close".to_string(),
            BinAttribute::ProbTimeAvg => "Market Time-Averaged Probability".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WeightAttribute {
    None,
    OpenDays,
    VolumeUsd,
    NumTraders,
}

pub trait YAxisMethods {
    /// Get the value to use for the y-axis (resolution).
    fn get_y_value(&self, market: &Market) -> f32;
    /// Get the weight to use for the y-axis.
    fn get_weight(&self, market: &Market) -> f32;
    /// Get the title to use for the y-axis.
    fn get_title(&self) -> String;
}
impl YAxisMethods for WeightAttribute {
    fn get_y_value(&self, market: &Market) -> f32 {
        market.resolution
    }
    fn get_weight(&self, market: &Market) -> f32 {
        match self {
            WeightAttribute::None => 1.0,
            WeightAttribute::OpenDays => market.open_days,
            WeightAttribute::VolumeUsd => market.volume_usd,
            WeightAttribute::NumTraders => market.num_traders as f32,
        }
    }
    fn get_title(&self) -> String {
        match self {
            WeightAttribute::None => "Resolution, Unweighted".to_string(),
            WeightAttribute::OpenDays => "Resolution, Weighted by Duration".to_string(),
            WeightAttribute::VolumeUsd => "Resolution, Weighted by Traders".to_string(),
            WeightAttribute::NumTraders => "Resolution, Weighted by Volume".to_string(),
        }
    }
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
            let market_x_value = query.bin_attribute.get_x_value(market);
            let market_y_value = query.weight_attribute.get_y_value(market);
            let market_weight_value = query.weight_attribute.get_weight(market);

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

        // get platform data
        let platform = get_platform_by_name(conn, &platform)?;

        // scale and save the data
        let denominator_list = bins.iter().map(|bin| bin.y_axis_denominator).collect();
        let scale_params = get_scale_params(
            denominator_list,
            POINT_SIZE_MIN,
            POINT_SIZE_MAX,
            POINT_SIZE_DEFAULT,
        );
        let points = bins
            .iter()
            .map(|bin| {
                let y_value = bin.y_axis_numerator / bin.y_axis_denominator;
                Point {
                    x: bin.middle,
                    y: y_value,
                    r: scale_data_point(bin.y_axis_denominator, scale_params.clone()),
                    point_title: format!(
                        "Predicted: {:.0} to {:.0}%",
                        bin.start * 100.0,
                        bin.end * 100.0
                    ),
                    point_label: format!("{}: {:.1}%", platform.name_fmt, y_value * 100.0,),
                }
            })
            .collect();

        // save it all to the trace and push it to result
        traces.push(Trace { platform, points })
    }

    // sort the market lists by platform name so it's consistent
    traces.sort_unstable_by_key(|t| t.platform.name.clone());

    // get plot and axis titles
    let metadata = PlotMetadata {
        title: format!("Calibration Plot"),
        x_title: query.bin_attribute.get_title(),
        y_title: query.bin_attribute.get_title(),
    };

    let response = CalibrationPlotResponse {
        query: query.into_inner(),
        metadata,
        traces,
    };

    Ok(HttpResponse::Ok().json(response))
}
