-- Your SQL goes here
create table proposals (
    id varchar(20) primary key,
    ityp varchar(7) not null,
    gp varchar(7) not null,
    title text not null,
    description text,
    created_at timestamp not null,
    updated_at timestamp
);