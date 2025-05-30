-- ==========================================
-- LIVE-UPDATED VIEWS
-- ==========================================
--
-- === DROP ALL VIEWS AND FUNCTIONS (NO DATA LOSS) ===
DROP MATERIALIZED VIEW IF EXISTS question_details;
DROP MATERIALIZED VIEW IF EXISTS market_details;
DROP MATERIALIZED VIEW IF EXISTS platform_category_scores_details;
DROP MATERIALIZED VIEW IF EXISTS daily_probability_details;
DROP MATERIALIZED VIEW IF EXISTS market_scores_details;
DROP MATERIALIZED VIEW IF EXISTS category_details;
DROP MATERIALIZED VIEW IF EXISTS platform_details;

-- Drop function with all possible parameter combinations for safety
DROP FUNCTION IF EXISTS calculate_hotness_score(integer, integer, decimal, date, date, decimal, decimal, decimal, decimal, decimal);

-- === HOTNESS SCORE CALCULATION ===
CREATE OR REPLACE FUNCTION calculate_hotness_score(
    market_count INTEGER,
    total_traders INTEGER,
    total_volume DECIMAL,
    start_date DATE,
    end_date DATE,
    weight_market_count DECIMAL DEFAULT 20,
    weight_traders DECIMAL DEFAULT 10,
    weight_volume DECIMAL DEFAULT 5,
    weight_duration DECIMAL DEFAULT 10,
    weight_age DECIMAL DEFAULT -1
) RETURNS DECIMAL AS $$
DECLARE
    duration_days INTEGER;
    days_since_end INTEGER;
    current_date DATE := CURRENT_DATE;
BEGIN
    -- Calculate duration and recency
    duration_days := (end_date - start_date)::INTEGER;
    days_since_end := (current_date - end_date)::INTEGER;

    -- Normalize inputs to avoid extreme values
    -- Log transformation for count-based metrics
    RETURN (weight_market_count * COALESCE(LN(GREATEST(market_count, 1)), 0)) +
           (weight_traders * COALESCE(LN(GREATEST(total_traders, 1)), 0)) +
           (weight_volume * COALESCE(LN(GREATEST(total_volume, 1)), 0)) +
           (weight_duration * COALESCE(LN(GREATEST(duration_days, 1)), 0)) +
           (weight_age * COALESCE(LN(GREATEST(days_since_end, 1)), 0));
END;
$$ LANGUAGE plpgsql;

-- === PLATFORM DETAILS ===
CREATE MATERIALIZED VIEW platform_details AS
SELECT
    p.*,
    stats.total_markets,
    stats.total_traders,
    stats.total_volume
FROM
    platforms p
    LEFT JOIN (
        SELECT
            platform_slug,
            COUNT(DISTINCT id) AS total_markets,
            SUM(traders_count) AS total_traders,
            SUM(volume_usd) AS total_volume
        FROM
            markets
        GROUP BY
            platform_slug
    ) stats ON p.slug = stats.platform_slug;
CREATE UNIQUE INDEX platform_details_slug_idx ON platform_details (slug);

-- === CATEGORY DETAILS ===
CREATE MATERIALIZED VIEW category_details AS
SELECT
    c.*,
    COALESCE(stats.total_markets, 0) AS total_markets,
    COALESCE(stats.total_traders, 0) AS total_traders,
    COALESCE(stats.total_volume, 0) AS total_volume
FROM
    categories c
    LEFT JOIN (
        SELECT
            q.category_slug,
            COUNT(DISTINCT m.id) AS total_markets,
            SUM(m.traders_count) AS total_traders,
            SUM(m.volume_usd) AS total_volume
        FROM
            questions q
            LEFT JOIN market_questions mq ON q.id = mq.question_id
            LEFT JOIN markets m ON mq.market_id = m.id
        GROUP BY
            q.category_slug
    ) stats ON c.slug = stats.category_slug;
CREATE UNIQUE INDEX category_details_slug_idx ON category_details (slug);

-- === MARKET-QUESTION SCORE DETAILS ===
CREATE MATERIALIZED VIEW market_scores_details AS
SELECT
    ms.score_type,
    m.id AS market_id,
    m.title AS market_title,
    m.url AS market_url,
    m.platform_slug,
    p.name AS platform_name,
    m.category_slug,
    mq.question_id AS question_id,
    m.open_datetime,
    m.close_datetime,
    m.traders_count,
    m.volume_usd,
    m.duration_days,
    mq.question_invert,
    m.resolution,
    ms.score,
    ms.grade
FROM
    market_scores ms
    JOIN markets m ON ms.market_id = m.id
    JOIN platforms p ON m.platform_slug = p.slug
    LEFT JOIN market_questions mq ON m.id = mq.market_id;
CREATE UNIQUE INDEX market_scores_details_qms_idx ON market_scores_details (question_id, market_id, score_type);

-- === DAILY PROBABILITY POINT DETAILS ===
CREATE MATERIALIZED VIEW daily_probability_details AS
SELECT
    m.id AS market_id,
    m.title AS market_title,
    p.slug AS platform_slug,
    p.name AS platform_name,
    mq.question_id AS question_id,
    mq.question_invert AS question_invert,
    dp.date AS date,
    dp.prob AS prob
FROM
    daily_probabilities dp
    JOIN markets m ON dp.market_id = m.id
    JOIN platforms p ON m.platform_slug = p.slug
    LEFT JOIN market_questions mq ON m.id = mq.market_id;
