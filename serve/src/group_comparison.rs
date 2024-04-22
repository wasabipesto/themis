use super::*;

type PlatformKey = String;
type DateKey = String;
type CategoryKey = String;

/// Structure for deserialization from config file.
#[derive(Serialize, Deserialize, Debug)]
struct InputMarketData {
    platform: String,
    platform_id: String,
    invert: Option<bool>,
}

/// Structure for deserialization from config file.
#[derive(Serialize, Deserialize, Debug)]
struct InputGroupData {
    title: String,
    category: String,
    markets: Vec<InputMarketData>,
}

/// Structure for serialization for response.
#[derive(Serialize, Debug, Clone)]
struct ResponseMarketData {
    market_data: Market,
    platform: String,
    absolute_brier: f32,
    relative_brier: f32,
}

/// Structure for serialization for response.
#[derive(Serialize, Debug, Clone)]
struct ResponseGroupData {
    group_title: String,
    category: String,
    markets: Vec<ResponseMarketData>,
}

/// Structure for serialization for response.
#[derive(Serialize, Debug)]
struct ResponsePlatformStats {
    platform: String,
    category: String,
    /// The mean absolute_brier of all markets in sample.
    platform_absolute_brier: Option<f32>,
    /// The mean relative_brier of all markets in sample.
    platform_relative_brier: Option<f32>,
    /// The percent of groups in the sample where this platform is represented.
    platform_sample_presence: f32,
}

/// Structure for serialization for response (top-level).
#[derive(Serialize, Debug)]
struct FullResponse {
    platform_metadata: Vec<Platform>,
    platform_stats: Vec<ResponsePlatformStats>,
    groups: Vec<ResponseGroupData>,
}

/// Gets a list of all dates where 2 or more markets were open.
/// Used to calculate the absolute Brier score.
fn get_dates_for_absolute_scoring(markets: &HashMap<String, Market>) -> Vec<DateKey> {
    let mut date_set: HashSet<DateKey> = HashSet::new();
    for market in markets.values() {
        for date in market.prob_each_date.as_object().unwrap().keys() {
            date_set.insert(date.to_string());
        }
    }
    let mut date_vec: Vec<DateKey> = Vec::new();
    for date in date_set {
        if markets
            .values()
            .filter(|m| m.prob_each_date.as_object().unwrap().contains_key(&date))
            .count()
            >= 2
        {
            date_vec.push(date);
        }
    }
    date_vec
}

/// Gets a list of all dates where ALL markets were open.
/// Used to calculate the relative Brier score.
fn get_dates_for_relative_scoring(markets: &HashMap<String, Market>) -> Vec<DateKey> {
    let mut date_set: HashSet<DateKey> = HashSet::new();
    for market in markets.values() {
        for date in market.prob_each_date.as_object().unwrap().keys() {
            date_set.insert(date.to_string());
        }
    }
    let mut date_vec: Vec<DateKey> = Vec::new();
    for date in date_set {
        if markets
            .values()
            .all(|m| m.prob_each_date.as_object().unwrap().contains_key(&date))
        {
            date_vec.push(date);
        }
    }
    date_vec
}

/// Extract the unique platform names from a list of groups.
fn get_unique_platforms_from_groups(groups: &Vec<ResponseGroupData>) -> Vec<PlatformKey> {
    let mut set: HashSet<String> = HashSet::new();
    for group in groups {
        for market in &group.markets {
            set.insert(market.platform.clone());
        }
    }
    set.into_iter().collect()
}

/// Extract the unique category names from a list of groups.
fn get_unique_categories_from_groups(groups: &Vec<ResponseGroupData>) -> Vec<CategoryKey> {
    let mut set: HashSet<String> = HashSet::new();
    for group in groups {
        set.insert(group.category.clone());
    }
    set.into_iter().collect()
}

/// Save a score to a map in the form of {platform: {date: score}}.
/// Errors if a duplicate date is given.
fn save_score_to_nested_map(
    score_data: &mut HashMap<PlatformKey, HashMap<DateKey, f32>>,
    platform: &PlatformKey,
    date: &DateKey,
    score: f32,
) -> Result<(), ApiError> {
    match score_data.get_mut(platform) {
        None => {
            score_data.insert(platform.clone(), HashMap::from([(date.to_owned(), score)]));
            Ok(())
        }
        Some(subdata) => match subdata.get(date) {
            None => {
                subdata.insert(date.clone(), score);
                Ok(())
            }
            Some(_) => Err(ApiError {
                status_code: 500,
                message: "date already in map".to_owned(),
            }),
        },
    }
}

/// Gets a score from a map in the form of {platform: {date: score}}.
fn get_score_from_nested_map(
    score_data: &HashMap<PlatformKey, HashMap<DateKey, f32>>,
    platform: &PlatformKey,
    date: &DateKey,
) -> Result<f32, ApiError> {
    Ok(*score_data.get(platform).unwrap().get(date).unwrap())
}

