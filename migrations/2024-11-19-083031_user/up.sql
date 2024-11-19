-- Your SQL goes here
create table somes_user(
    id serial primary key,
    email varchar(356) not null unique,
    is_email_hashed boolean not null default false
);