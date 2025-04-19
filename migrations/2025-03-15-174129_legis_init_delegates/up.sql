-- Your SQL goes here
create table legis_init_delegates(
    id serial primary key,
    legis_init_id int not null,
    delegate_text varchar(255),
    delegate_id int not null,
    unique(legis_init_id, delegate_id),
    foreign key (legis_init_id) references legislative_initiatives(id),
    foreign key (delegate_id) references delegates(id)
);
