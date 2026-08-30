-- Your SQL goes here
create table parties (
    id serial primary key,
    name varchar(255) not null,
    gp varchar(14) not null,
    code varchar(255) not null,
    fraction int not null,
    color varchar(7) not null,
    unique (code, gp, name)
);
