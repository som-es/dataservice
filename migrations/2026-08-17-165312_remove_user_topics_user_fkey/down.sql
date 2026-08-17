-- This file should undo anything in `up.sql`
alter table user_topics add foreign key (user_id) REFERENCES somes_user(id);
