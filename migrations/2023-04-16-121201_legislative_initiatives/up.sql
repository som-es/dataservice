-- Your SQL goes here
create table legislative_initiatives (
    -- use serial id -> add inr, and UNIQUE of (inr, ityp, gp)
    id serial primary key,
    ityp varchar(7) not null,
    gp varchar(7) not null,
    inr int not null,
    emphasis varchar(6000),
    title text not null,
    description text not null,
    accepted boolean,
    -- rename this to handled_at
    created_at date not null default now(),
    appeared_at timestamptz,
    updated_at timestamptz,
    requires_simple_majority boolean default false,
    was_invisibly_declined boolean default false,
    UNIQUE (inr, ityp, gp)
);