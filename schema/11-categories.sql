-- ==========================================
-- SAMPLE DATA FOR CATEGORIES TABLE
-- ==========================================
INSERT INTO
    categories (name, slug, icon, description)
VALUES
    (
        'Culture',
        'culture',
        'mdi:theater',
        'Questions about the most popular movies, TV, music, games, celebrities, and the awards surrounding it all. More broadly, questions about demographics, cultural trends, and societal issues.'
    ),
    (
        'Economics',
        'economics',
        'mdi:chart-line',
        'Predictions for financial markets, economic indicators, and cryptocurrency. Includes forecasts on inflation, interest rates, commodity prices, and major economic developments worldwide.'
    ),
    (
        'Politics',
        'politics',
        'mdi:gavel',
        'Tracking electoral outcomes, policy changes, and geopolitical events. Covers national and international politics, including elections, conflicts, treaties, and major diplomatic developments.'
    ),
    (
        'Science',
        'science',
        'mdi:microscope',
        'Following advances across scientific disciplines, from climate research to space exploration and biotechnology. When will the next advances happen, and how much of an impact will they have?'
    ),
    (
        'Sports',
        'sports',
        'mdi:basketball',
        'Forecasting outcomes in professional athletics. Covers game results, championship predictions, player performance metrics, and olympic medals.'
    ),
    (
        'Technology',
        'technology',
        'mdi:chip',
        'Monitoring innovations in the tech industry such as new devices and AI capabilities. Also covers new advancements in tech such as blockchain, quantum computing, and virtual reality.'
    );
