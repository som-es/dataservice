-- Your SQL goes here
create table users(
    id serial primary key,
    username varchar(255) not null unique,
    email varchar(300) not null unique,
    password_hash varchar(356) not null
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
    FOREIGN KEY (issuer_id) REFERENCES users(id),
    FOREIGN KEY (delegate_id) REFERENCES delegates(id),
    UNIQUE (issuer_id, delegate_id, title, body)
);
