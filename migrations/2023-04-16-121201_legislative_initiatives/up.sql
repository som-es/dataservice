-- Your SQL goes here
create table legislative_initiatives (
    id varchar(20) primary key,
    ityp varchar(7) not null,
    -- gp varchar(7) not null,
    title text not null,
    description text not null,
    accepted boolean not null default false,
    created_at timestamptz not null default now()
);