-- ==========================================
-- DO THE MAGIC, DO THE MAGIC, DO THE MAGIC
-- ==========================================
--
-- === FIND MARKETS SIMILAR TO TARGET ===
DROP FUNCTION IF EXISTS find_similar_markets_by_id(text,double precision,integer);
CREATE OR REPLACE FUNCTION find_similar_markets_by_id(
    target_market_id TEXT,
    threshold FLOAT DEFAULT 0.3,
    limit_count INTEGER DEFAULT 1000
)
RETURNS TABLE(
    id TEXT,
    title TEXT,
    url TEXT,
    platform_slug TEXT,
    platform_name TEXT,
    category_slug TEXT,
    category_name TEXT,
    question_id INTEGER,
    question_invert BOOLEAN,
    question_dismissed INTEGER,
    open_datetime TIMESTAMPTZ,
    close_datetime TIMESTAMPTZ,
    traders_count INTEGER,
    volume_usd DECIMAL,
    duration_days INTEGER,
    resolution DECIMAL,
    cosine_distance FLOAT
) AS $$
DECLARE
    input_embedding VECTOR(768);
BEGIN
    -- Fetch the embedding of the specified market
    SELECT me.embedding INTO input_embedding
    FROM market_embeddings me
    WHERE me.market_id = target_market_id;

    IF input_embedding IS NULL THEN
        RAISE EXCEPTION 'Market ID % does not have embeddings', target_market_id;
    END IF;

    -- Perform similarity search using the fetched embedding
    RETURN QUERY
    SELECT
        m.id,
        m.title,
        m.url,
        m.platform_slug,
        m.platform_name,
        m.category_slug,
        m.category_name,
        m.question_id,
        m.question_invert,
        m.question_dismissed,
        m.open_datetime,
        m.close_datetime,
        m.traders_count,
        m.volume_usd,
        m.duration_days,
        m.resolution,
        (input_embedding <=> me.embedding) AS cosine_distance
    FROM market_embeddings me
    JOIN market_details m ON me.market_id = m.id
    WHERE me.market_id != target_market_id AND
          (input_embedding <=> me.embedding) <= threshold
    ORDER BY cosine_distance ASC
    LIMIT limit_count;
END;
$$ LANGUAGE plpgsql STABLE;

-- === FIND MARKETS THAT NEED EMBEDDINGS ===
CREATE OR REPLACE FUNCTION find_markets_missing_embeddings()
RETURNS SETOF markets AS $$
BEGIN
    RETURN QUERY
    SELECT m.*
    FROM markets m
    WHERE NOT EXISTS (
        SELECT 1
        FROM market_embeddings me
        WHERE me.market_id = m.id
    );
END;
$$ LANGUAGE plpgsql STABLE;

-- === FIND QUESTIONS SIMILAR TO TARGET ===
CREATE OR REPLACE FUNCTION find_similar_questions_by_id(
    target_question_id INTEGER,
    threshold FLOAT DEFAULT 0.3,
    limit_count INTEGER DEFAULT 100
)
RETURNS TABLE(question_id INTEGER, cosine_distance FLOAT) AS $$
DECLARE
    input_embedding VECTOR(768);
BEGIN
    -- Fetch the embedding of the specified question
    SELECT qe.embedding INTO input_embedding
    FROM question_embeddings qe
    WHERE qe.question_id = target_question_id;

    IF input_embedding IS NULL THEN
        RAISE EXCEPTION 'Question ID % does not have embeddings', target_question_id;
    END IF;

    -- Perform similarity search using the fetched embedding
    RETURN QUERY
    SELECT qe.question_id, (input_embedding <=> qe.embedding) AS cosine_distance
    FROM question_embeddings qe
    WHERE qe.question_id != target_question_id AND
          (input_embedding <=> qe.embedding) <= threshold
    ORDER BY cosine_distance ASC
    LIMIT limit_count;
END;
$$ LANGUAGE plpgsql STABLE;

-- === FIND QUESTIONS THAT NEED EMBEDDINGS ===
CREATE OR REPLACE FUNCTION find_questions_missing_embeddings()
RETURNS SETOF questions AS $$
BEGIN
    RETURN QUERY
    SELECT q.*
    FROM questions q
    WHERE NOT EXISTS (
        SELECT 1
        FROM question_embeddings qe
        WHERE qe.question_id = q.id
    );
END;
$$ LANGUAGE plpgsql STABLE;
