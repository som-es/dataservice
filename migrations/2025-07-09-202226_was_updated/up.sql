-- Your SQL goes here
create table legis_init_was_updated(
    id serial primary key,
    legis_init_id int not null,
    updated_at timestamptz not null default now(),
    unique(legis_init_id),
    foreign key(legis_init_id) references legislative_initiatives(id)
);