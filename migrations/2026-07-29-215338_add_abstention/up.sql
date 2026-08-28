-- Your SQL goes here
ALTER TABLE named_votes
    ADD COLUMN was_abstention boolean NOT NULL DEFAULT false;
