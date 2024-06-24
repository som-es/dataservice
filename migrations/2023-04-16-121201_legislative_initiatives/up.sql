-- Your SQL goes here
create table legislative_initiatives (
    -- use serial id -> add inr, and UNIQUE of (inr, ityp, gp)
    id serial primary key,
    ityp varchar(7) not null,
    gp varchar(7) not null,
    inr int not null,
    emphasis varchar(4000),
    title text not null,
    description text not null,
    accepted boolean,
    -- rename this to handled_at
    created_at date not null default now(),
    -- created_at timestamptz not null,
    updated_at timestamptz,
    UNIQUE (inr, ityp, gp)
);