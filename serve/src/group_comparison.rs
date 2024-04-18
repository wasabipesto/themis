use super::*;

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

/// Extract the unique platform names from a list of groups.
fn get_unique_platforms_from_groups(groups: &Vec<ResponseGroupData>) -> Vec<String> {
    let mut set: HashSet<String> = HashSet::new();
    for group in groups {
        for market in &group.markets {
            set.insert(market.platform.clone());
        }
    }
    set.into_iter().collect()
}

/// Extract the unique category names from a list of groups.
fn get_unique_categories_from_groups(groups: &Vec<ResponseGroupData>) -> Vec<String> {
    let mut set: HashSet<String> = HashSet::new();
    for group in groups {
        set.insert(group.category.clone());
    }
    set.into_iter().collect()
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
            platform_absolute_brier: Some(psi.cumulative_absolute_brier / psi.count as f32),
            platform_relative_brier: Some(psi.cumulative_relative_brier / psi.count as f32),
            platform_sample_presence: psi.count as f32 / total_count as f32,
        })
    }
    platform_stats
}

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
        let mut markets = Vec::with_capacity(group.markets.len());
        for market in group.markets {
            // get market data from db
            let market_data =
                get_market_by_platform_id(conn, &market.platform, &market.platform_id)?;

            markets.push(ResponseMarketData {
                market_data,
                platform: market.platform,
                absolute_brier: 0.0, // TODO
                relative_brier: 0.0, // TODO
            })
        }

        groups.push(ResponseGroupData {
            group_title: group.title,
            category: group.category,
            markets,
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
