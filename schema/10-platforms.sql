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
        'Kalshi is the first CFTC-regulated prediction market exchange in the United States, launched in 2021. The platform allows users to trade on event contracts with real money. Kalshi''s unique position as a regulated entity means it maintains strict compliance standards while offering a legitimate way to hedge against real-world events. The platform uses a central limit order book model and emphasizes transparency in market operations.\nThe platform has a large number of short-duration, high frequency markets for predictions on common events. It generates revenue from fees on market orders and is required to limit individual positions to $25,000 per market.',
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
        'Manifold is a prediction market platform launched in 2021 that uses play money (called ''mana'') to allow users to create and trade on virtually any topic. The platform''s unique feature is its permissionless nature - anyone can create markets about anything, from major world events to personal goals.\nIn order to provide liquidity on the huge number of markets, the platform uses an automated market maker while also allowing user limit orders and bots. Mana can be earned through the site''s numerous social features, such as betting streaks, trader bonuses, and monthly leagues. The site is sustained by users purchasing mana or boosts to drive engagement on their markets.',
        'images/manifold.svg',
        'https://manifold.markets/',
        'https://en.wikipedia.org/wiki/Manifold_(prediction_market)',
        '#4337c9',
        '#211b64'
    ),
    (
        'metaculus',
        'Metaculus',
        'A long-horizon forecasting platform, not a prediction market.',
        'Metaculus is a sophisticated forecasting platform established in 2015 that focuses on aggregating predictions about scientific, technological, and social events. Unlike traditional prediction markets, Metaculus emphasizes forecaster calibration and accuracy over monetary gains. The platform uses advanced aggregation algorithms to combine predictions from its community of forecasters, many of whom are experts in their fields.\nMetaculus is known for its long-term forecasting tournaments, detailed discussion forums, and comprehensive tracking of forecaster performance. The site provides detailed information to forecasters and a number of tournaments to incentivise deep desearch, advanced AI bots, and new forecasting techniques.',
        'images/metaculus.png',
        'https://www.metaculus.com/home/',
        'https://en.wikipedia.org/wiki/Metaculus',
        '#283441',
        '#141a20'
    ),
    (
        'polymarket',
        'Polymarket',
        'A high-volume, decentralized cryptocurrency exchange.',
        'Polymarket is a blockchain-based prediction market platform launched in 2020 that allows users to trade on real-world events using cryptocurrency. The site is banned from use in several countries including the US and UK, but these bans are fairly simple to circumvent with VPNs. As such, the platform has a large number of users and very high volume on popular markets.\nMarket resolutions are determined by a decentralized "optimistic oracle" which incentivises tokenholders to vote on the correct outcome with allowances for disputes or emergency overrides (though some users still intentionally use their voting power to profit). The site also provides rewards for liquidity providers and some negative risk events.',
        'images/polymarket.png',
        'https://polymarket.com/',
        'https://en.wikipedia.org/wiki/Polymarket',
        '#0072f9',
        '#00397c'
    );
