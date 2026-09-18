-- Only the parent keys needed by the actual delegate-question migration.
-- These are deliberately not a replacement for the full application schema.
CREATE TABLE somes_user (id INTEGER PRIMARY KEY);
CREATE TABLE delegates (id INTEGER PRIMARY KEY);
INSERT INTO somes_user VALUES (1);
INSERT INTO delegates VALUES (1);
