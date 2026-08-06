-- This file should undo anything in `up.sql`
alter table unique_eurovoc_topics drop COLUMN language;
alter table unique_eurovoc_topics drop COLUMN eurovoc_id;
alter table unique_eurovoc_topics drop COLUMN eurovoc_type;
