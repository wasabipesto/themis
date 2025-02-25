-- ==========================================
-- DATA FOR PLATFORMS TABLE
-- ==========================================
INSERT INTO
    platforms (
        slug,
        name,
        description,
        long_description,
        icon_url,
        site_url,
        wikipedia_url,
        color_primary,
        color_accent
    )
VALUES
    (
        'kalshi',
        'Kalshi',
        'A US-regulated exchange with limited real-money contracts.',
        'Kalshi is the first CFTC-regulated prediction market exchange in the United States, launched in 2021. The platform allows users to trade on event contracts with real money, focusing on carefully regulated markets covering economics, politics, climate, and other measurable outcomes.\nKalshi''s unique position as a regulated entity means it maintains strict compliance standards while offering a legitimate way to hedge against real-world events. The platform uses a central limit order book model and emphasizes transparency in market operations.',
        'images/kalshi.png',
        'https://kalshi.com/',
        'https://en.wikipedia.org/wiki/Kalshi',
        '#00d298',
        '#00694c'
    ),
    (
        'manifold',
        'Manifold',
        'A play-money platform where anyone can make any market.',
        'Manifold Markets is an innovative prediction market platform launched in 2021 that uses play money (called ''mana'') to allow users to create and trade on virtually any topic. The platform''s unique feature is its permissionless nature - anyone can create markets about anything, from world events to personal predictions.\nThe platform uses a automated market maker system to ensure liquidity and implements various market types including binary, numeric, and free response markets. While using virtual currency, Manifold maintains high engagement through social features and competitive elements like leaderboards and user rankings.',
        'images/manifold.svg',
        'https://manifold.markets/',
        'https://en.wikipedia.org/wiki/Manifold_(prediction_market)',
        '#4337c9',
        '#211b64'
    ),
    (
        'metaculus',
        'Metaculus',
        'A forecasting platform focused on calibration instead of bets.',
        'Metaculus is a sophisticated forecasting platform established in 2015 that focuses on aggregating predictions about scientific, technological, and social events. Unlike traditional prediction markets, Metaculus emphasizes forecaster calibration and accuracy over monetary gains.\nThe platform uses advanced aggregation algorithms to combine predictions from its community of forecasters, many of whom are experts in their fields. Metaculus is known for its long-term forecasting tournaments, detailed discussion forums, and comprehensive tracking of forecaster performance. The platform has gained recognition for its accurate predictions on various topics, from pandemic outcomes to technological developments, and places a strong emphasis on educational content and methodological rigor.',
        'images/metaculus.png',
        'https://www.metaculus.com/home/',
        'https://en.wikipedia.org/wiki/Metaculus',
        '#283441',
        '#141a20'
    ),
    (
        'polymarket',
        'Polymarket',
        'A high-volume cryptocurrency exchange backed by USDC.',
        'Polymarket is a blockchain-based prediction market platform launched in 2020 that allows users to trade on real-world events using cryptocurrency (USDC). The platform operates on the Polygon network, offering low transaction fees and quick settlement times. Polymarket implements an automated market maker system and focuses on high-profile current events, politics, sports, and cryptocurrency markets.\nKnown for its significant trading volumes and liquidity, the platform attracts both retail and professional traders. Polymarket''s decentralized nature allows for global participation, though with some geographical restrictions. The platform features a curated selection of markets with clear resolution sources and has gained popularity in both the cryptocurrency and prediction market communities.',
        'images/polymarket.png',
        'https://polymarket.com/',
        'https://en.wikipedia.org/wiki/Polymarket',
        '#0072f9',
        '#00397c'
    );
