-- This file should undo anything in `up.sql`
alter table unique_topics drop constraint unique_topics_unique_id_language;
alter table unique_topics drop COLUMN id_as_hash;
alter table unique_topics drop COLUMN eurovoc_type;
alter table unique_topics drop COLUMN eurovoc_id;
alter table unique_topics drop COLUMN language;
