-- Your SQL goes here
ALTER TABLE votes
    ADD COLUMN infavor_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN against_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN abstention_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN absence_count INTEGER NOT NULL DEFAULT 0;

UPDATE votes
SET infavor_count = CASE WHEN infavor THEN fraction ELSE 0 END,
    against_count = CASE WHEN infavor THEN 0 ELSE fraction END;

ALTER TABLE votes
    DROP COLUMN fraction,
    DROP COLUMN infavor;

ALTER TABLE votes
    ALTER COLUMN infavor_count DROP DEFAULT,
    ALTER COLUMN against_count DROP DEFAULT,
    ALTER COLUMN abstention_count DROP DEFAULT,
    ALTER COLUMN absence_count DROP DEFAULT;
