-- Your SQL goes here
create table mood (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    auto_mood double precision not null default 0,
    pre_aggregated_user_mood double precision
);

create table user_mood (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_mood double precision not null default 0,
    user_id int not null references somes_user(id),
    mood_id bigint not null references mood(id),
    created_at timestamptz not null default now(),
    updated_at timestamptz,
    unique(user_id, mood_id)
);

create table gov_prop_mood (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    gov_prop_id int not null references ministrial_proposals(id),
    mood_id bigint not null references mood(id),
    unique(gov_prop_id, mood_id)
);
