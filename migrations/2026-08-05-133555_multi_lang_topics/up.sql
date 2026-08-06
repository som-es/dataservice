-- Your SQL goes here
alter table unique_eurovoc_topics add column language text not null default 'de';
alter table unique_eurovoc_topics add column eurovoc_id text;
alter table unique_eurovoc_topics add column eurovoc_type text;
