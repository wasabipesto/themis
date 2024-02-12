use super::*;

const NUM_ACCURACY_BINS: usize = 20;

/// Parameters passed to the accuracy function.
/// If the parameter is not supplied, the default values are used.
#[derive(Debug, Deserialize, Serialize)]
pub struct AccuracyQueryParams {
    #[serde(default = "default_scoring_attribute")]
    scoring_attribute: ScoringAttribute,
    #[serde(default = "default_xaxis_attribute")]
    xaxis_attribute: XAxisAttribute,
    #[serde(default = "default_num_market_points")]
    num_market_points: usize,
    #[serde(flatten)]
    pub filters: CommonFilterParams,
}
fn default_scoring_attribute() -> ScoringAttribute {
    ScoringAttribute::ProbAtMidpoint
}
fn default_xaxis_attribute() -> XAxisAttribute {
    XAxisAttribute::OpenDays
}
fn default_num_market_points() -> usize {
    1000
}

#[derive(Debug, Clone)]
/// Data for each bin and the markets included.
struct XAxisBin {
    start: f32,
    middle: f32,
    end: f32,
    brier_sum: f32,
    count: u32,
}

/// An individual datapoint to be plotted.
#[derive(Debug, Serialize)]
struct Point {
    x: f32,
    y: f32,
    point_title: Option<String>,
    point_label: String,
}

/// Data sent to the client to render a plot, one plot per platform.
#[derive(Debug, Serialize)]
struct Trace {
    platform: Platform,
    market_points: Vec<Point>,
    accuracy_line: Vec<Point>,
}

/// Metadata to help label a plot.
#[derive(Debug, Serialize)]
struct PlotMetadata {
    title: String,
    x_title: String,
    x_max: f32,
    y_title: String,
}

/// Full response for a plot.
#[derive(Debug, Serialize)]
struct AccuracyPlotResponse {
    query: AccuracyQueryParams,
    metadata: PlotMetadata,
    traces: Vec<Trace>,
}

/// A selector for how to score each market.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScoringAttribute {
    ProbAtMidpoint,
    ProbAtClose,
    ProbTimeAvg,
}
pub trait YAxisMethods {
    /// Get the value to use for the y-axis (brier score).
    fn get_y_value(&self, market: &Market) -> f32;
    /// Get the title to use for the y-axis.
    fn get_title(&self) -> String;
}
impl YAxisMethods for ScoringAttribute {
    fn get_y_value(&self, market: &Market) -> f32 {
        match self {
            ScoringAttribute::ProbAtMidpoint => {
                (market.resolution - market.prob_at_midpoint).powf(2.0)
            }
            ScoringAttribute::ProbAtClose => (market.resolution - market.prob_at_close).powf(2.0),
            ScoringAttribute::ProbTimeAvg => (market.resolution - market.prob_time_avg).powf(2.0),
        }
    }
    fn get_title(&self) -> String {
        match self {
            ScoringAttribute::ProbAtMidpoint => "Brier Score from Midpoint Probability".to_string(),
            ScoringAttribute::ProbAtClose => "Brier Score from Closing Probability".to_string(),
            ScoringAttribute::ProbTimeAvg => {
                "Brier Score from Time-Averaged Probability".to_string()
            }
        }
    }
}

/// A selector for the x-axis attribute to compare against.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum XAxisAttribute {
    OpenDays,
    VolumeUsd,
    NumTraders,
}
pub trait XAxisMethods {
    /// Get the value to use for the x-axis.
    fn get_x_value(&self, market: &Market) -> f32;
    /// Get the default maximum to use for the x-axis.
    fn get_default_max(&self) -> f32;
    /// Get the title to use for the x-axis.
    fn get_title(&self) -> String;
    /// Get the units to use for the x-axis.
    fn get_units(&self) -> String;
}
impl XAxisMethods for XAxisAttribute {
    fn get_x_value(&self, market: &Market) -> f32 {
        match self {
            XAxisAttribute::OpenDays => market.open_days,
            XAxisAttribute::VolumeUsd => market.volume_usd,
            XAxisAttribute::NumTraders => market.num_traders as f32,
        }
    }
    fn get_default_max(&self) -> f32 {
        match self {
            XAxisAttribute::OpenDays => 500.0,
            XAxisAttribute::VolumeUsd => 500.0,
            XAxisAttribute::NumTraders => 60.0,
        }
    }
    fn get_title(&self) -> String {
        match self {
            XAxisAttribute::OpenDays => "Market Open Length (days)".to_string(),
            XAxisAttribute::VolumeUsd => "Market Volume (USD)".to_string(),
            XAxisAttribute::NumTraders => "Number of Unique Traders".to_string(),
        }
    }
    fn get_units(&self) -> String {
        match self {
            XAxisAttribute::OpenDays => "days".to_string(),
            XAxisAttribute::VolumeUsd => "USD".to_string(),
            XAxisAttribute::NumTraders => "traders".to_string(),
        }
    }
}

