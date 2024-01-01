DROP TABLE IF EXISTS market;
CREATE TABLE market (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL,
    platform VARCHAR NOT NULL,
    platform_id VARCHAR NOT NULL,
    url VARCHAR NOT NULL,
    CONSTRAINT platform_unique_by_id UNIQUE (platform, platform_id)
);