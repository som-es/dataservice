-- Your SQL goes here
create table topics_legis_init(
    id serial primary key,
    topic varchar(100) not null,
    legislative_initiatives_id int not null,
    FOREIGN KEY (legislative_initiatives_id) REFERENCES legislative_initiatives(id),
    UNIQUE (legislative_initiatives_id, topic)
);