/// Generate `count` equally-spaced bins from 0 to `max`
/// The first bin is from 0 to `step` and the last one is from `max`-`step` to `max`.
fn generate_xaxis_bins(max: f32, count: usize) -> Result<Vec<XAxisBin>, ApiError> {
    let step = max / count as f32;
    let mut bins = Vec::with_capacity(count);
    for i in 0..count {
        let start = i as f32 * step;
        let end = (i as f32 + 1.0) * step;
        let middle = (start + end) / 2.0;
        bins.push(XAxisBin {
            start,
            middle,
            end,
            brier_sum: 0.0,
            count: 0,
        });
    }
    Ok(bins)
}

/// Takes a set of markets and generates calibration plots for each.
pub fn build_accuracy_plot(
    query: Query<AccuracyQueryParams>,
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
) -> Result<HttpResponse, ApiError> {
    // get rng thread
    let mut rng = rand::thread_rng();

    // get markets from database
    let (markets, _) = get_markets_filtered(conn, Some(&query.filters), None)?;
    // sort by platform
    let markets_by_platform = categorize_markets_by_platform(markets.clone());

    // get maximum value for x-axis bins
    let default_maximum = query.xaxis_attribute.get_default_max();
    let bin_maximum = markets
        .iter()
        .map(|market| query.xaxis_attribute.get_x_value(market))
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .ok_or_else(|| ApiError {
            status_code: 500,
            message: format!(
                "Failed to get maximum value in column {:?}",
                query.xaxis_attribute
            ),
        })?
        .min(default_maximum);
    // generate bins for accuracy measurement
    let bins_orig = generate_xaxis_bins(bin_maximum, NUM_ACCURACY_BINS)?;

    let mut traces = Vec::new();
    for (platform_name, market_list) in markets_by_platform {
        // get platform info
        let platform = get_platform_by_name(conn, &platform_name)?;

        // clone bins
        let mut bins = bins_orig.clone();

        // get a set of random markets for the scatterplot
        // we get the requested amount plus a few so we can filter out some outliers
        let random_markets = market_list.choose_multiple(&mut rng, query.num_market_points);
        let mut market_points = Vec::with_capacity(query.num_market_points);
        for market in random_markets {
            market_points.push(Point {
                x: query.xaxis_attribute.get_x_value(market),
                y: query.scoring_attribute.get_y_value(market),
                point_title: None,
                point_label: format!("{}: {}", platform.name_fmt.clone(), market.title.clone()),
            })
        }
        // sort by x ascending for easier rendering (remove?)
        market_points.sort_by(|a, b| {
            a.x.partial_cmp(&b.x)
                .expect("Failed to compare values (NaN?)")
        });

        // calculate brier scores for each market
        // this is a hot loop since we iterate over all markets
        for market in market_list.iter() {
            // find the closest bin based on the market's selected x value
            let market_xaxis_value = query.xaxis_attribute.get_x_value(market);
            let bin_opt = bins
                .iter_mut()
                .find(|bin| bin.start <= market_xaxis_value && market_xaxis_value <= bin.end);

            // if it's in our range, calculate and save
            if let Some(bin) = bin_opt {
                bin.brier_sum += query.scoring_attribute.get_y_value(market);
                bin.count += 1;
            }
        }

        // get the final result per bin
        let accuracy_line = bins
            .iter()
            .map(|bin| {
                let brier_score = bin.brier_sum / bin.count as f32;
                Point {
                    x: bin.middle,
                    y: brier_score,
                    point_title: Some(format!(
                        "{} to {} {}",
                        bin.start,
                        bin.end,
                        query.xaxis_attribute.get_units()
                    )),
                    point_label: format!(
                        "{} Score: {:.04} from {} markets",
                        platform.name_fmt.clone(),
                        brier_score,
                        bin.count
                    ),
                }
            })
            .collect();

        // save it all to the trace and push it to result
        traces.push(Trace {
            platform,
            market_points,
            accuracy_line,
        })
    }

    // sort the market lists by platform name so it's consistent
    traces.sort_unstable_by_key(|t| t.platform.name.clone());

    // get plot and axis titles
    let metadata = PlotMetadata {
        title: "Accuracy Plot".to_string(),
        x_title: query.xaxis_attribute.get_title(),
        x_max: bin_maximum,
        y_title: query.scoring_attribute.get_title(),
    };

    let response = AccuracyPlotResponse {
        query: query.into_inner(),
        metadata,
        traces,
    };

    Ok(HttpResponse::Ok().json(response))
}
