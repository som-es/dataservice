-- Your SQL goes here
create table proposals (
    id varchar(20) primary key,
    ityp varchar(7) not null,
    gp varchar(7) not null,
    title text not null,
    description text not null,
    
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);