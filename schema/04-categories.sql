-- ==========================================
-- SAMPLE DATA FOR CATEGORIES TABLE
-- ==========================================
INSERT INTO
    categories (
        name,
        slug,
        parent_slug,
        is_parent,
        icon,
        description
    )
VALUES
    (
        'AI',
        'ai',
        NULL,
        TRUE,
        'mdi-robot',
        'Tracking developments in artificial intelligence, from breakthroughs in machine learning to predictions about AGI timelines, capabilities, and potential impacts on society and industry.'
    ),
    (
        'Culture',
        'culture',
        NULL,
        TRUE,
        'mdi-theater',
        'Forecasts on social trends, entertainment, media, and demographic shifts. Covers evolving cultural phenomena, from streaming platforms to social movements and changing consumer behaviors.'
    ),
    (
        'Economics',
        'economics',
        NULL,
        TRUE,
        'mdi-chart-line',
        'Predictions for financial markets, economic indicators, and global trade. Includes forecasts on inflation, interest rates, commodity prices, and major economic developments worldwide.'
    ),
    (
        'Politics',
        'politics',
        NULL,
        TRUE,
        'mdi-gavel',
        'Tracking electoral outcomes, policy changes, and geopolitical events. Covers national and international politics, including elections, conflicts, treaties, and major diplomatic developments.'
    ),
    (
        'Science',
        'science',
        NULL,
        TRUE,
        'mdi-microscope',
        'Following advances across scientific disciplines, from climate research to space exploration. Includes breakthrough predictions in physics, biology, medicine, and environmental science.'
    ),
    (
        'Sports',
        'sports',
        NULL,
        TRUE,
        'mdi-basketball',
        'Forecasting outcomes in professional and amateur athletics. Covers game results, championship predictions, player performance metrics, and industry developments across major sports.'
    ),
    (
        'Technology',
        'technology',
        NULL,
        TRUE,
        'mdi-chip',
        'Monitoring innovations beyond AI, including renewable energy, biotechnology, quantum computing, and emerging tech trends reshaping industries and daily life.'
    ),
    (
        'US Politics',
        'us-politics',
        'politics',
        FALSE,
        'mdi-death-star-variant',
        'Politics specifically within the United States.'
    ),
    (
        'Football',
        'football',
        'sports',
        FALSE,
        'mdi-football',
        'Foobaw!'
    );
