-- This file should undo anything in `up.sql`
ALTER TABLE user_topics
ALTER COLUMN topic_id TYPE INT;

ALTER TABLE user_topics
ADD CONSTRAINT user_topics_topic_id_fkey
FOREIGN KEY (topic_id)
REFERENCES unique_eurovoc_topics(id);

alter table unique_eurovoc_topics drop constraint user_topics_unique_id_language;
alter table unique_eurovoc_topics drop COLUMN id_as_hash;
alter table unique_eurovoc_topics drop COLUMN eurovoc_type;
alter table unique_eurovoc_topics drop COLUMN eurovoc_id;
alter table unique_eurovoc_topics drop COLUMN language;
