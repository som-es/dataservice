-- Your SQL goes here
create table events (
    id serial primary key,
    title text not null,
    location text not null,
    event_date date not null,
    start_time time not null,
    description text not null,
    image text,
    requires_membership boolean default false,
    requires_registration boolean default false
);
