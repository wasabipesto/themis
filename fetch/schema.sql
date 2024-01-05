DROP TABLE IF EXISTS market;
CREATE TABLE market (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL,
    platform VARCHAR NOT NULL,
    platform_id VARCHAR NOT NULL,
    url VARCHAR NOT NULL,
    open_days REAL NOT NULL,
    volume_usd REAL NOT NULL,
    prob_at_midpoint REAL NOT NULL,
    prob_at_close REAL NOT NULL,
    resolution REAL NOT NULL,
    CONSTRAINT platform_unique_by_id UNIQUE (platform, platform_id)
);