alter table unique_topics add column language text not null default 'de';
alter table unique_topics add column eurovoc_id text;
alter table unique_topics add column eurovoc_type text;
alter table unique_topics add column id_as_hash bigint;
alter table unique_topics add constraint unique_topics_unique_id_language unique (id_as_hash, language);