/// Aggregate data from a list of groups.
/// The result is a list where each item represents all markets in a platform.
fn get_platform_aggregate_stats(
    groups: &Vec<ResponseGroupData>,
    category: String,
) -> Vec<ResponsePlatformStats> {
    // filter out the groups we want
    let category_groups: Vec<ResponseGroupData> = match category.as_str() {
        "All" => groups.clone(),
        _ => groups
            .clone()
            .into_iter()
            .filter(|g| g.category == category)
            .collect(),
    };
    let total_count = category_groups.len();

    // set up the counters
    struct PlatformStatsIntermediate {
        cumulative_absolute_brier: f32,
        cumulative_relative_brier: f32,
        count: usize,
    }
    let mut platform_stat_intermediates: HashMap<String, PlatformStatsIntermediate> =
        HashMap::new();
    for group in category_groups {
        for market in group.markets {
            let platform_name = market.platform.clone();
            // add new counter or update existing
            match platform_stat_intermediates.get_mut(&platform_name) {
                None => {
                    platform_stat_intermediates.insert(
                        platform_name,
                        PlatformStatsIntermediate {
                            cumulative_absolute_brier: market.absolute_brier,
                            cumulative_relative_brier: market.relative_brier,
                            count: 1,
                        },
                    );
                }
                Some(psi) => {
                    psi.cumulative_absolute_brier += market.absolute_brier;
                    psi.cumulative_relative_brier += market.relative_brier;
                    psi.count += 1;
                }
            }
        }
    }

    // divide out into averages
    let mut platform_stats = Vec::new();
    for (platform_name, psi) in platform_stat_intermediates {
        platform_stats.push(ResponsePlatformStats {
            platform: platform_name,
            category: category.clone(),
            // TODO: set scores to none if presence < 10%
            platform_absolute_brier: Some(psi.cumulative_absolute_brier / psi.count as f32),
            platform_relative_brier: Some(psi.cumulative_relative_brier / psi.count as f32),
            platform_sample_presence: psi.count as f32 / total_count as f32,
        })
    }
    platform_stats
}

/// Take data from a group mapping file, grab the relevant markets, and get
/// their brier scores over time. Also compare their scores to see which
/// platforms were more accurate over time.
pub fn build_group_comparison(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
) -> Result<HttpResponse, ApiError> {
    // load group data from the file
    let config_file = File::open("groups.yaml")
        .map_err(|e| ApiError::new(500, format!("failed to load config file: {e}")))?;
    let config_file_groups: Vec<InputGroupData> = serde_yaml::from_reader(config_file)
        .map_err(|e| ApiError::new(500, format!("failed to parse config file: {e}")))?;

    // go through each group & constituent market
    let mut groups = Vec::with_capacity(config_file_groups.len());
    for group in config_file_groups {
        // get market data from db
        let mut markets_by_platform: HashMap<String, Market> =
            HashMap::with_capacity(group.markets.len());
        for market in group.markets {
            let market_data =
                get_market_by_platform_id(conn, &market.platform, &market.platform_id)?;
            markets_by_platform.insert(market.platform, market_data);
        }

        // get absolute brier per day on each market
        let dates_for_absolute_scoring = get_dates_for_absolute_scoring(&markets_by_platform);
        let mut absolute_score_data: HashMap<PlatformKey, HashMap<DateKey, f32>> = HashMap::new();
        for (platform, market) in &markets_by_platform {
            for date in &dates_for_absolute_scoring {
                // calculate brier for the day
                let resolution = market.resolution.clone();
                let prediction = market.prob_each_date.get(date).unwrap().as_f64().unwrap() as f32;
                let absolute_brier = (resolution - prediction).powi(2);
                // save it to map
                save_score_to_nested_map(&mut absolute_score_data, platform, date, absolute_brier)?;
            }
        }

        // get median brier per day
        for date in &dates_for_absolute_scoring {
            let platform = &"median".to_string();
            let mut median_brier_acc = 0.0;
            let mut median_brier_len = 0;
            for (_, date_map) in &absolute_score_data {
                if let Some(brier) = date_map.get(date) {
                    median_brier_acc += brier;
                    median_brier_len += 1;
                }
            }
            let median_brier = median_brier_acc / median_brier_len as f32;
            save_score_to_nested_map(&mut absolute_score_data, platform, date, median_brier)?;
        }

        // get relative brier per day on each market
        let dates_for_relative_scoring = get_dates_for_relative_scoring(&markets_by_platform);
        let mut relative_score_data: HashMap<PlatformKey, HashMap<DateKey, f32>> = HashMap::new();
        for (platform, _) in &markets_by_platform {
            for date in &dates_for_relative_scoring {
                // calculate relative brier for the day
                let absolute = get_score_from_nested_map(&absolute_score_data, platform, date)?;
                let median =
                    get_score_from_nested_map(&absolute_score_data, &"median".to_owned(), date)?;
                let relative_brier = absolute - median;
                // save it to map
                save_score_to_nested_map(&mut relative_score_data, platform, date, relative_brier)?;
            }
        }

        groups.push(ResponseGroupData {
            group_title: group.title,
            category: group.category,
            markets: markets_by_platform
                .into_iter()
                .map(|(platform, market)| {
                    let absolute_brier = absolute_score_data
                        .get(&platform)
                        .unwrap()
                        .values()
                        .sum::<f32>()
                        / absolute_score_data.get(&platform).unwrap().len() as f32;
                    let relative_brier = relative_score_data
                        .get(&platform)
                        .unwrap()
                        .values()
                        .sum::<f32>()
                        / relative_score_data.get(&platform).unwrap().len() as f32;
                    ResponseMarketData {
                        market_data: market,
                        platform,
                        absolute_brier,
                        relative_brier,
                    }
                })
                .collect(),
        })
    }

    // get the platform metadata
    let platform_list = get_unique_platforms_from_groups(&groups);
    let mut platform_metadata = Vec::with_capacity(platform_list.len());
    for platform in platform_list {
        platform_metadata.push(get_platform_by_name(conn, &platform)?)
    }

    // get the aggregate stats for all categories then each individual category
    let category_list = get_unique_categories_from_groups(&groups);
    let mut platform_stats = get_platform_aggregate_stats(&groups, "All".to_string());
    platform_stats.extend(
        category_list
            .iter()
            .flat_map(|category| get_platform_aggregate_stats(&groups, category.clone())),
    );

    // save it all to the response struct & ship
    let response = FullResponse {
        platform_metadata,
        platform_stats,
        groups,
    };
    Ok(HttpResponse::Ok().json(response))
}