CREATE UNIQUE INDEX daily_probability_details_md_idx ON daily_probability_details (market_id, date);

-- === PLATFORM-CATEGORY SCORE DETAILS ===
CREATE MATERIALIZED VIEW platform_category_scores_details AS
SELECT
    ps.platform_slug,
    p.name AS platform_name,
    ps.category_slug,
    c.name AS category_name,
    ps.score_type,
    ps.num_markets,
    ps.score,
    ps.grade
FROM
    platform_category_scores ps
    JOIN platforms p ON ps.platform_slug = p.slug
    JOIN categories c ON ps.category_slug = c.slug;
CREATE UNIQUE INDEX platform_category_scores_details_pc_idx ON platform_category_scores_details (platform_slug, category_slug, score_type);

-- === MARKET DETAILS ===
CREATE MATERIALIZED VIEW market_details AS
SELECT
    m.id,
    m.title,
    m.url,
    m.description,
    m.platform_slug,
    p.name AS platform_name,
    m.category_slug,
    c.name AS category_name,
    mq.question_id,
    q.slug AS question_slug,
    q.title AS question_title,
    mq.question_invert,
    COALESCE(md.dismissed_status, 0) AS question_dismissed,
    m.open_datetime,
    m.close_datetime,
    m.traders_count,
    m.volume_usd,
    m.duration_days,
    m.resolution
FROM
    markets m
    JOIN platforms p ON m.platform_slug = p.slug
    LEFT JOIN market_questions mq ON m.id = mq.market_id
    LEFT JOIN questions q ON mq.question_id = q.id
    LEFT JOIN market_dismissals md ON m.id = md.market_id
    LEFT JOIN categories c ON m.category_slug = c.slug;
CREATE UNIQUE INDEX market_details_id_idx ON market_details (id);

-- === QUESTION DETAILS ===
CREATE MATERIALIZED VIEW question_details AS
SELECT
    q.*,
    c.name AS category_name,
    COALESCE(stats.market_count, 0) AS market_count,
    COALESCE(stats.total_traders, 0) AS total_traders,
    COALESCE(stats.total_volume, 0) AS total_volume,
    COALESCE(q.start_date_override, stats.min_open_date::DATE) AS start_date_actual,
    COALESCE(q.end_date_override, stats.max_close_date::DATE) AS end_date_actual,
    (COALESCE(q.end_date_override, stats.max_close_date::DATE) -
     COALESCE(q.start_date_override, stats.min_open_date::DATE))::INTEGER AS total_duration,
    calculate_hotness_score(
        COALESCE(stats.market_count, 0)::INTEGER,
        COALESCE(stats.total_traders, 0)::INTEGER,
        COALESCE(stats.total_volume, 0)::DECIMAL,
        COALESCE(q.start_date_override, stats.min_open_date::DATE),
        COALESCE(q.end_date_override, stats.max_close_date::DATE)
    ) AS hotness_score
FROM
    questions q
    LEFT JOIN categories c ON q.category_slug = c.slug
    LEFT JOIN (
        SELECT
            question_id,
            COUNT(DISTINCT market_id) AS market_count,
            SUM(m.traders_count) AS total_traders,
            SUM(m.volume_usd) AS total_volume,
            MIN(m.open_datetime) AS min_open_date,
            MAX(m.close_datetime) AS max_close_date
        FROM
            market_questions mq
            JOIN markets m ON mq.market_id = m.id
        GROUP BY
            question_id
    ) stats ON q.id = stats.question_id;
CREATE UNIQUE INDEX question_details_id_idx ON question_details (id);



-- === REFRESH ALL VIEWS ===
CREATE OR REPLACE FUNCTION refresh_all_materialized_views()
RETURNS VOID AS $$
BEGIN
    -- Refresh views in order of dependencies (lowest level first)
    REFRESH MATERIALIZED VIEW CONCURRENTLY platform_details; -- 2 SECONDS
    REFRESH MATERIALIZED VIEW CONCURRENTLY category_details; -- INSTANT
    REFRESH MATERIALIZED VIEW CONCURRENTLY market_scores_details;
    REFRESH MATERIALIZED VIEW CONCURRENTLY platform_category_scores_details;
    REFRESH MATERIALIZED VIEW CONCURRENTLY daily_probability_details; -- >30 SECONDS
    -- These depend on other materialized views
    REFRESH MATERIALIZED VIEW CONCURRENTLY question_details; -- INSTANT
    REFRESH MATERIALIZED VIEW CONCURRENTLY market_details; -- 3 SECONDS
END;
$$ LANGUAGE plpgsql;

-- ONLY ALLOW ADMINS TO INVOKE REFRESH
REVOKE EXECUTE ON FUNCTION refresh_all_materialized_views() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION refresh_all_materialized_views() TO admin;

-- === REFRESH JUST THE QUICKEST VIEWS FOR DEVELOPMENT LOOP ===
CREATE OR REPLACE FUNCTION refresh_quick_materialized_views()
RETURNS VOID AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY category_details; -- INSTANT
    REFRESH MATERIALIZED VIEW CONCURRENTLY question_details; -- INSTANT
    REFRESH MATERIALIZED VIEW CONCURRENTLY market_details; -- 3 SECONDS
END;
$$ LANGUAGE plpgsql;

-- ONLY ALLOW ADMINS TO INVOKE REFRESH
REVOKE EXECUTE ON FUNCTION refresh_quick_materialized_views() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION refresh_quick_materialized_views() TO admin;
