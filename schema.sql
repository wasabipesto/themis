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
    category VARCHAR DEFAULT 'None' NOT NULL,
    prob_at_midpoint REAL NOT NULL,
    prob_at_close REAL NOT NULL,
    prob_time_avg REAL NOT NULL,
    resolution REAL NOT NULL,
    CONSTRAINT platform_unique_by_id UNIQUE (platform, platform_id)
);
DROP TABLE IF EXISTS platform;
CREATE TABLE platform (
    name VARCHAR PRIMARY KEY,
    name_fmt VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    site_url VARCHAR NOT NULL,
    avatar_url VARCHAR NOT NULL,
    color VARCHAR NOT NULL
);
INSERT INTO platform (
        name,
        name_fmt,
        description,
        site_url,
        avatar_url,
        color
    )
VALUES (
        'manifold',
        'Manifold',
        'A play-money platform where anyone can make any market.',
        'https://manifold.markets/',
        'images/manifold.svg',
        '#4337c9'
    ),
    (
        'kalshi',
        'Kalshi',
        'A US-regulated exchange with limited real-money contracts.',
        'https://kalshi.com/',
        'images/kalshi.png',
        '#00d298'
    ),
    (
        'metaculus',
        'Metaculus',
        'A forecasting platform focused on calibration instead of bets.',
        'https://www.metaculus.com/home/',
        'images/metaculus.png',
        '#283441'
    ),
    (
        'polymarket',
        'Polymarket',
        'A high-volume cryptocurrency exchange backed by USDC.',
        'https://polymarket.com/',
        'images/polymarket.png',
        '#0072f9'
    );