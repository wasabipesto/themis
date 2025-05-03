-- ==========================================
-- DO THE MAGIC, DO THE MAGIC, DO THE MAGIC
-- ==========================================

-- === FIND MARKETS SIMILAR TO TARGET ===
CREATE OR REPLACE FUNCTION find_similar_markets_by_id(
    target_market_id TEXT,
    threshold FLOAT DEFAULT 0.3,
    limit_count INTEGER DEFAULT 10
)
RETURNS TABLE(market_id TEXT, cosine_distance FLOAT) AS $$
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
    SELECT me.market_id, (input_embedding <=> me.embedding) AS cosine_distance
    FROM market_embeddings me
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
    limit_count INTEGER DEFAULT 10
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
