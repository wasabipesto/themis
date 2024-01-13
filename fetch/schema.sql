DROP TABLE IF EXISTS market;
CREATE TABLE market (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL,
    platform VARCHAR NOT NULL,
    platform_id VARCHAR NOT NULL,
    url VARCHAR NOT NULL,
    open_dt TIMESTAMPTZ NOT NULL,
    close_dt TIMESTAMPTZ NOT NULL,
    open_days REAL NOT NULL,
    volume_usd REAL NOT NULL,
    num_traders INTEGER NOT NULL,
    prob_at_midpoint REAL NOT NULL,
    prob_at_close REAL NOT NULL,
    prob_time_weighted REAL NOT NULL,
    resolution REAL NOT NULL,
    CONSTRAINT platform_unique_by_id UNIQUE (platform, platform_id)
);
DROP TABLE IF EXISTS platform;
CREATE TABLE platform (
    platform_name VARCHAR PRIMARY KEY,
    platform_name_fmt VARCHAR NOT NULL,
    platform_description VARCHAR NOT NULL,
    platform_avatar_url VARCHAR NOT NULL,
    platform_color VARCHAR NOT NULL
);
INSERT INTO platform (
        platform_name,
        platform_name_fmt,
        platform_description,
        platform_avatar_url,
        platform_color
    )
VALUES (
        'manifold',
        'Manifold',
        'A play-money platform where anyone can make any market.',
        'images/manifold.svg',
        '#4337c9'
    ),
    (
        'kalshi',
        'Kalshi',
        'A US-regulated exchange with limited real-money contracts.',
        'images/kalshi.png',
        '#00d298'
    ),
    (
        'metaculus',
        'Metaculus',
        'A forecasting platform focused on calibration instead of bets.',
        'images/metaculus.png',
        '#283441'
    ),
    (
        'polymarket',
        'Polymarket',
        'A high-volume cryptocurrency exchange backed by USDC.',
        'images/polymarket.png',
        '#0072f9'
    );