-- Your SQL goes here
alter table unique_eurovoc_topics add column language text not null default 'de';
alter table unique_eurovoc_topics add column eurovoc_id text;
alter table unique_eurovoc_topics add column eurovoc_type text;
alter table unique_eurovoc_topics add column id_as_hash bigint;
alter table unique_eurovoc_topics add constraint user_topics_unique_id_language unique (id_as_hash, language);

ALTER TABLE user_topics
DROP CONSTRAINT user_topics_topic_id_fkey;

ALTER TABLE user_topics
ALTER COLUMN topic_id TYPE BIGINT;
