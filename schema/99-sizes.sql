-- ==========================================
-- GET ALL TABLE AND MATLZD VIEW SIZES
-- ==========================================

SELECT
    relname AS table_name,
    pg_total_relation_size(c.oid) AS size_raw,
    pg_size_pretty(pg_total_relation_size(c.oid)) AS size_pretty
FROM
    pg_catalog.pg_class c
JOIN
    pg_catalog.pg_namespace n ON n.oid = c.relnamespace
WHERE
    n.nspname = 'public'
AND (
    c.relkind = 'r'  -- Tables
    OR c.relkind = 'm'  -- Materialized views
)
ORDER BY size_raw DESC;
