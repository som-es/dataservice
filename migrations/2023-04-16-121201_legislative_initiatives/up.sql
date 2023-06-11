-- Your SQL goes here
create table legislative_initiatives (
    -- use serial id -> add inr, and UNIQUE of (inr, ityp, gp)
    id varchar(20) primary key,
    ityp varchar(7) not null,
    gp varchar(7) not null,
    emphasis varchar(4000),
    title text not null,
    description text not null,
    accepted boolean not null default false,
    created_at timestamptz not null default now()
);