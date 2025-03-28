-- ==========================================
-- ROLE CREATION
-- ==========================================
-- Anonymous role for public read-only access
CREATE ROLE web_anon NOLOGIN;

-- Authenticator role - acts as a bridge between HTTP requests and database roles
-- Update this if you change the PostgREST login credentials
CREATE ROLE authenticator NOINHERIT LOGIN PASSWORD 'authenticator';

-- Administrator role for full database access
CREATE ROLE admin NOLOGIN;

-- ==========================================
-- ANONYMOUS USER PERMISSIONS
-- ==========================================
-- Grant schema usage to anonymous users
GRANT USAGE ON SCHEMA public TO web_anon;

-- Grant read-only access to existing tables
GRANT
SELECT
    ON ALL TABLES IN SCHEMA public TO web_anon;

-- Automatically grant read-only access to future tables
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT
SELECT
    ON TABLES TO web_anon;

-- ==========================================
-- ADMIN PERMISSIONS
-- ==========================================
-- Grant schema usage to administrators
GRANT USAGE ON SCHEMA public TO admin;

-- Grant full access to existing tables
GRANT ALL ON ALL TABLES IN SCHEMA public TO admin;

-- Grant full access to existing sequences
GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO admin;

-- Automatically grant full access to future tables
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO admin;

-- Automatically grant full access to future sequences
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO admin;

-- ==========================================
-- ROLE INHERITANCE
-- ==========================================
-- Allow authenticator to switch to anonymous role
GRANT web_anon TO authenticator;

-- Allow authenticator to switch to admin role
GRANT admin TO authenticator;
