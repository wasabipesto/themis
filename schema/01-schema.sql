-- ==========================================
-- TABLE SCHEMA
-- ==========================================
-- === PLATFORMS ===
CREATE TABLE platforms (
    slug TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    long_description TEXT,
    icon_url TEXT,
    site_url TEXT,
    wikipedia_url TEXT,
    color_primary TEXT,
    color_accent TEXT
);

-- === CATEGORIES ===
CREATE TABLE categories (
    slug TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    icon TEXT
);

-- === QUESTIONS ===
CREATE TABLE questions (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    slug TEXT UNIQUE NOT NULL,
    description TEXT,
    category_slug TEXT NOT NULL,
    start_date_override DATE,
    end_date_override DATE,
    overall_grade TEXT,
    overall_brier_score_rel DECIMAL,
    overall_brier_score_abs DECIMAL,
    FOREIGN KEY (category_slug) REFERENCES categories (slug) ON UPDATE CASCADE
);

CREATE INDEX idx_questions_category_slug ON questions (category_slug);

-- === MARKETS ===
CREATE TABLE markets (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    url TEXT NOT NULL,
    description TEXT,
    platform_slug TEXT NOT NULL,
    category_slug TEXT,
    open_datetime TIMESTAMPTZ NOT NULL,
    close_datetime TIMESTAMPTZ NOT NULL,
    traders_count INTEGER,
    volume_usd DECIMAL,
    duration_days INTEGER NOT NULL,
    prob_at_midpoint DECIMAL NOT NULL,
    prob_time_avg DECIMAL NOT NULL,
    resolution DECIMAL NOT NULL,
    FOREIGN KEY (platform_slug) REFERENCES platforms (slug) ON UPDATE CASCADE,
    FOREIGN KEY (category_slug) REFERENCES categories (slug) ON UPDATE CASCADE
);

CREATE INDEX idx_markets_platform_slug ON markets (platform_slug);

CREATE INDEX idx_markets_category_slug ON markets (category_slug);

CREATE INDEX idx_markets_platform_category ON markets (platform_slug, category_slug);

CREATE INDEX idx_markets_platform_traders_volume ON markets (platform_slug, traders_count, volume_usd);

ALTER TABLE markets ADD CONSTRAINT chk_prob_at_midpoint_range CHECK (prob_at_midpoint BETWEEN 0 AND 1);

ALTER TABLE markets ADD CONSTRAINT chk_prob_time_avg_range CHECK (prob_time_avg BETWEEN 0 AND 1);

ALTER TABLE markets ADD CONSTRAINT chk_resolution_range CHECK (resolution BETWEEN 0 AND 1);

-- === MARKET-QUESTIONS JUNCTIONS ===
CREATE TABLE market_questions (
    market_id TEXT PRIMARY KEY,
    question_id INTEGER NOT NULL,
    question_invert BOOLEAN DEFAULT false NOT NULL,
    FOREIGN KEY (market_id) REFERENCES markets (id) ON UPDATE CASCADE,
    FOREIGN KEY (question_id) REFERENCES questions (id) ON UPDATE CASCADE,
    CONSTRAINT unique_market_question UNIQUE (market_id, question_id)
);

CREATE INDEX idx_market_questions_question_id ON market_questions (question_id);

CREATE INDEX idx_market_questions_market_id ON market_questions (market_id);

CREATE INDEX idx_market_questions_question_comprehensive ON market_questions (question_id, market_id);

-- === MARKET DISMISSALS ===
CREATE TABLE market_dismissals (
    market_id TEXT PRIMARY KEY,
    dismissed_status INTEGER NOT NULL,
    last_updated TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (market_id) REFERENCES markets (id) ON UPDATE CASCADE
);

CREATE INDEX idx_market_dismissals_market_id ON market_dismissals (market_id);

CREATE INDEX idx_market_dismissals_status ON market_dismissals (market_id, dismissed_status);

-- === PLATFORM-CATEGORY SCORES ===
CREATE TABLE platform_scores (
    platform_slug TEXT NOT NULL,
    category_slug TEXT NOT NULL,
    num_markets INTEGER NOT NULL,
    grade TEXT NOT NULL,
    brier_score_rel DECIMAL NOT NULL,
    brier_score_abs DECIMAL NOT NULL,
    PRIMARY KEY (platform_slug, category_slug),
    FOREIGN KEY (platform_slug) REFERENCES platforms (slug) ON UPDATE CASCADE,
    FOREIGN KEY (category_slug) REFERENCES categories (slug) ON UPDATE CASCADE
);

-- === MARKET-QUESTION SCORES ===
CREATE TABLE market_scores (
    market_id TEXT PRIMARY KEY,
    grade TEXT NOT NULL,
    brier_score_rel DECIMAL NOT NULL,
    brier_score_abs DECIMAL NOT NULL,
    FOREIGN KEY (market_id) REFERENCES markets (id) ON UPDATE CASCADE
);

CREATE INDEX idx_market_scores_market_id ON market_scores (market_id);

CREATE INDEX idx_market_scores_comprehensive ON market_scores (
    market_id,
    grade,
    brier_score_rel,
    brier_score_abs
);

-- === DAILY PROBABILITY POINTS ===
CREATE TABLE daily_probabilities (
    market_id TEXT NOT NULL,
    date TIMESTAMPTZ NOT NULL,
    prob DECIMAL NOT NULL,
    PRIMARY KEY (market_id, date),
    FOREIGN KEY (market_id) REFERENCES markets (id) ON UPDATE CASCADE
);

CREATE INDEX idx_daily_probabilities_market_id ON daily_probabilities (market_id);

ALTER TABLE daily_probabilities ADD CONSTRAINT chk_prob_range CHECK (prob BETWEEN 0 AND 1);

-- === CALIBRATION CHART POINTS ===
CREATE TABLE calibration_points (
    platform_slug TEXT NOT NULL,
    x_start DECIMAL,
    x_center DECIMAL NOT NULL,
    x_end DECIMAL,
    y_start DECIMAL,
    y_center DECIMAL NOT NULL,
    y_end DECIMAL,
    count INTEGER,
    PRIMARY KEY (platform_slug, x_center, y_center),
    FOREIGN KEY (platform_slug) REFERENCES platforms (slug) ON UPDATE CASCADE
);
