alter table unique_topics add column language text not null default 'de';
alter table unique_topics add column eurovoc_id text;
alter table unique_topics add column eurovoc_type text;
alter table unique_topics add column id_as_hash bigint;
alter table unique_topics add constraint unique_topics_unique_id_language unique (id_as_hash, language);
alter table unique_topics add constraint unique_topics_unique_topic_name_language unique (topic_name, language);
alter table unique_topics drop constraint unique_topics_topic_name_key;
alter table unique_eurovoc_topics drop constraint unique_eurovoc_topics_topic_name_key;
alter table unique_eurovoc_topics add constraint unique_eurovoc_topics_unique_topic_name_language unique (topic_name, language);
