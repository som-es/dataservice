-- Your SQL goes here
create table votes (
    id serial primary key,
    party varchar(30) not null,
    votes int not null,
    infavor boolean not null,
    legislative_initiatives_id varchar(20) not null,
    FOREIGN KEY (legislative_initiatives_id) REFERENCES legislative_initiatives(id),
);