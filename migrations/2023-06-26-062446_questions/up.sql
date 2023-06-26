-- Your SQL goes here
create table users(
    id int primary key
);

create table questions(
    id serial primary key,
    -- mind keeping this id private!
    issuer_id int not null,
    delegate_id int not null,
    title varchar(255) not null,
    body text not null,
    created_at timestamp not null default now(),
    updated_at timestamp not null default now(),
    UNIQUE (issuer_id, delegate_id, title, body)
);
