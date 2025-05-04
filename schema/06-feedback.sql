-- ==========================================
-- THE BIZARRO TABLES:
-- ANYONE CAN WRITE, NO ONE CAN READ
-- ==========================================
--
-- === DROP TABLES IF EXISTING ===
DROP TABLE IF EXISTS newsletter_signups;

DROP TABLE IF EXISTS general_feedback;

-- === EMAIL NEWSLETTER SIGNUPS ===
CREATE TABLE newsletter_signups (
    email TEXT NOT NULL,
    date TIMESTAMPTZ NOT NULL DEFAULT NOW ()
);

-- === OTHER FEEDBACK: SUGGESTIONS, ISSUES, ETC. ===
CREATE TABLE general_feedback (
    email TEXT NOT NULL,
    feedback_type TEXT NOT NULL,
    feedback TEXT NOT NULL,
    date TIMESTAMPTZ NOT NULL DEFAULT NOW ()
);

-- ANYONE CAN GIVE FEEDBACK
GRANT INSERT ON TABLE newsletter_signups TO web_anon;

GRANT INSERT ON TABLE general_feedback TO web_anon;

-- BUT ONLY ADMINS CAN SEE IT
REVOKE
SELECT
    ON TABLE newsletter_signups
FROM
    web_anon;

REVOKE
SELECT
    ON TABLE general_feedback
FROM
    web_anon;
