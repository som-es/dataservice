-- Your SQL goes here
create table legislative_initiatives (
    -- use serial id -> add inr, and UNIQUE of (inr, ityp, gp)
    id serial primary key,
    ityp varchar(7) not null,
    doktyp varchar(20),
    gp varchar(7) not null,
    inr int not null,
    emphasis varchar(6000),
    is_emphasis_ai_generated boolean default false,
    title text not null,
    pre_declined_type varchar(10),
    voted_by_name boolean not null default false,
    description text not null,
    accepted varchar(10),
    -- rename this to handled_at
    created_at date not null default now(),
    appeared_at timestamp,
    updated_at timestamp,
    requires_simple_majority boolean default true,
    -- was_invisibly_declined boolean default false,
    plenary_session_id int,
    notified_with_mail boolean default false,
    is_law boolean default false,
    law_accepted boolean,
    law_come_into_effect_date date,
    law_expires_on_date date,
    by_publication boolean,
    foreign key (plenary_session_id) references plenar_infos(id),
    UNIQUE (inr, ityp, gp)
);