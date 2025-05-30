-- ==========================================
-- PERFORMANCE INDEXES FOR MATERIALIZED VIEWS
-- ==========================================
--
-- Additional indexes to optimize materialized view refresh performance
-- These complement the existing indexes in 02-schema.sql

-- === MARKET_SCORES TABLE INDEXES ===
-- For market_scores_details view joins
CREATE INDEX IF NOT EXISTS idx_market_scores_market_id ON market_scores (market_id);
CREATE INDEX IF NOT EXISTS idx_market_scores_comprehensive ON market_scores (market_id, score_type);

-- === PLATFORM_CATEGORY_SCORES TABLE INDEXES ===
-- For platform_category_scores_details view joins
CREATE INDEX IF NOT EXISTS idx_platform_category_scores_platform ON platform_category_scores (platform_slug);
CREATE INDEX IF NOT EXISTS idx_platform_category_scores_category ON platform_category_scores (category_slug);

-- === MARKETS TABLE ADDITIONAL INDEXES ===
-- For various aggregations in views
CREATE INDEX IF NOT EXISTS idx_markets_open_close_dates ON markets (open_datetime, close_datetime);
CREATE INDEX IF NOT EXISTS idx_markets_stats_aggregation ON markets (platform_slug, traders_count, volume_usd, open_datetime, close_datetime);

-- === MARKET_QUESTIONS TABLE COMPOSITE INDEXES ===
-- For efficient joins in category_details and question_details views
CREATE INDEX IF NOT EXISTS idx_market_questions_question_market_comprehensive ON market_questions (question_id, market_id, question_invert);

-- === DAILY_PROBABILITIES TABLE ADDITIONAL INDEXES ===
-- For daily_probability_details view performance
CREATE INDEX IF NOT EXISTS idx_daily_probabilities_date ON daily_probabilities (date);
CREATE INDEX IF NOT EXISTS idx_daily_probabilities_market_date_prob ON daily_probabilities (market_id, date, prob);

-- === QUESTIONS TABLE ADDITIONAL INDEXES ===
-- For question_details view and category aggregations
CREATE INDEX IF NOT EXISTS idx_questions_category_id ON questions (category_slug, id);
CREATE INDEX IF NOT EXISTS idx_questions_dates ON questions (start_date_override, end_date_override);

-- === COVERING INDEXES FOR HEAVY AGGREGATION QUERIES ===
-- These include commonly selected columns to avoid table lookups

-- Markets covering index for aggregations
CREATE INDEX IF NOT EXISTS idx_markets_aggregation_covering ON markets (platform_slug) 
INCLUDE (id, traders_count, volume_usd, open_datetime, close_datetime, category_slug);

-- Market questions covering index
CREATE INDEX IF NOT EXISTS idx_market_questions_covering ON market_questions (question_id) 
INCLUDE (market_id, question_invert);

-- Daily probabilities covering index for joins
CREATE INDEX IF NOT EXISTS idx_daily_probabilities_covering ON daily_probabilities (market_id) 
INCLUDE (date, prob);