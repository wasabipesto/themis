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
