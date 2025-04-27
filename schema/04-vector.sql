-- ==========================================
-- THE OLD WORLD IS DYING
-- THE NEW WORLD STRUGGLES TO BE BORN
-- NOW IS THE TIME OF VECTORS
-- ==========================================
-- === INITIALIZE THE EXTENSION ===
CREATE EXTENSION IF NOT EXISTS vector;

-- === DROP TABLES IF EXISTING ===
DROP TABLE IF EXISTS market_embeddings;

DROP TABLE IF EXISTS question_embeddings;

-- === MARKET EMBEDDINGS ===
CREATE TABLE market_embeddings (
    market_id TEXT PRIMARY KEY,
    embedding vector (768),
    FOREIGN KEY (market_id) REFERENCES markets (id) ON DELETE CASCADE
);

-- === QUESTION EMBEDDINGS ===
CREATE TABLE question_embeddings (
    question_id INTEGER PRIMARY KEY,
    embedding vector (768),
    FOREIGN KEY (question_id) REFERENCES questions (id) ON DELETE CASCADE
);

-- === FIND MARKETS SIMILAR TO TARGET ===
CREATE OR REPLACE FUNCTION find_similar_markets_by_id(
    target_market_id TEXT,
    threshold FLOAT,
    limit_count INTEGER DEFAULT 10
)
RETURNS TABLE(market_id TEXT, similarity_score FLOAT) AS $$
DECLARE
    input_embedding VECTOR(768);
BEGIN
    -- Fetch the embedding of the specified market
    SELECT me.embedding INTO input_embedding
    FROM market_embeddings me
    WHERE me.market_id = target_market_id;

    IF input_embedding IS NULL THEN
        RAISE EXCEPTION 'Market ID % does not exist', target_market_id;
    END IF;

    -- Perform similarity search using the fetched embedding
    RETURN QUERY
    SELECT m.market_id, vector_similarity(input_embedding, me.embedding) AS similarity_score
    FROM market_embeddings me
    JOIN markets m ON me.market_id = m.id
    WHERE me.market_id != target_market_id AND
          vector_distance(input_embedding, me.embedding) <= threshold
    ORDER BY similarity_score DESC
    LIMIT limit_count;
END;
$$ LANGUAGE plpgsql STABLE;

-- === FIND QUESTIONS SIMILAR TO TARGET ===
CREATE OR REPLACE FUNCTION find_similar_questions_by_id(
    target_question_id INTEGER,
    threshold FLOAT,
    limit_count INTEGER DEFAULT 10
)
RETURNS TABLE(question_id INTEGER, similarity_score FLOAT) AS $$
DECLARE
    input_embedding VECTOR(768);
BEGIN
    -- Fetch the embedding of the specified question
    SELECT qe.embedding INTO input_embedding
    FROM question_embeddings qe
    WHERE qe.question_id = target_question_id;

    IF input_embedding IS NULL THEN
        RAISE EXCEPTION 'Question ID % does not exist', target_question_id;
    END IF;

    -- Perform similarity search using the fetched embedding
    RETURN QUERY
    SELECT q.question_id, vector_similarity(input_embedding, qe.embedding) AS similarity_score
    FROM question_embeddings qe
    JOIN questions q ON qe.question_id = q.id
    WHERE qe.question_id != target_question_id AND
          vector_distance(input_embedding, qe.embedding) <= threshold
    ORDER BY similarity_score DESC
    LIMIT limit_count;
END;
$$ LANGUAGE plpgsql STABLE;
