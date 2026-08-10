ALTER TABLE votes
    ADD COLUMN fraction INTEGER,
    ADD COLUMN infavor BOOLEAN;

UPDATE votes
SET fraction = infavor_count + against_count + abstention_count + absence_count,
    infavor = infavor_count >= against_count;

ALTER TABLE votes
    ALTER COLUMN fraction SET NOT NULL,
    ALTER COLUMN infavor SET NOT NULL;

ALTER TABLE votes
    DROP COLUMN code,
    DROP COLUMN infavor_count,
    DROP COLUMN against_count,
    DROP COLUMN abstention_count,
    DROP COLUMN absence_count;
