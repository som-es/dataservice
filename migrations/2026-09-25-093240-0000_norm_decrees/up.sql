-- Your SQL goes here
alter table ministrial_decrees add column is_norm boolean not null default false;
alter table ministrial_decrees alter column gov_official_id drop not null;
alter table ministrial_decrees add column gesetzesnummer text;

create table ris_amendments(
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    bgbl text not null,
    ris_id text not null,
    norm_id int not null references ministrial_decrees(id),
    decree_id int references ministrial_decrees(id),
    unique(norm_id, ris_id)
);
