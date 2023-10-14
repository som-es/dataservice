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
    title varchar(1024) not null,
    body text not null,
    created_at timestamp not null default now(),
    updated_at timestamp not null default now(),
    is_sent boolean not null default false,
    is_sendable boolean not null default false,
    is_answerable boolean not null default true,
    FOREIGN KEY (issuer_id) REFERENCES users(id),
    FOREIGN KEY (delegate_id) REFERENCES delegates(id),
    UNIQUE (issuer_id, delegate_id, title, body)
);

create table answers(
    id serial primary key,
    question_id int not null,
    delegate_id int, -- get from email if email - need to check whether the sender of answer is actually the delegate
    via_mail boolean,
    received_at timestamp not null,
    text_type varchar(10), -- is text mark down or not? 
    mail_subject varchar(1024),
    body text not null,
    foreign key (question_id) REFERENCES questions(id)
);