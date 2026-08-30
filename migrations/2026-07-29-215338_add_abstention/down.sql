-- This file should undo anything in `up.sql`
ALTER TABLE named_votes
    DROP CONSTRAINT infavor_null_iff_not_voting;

ALTER TABLE named_votes
    DROP COLUMN was_abstention;
