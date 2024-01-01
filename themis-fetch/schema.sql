CREATE TABLE Market (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL,
    platform VARCHAR NOT NULL,
    platform_id VARCHAR NOT NULL,
    url VARCHAR NOT NULL
);