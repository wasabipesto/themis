-- ==========================================
-- TABLE SCHEMA
-- ==========================================
CREATE TABLE platforms (
    slug TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    long_description TEXT,
    icon_url TEXT,
    site_url TEXT,
    wikipedia_url TEXT,
    color_primary TEXT,
    color_accent TEXT,
    total_markets INTEGER,
    total_traders INTEGER,
    total_volume DECIMAL
);

CREATE TABLE categories (
    slug TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    parent_slug TEXT,
    is_parent BOOLEAN DEFAULT false NOT NULL,
    icon TEXT,
    total_markets INTEGER,
    total_traders INTEGER,
    total_volume DECIMAL,
    FOREIGN KEY (parent_slug) REFERENCES categories (slug)
);

CREATE TABLE questions (
    id SERIAL PRIMARY KEY,
    title TEXT NOT NULL,
    slug TEXT UNIQUE NOT NULL,
    description TEXT,
    category_slug TEXT NOT NULL,
    category_name TEXT NOT NULL,
    parent_category_slug TEXT,
    parent_category_name TEXT,
    start_date_override TIMESTAMPTZ,
    end_date_override TIMESTAMPTZ,
    total_traders INTEGER,
    total_volume DECIMAL,
    total_duration DECIMAL,
    overall_grade TEXT,
    overall_brier_score_rel DECIMAL,
    overall_brier_score_abs DECIMAL,
    FOREIGN KEY (category_slug) REFERENCES categories (slug),
    FOREIGN KEY (parent_category_slug) REFERENCES categories (slug)
);

CREATE TABLE markets (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    platform_slug TEXT NOT NULL,
    platform_name TEXT NOT NULL,
    question_id INTEGER,
    question_invert BOOLEAN DEFAULT false NOT NULL,
    question_dismissed INTEGER DEFAULT 0 NOT NULL,
    url TEXT NOT NULL,
    open_datetime TIMESTAMPTZ NOT NULL,
    close_datetime TIMESTAMPTZ NOT NULL,
    traders_count INTEGER,
    volume_usd DECIMAL,
    duration_days INTEGER NOT NULL,
    category TEXT DEFAULT 'None' NOT NULL,
    prob_at_midpoint DECIMAL NOT NULL,
    prob_time_avg DECIMAL NOT NULL,
    resolution DECIMAL NOT NULL,
    FOREIGN KEY (platform_slug) REFERENCES platforms (slug),
    FOREIGN KEY (question_id) REFERENCES questions (id)
);

CREATE TABLE platform_scores (
    platform_slug TEXT NOT NULL,
    platform_name TEXT NOT NULL,
    category_slug TEXT NOT NULL,
    category_name TEXT NOT NULL,
    num_markets INTEGER NOT NULL,
    grade TEXT NOT NULL,
    brier_score_rel DECIMAL NOT NULL,
    brier_score_abs DECIMAL NOT NULL,
    PRIMARY KEY (platform_slug, category_slug),
    FOREIGN KEY (platform_slug) REFERENCES platforms (slug),
    FOREIGN KEY (category_slug) REFERENCES categories (slug)
);

CREATE TABLE market_scores (
    question_id INTEGER NOT NULL,
    platform_slug TEXT NOT NULL,
    platform_name TEXT NOT NULL,
    market_id TEXT NOT NULL,
    market_link TEXT NOT NULL,
    traders INTEGER NOT NULL,
    volume DECIMAL NOT NULL,
    duration INTEGER NOT NULL,
    grade TEXT NOT NULL,
    brier_score_rel DECIMAL NOT NULL,
    brier_score_abs DECIMAL NOT NULL,
    PRIMARY KEY (question_id, platform_slug, market_id),
    FOREIGN KEY (question_id) REFERENCES questions (id),
    FOREIGN KEY (platform_slug) REFERENCES platforms (slug)
);

CREATE TABLE daily_probabilities (
    market_id TEXT NOT NULL,
    platform_slug TEXT NOT NULL,
    date TIMESTAMPTZ NOT NULL,
    prob DECIMAL NOT NULL,
    PRIMARY KEY (market_id, platform_slug, date),
    FOREIGN KEY (market_id) REFERENCES markets (id),
    FOREIGN KEY (platform_slug) REFERENCES platforms (slug)
);

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
    FOREIGN KEY (platform_slug) REFERENCES platforms (slug)
);
