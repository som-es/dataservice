-- Your SQL goes here
create table contacts(
    id serial primary key,
    mail varchar(255) unique,
    phone_number varchar(255) unique,
    twitter_url varchar(255) unique,
    facebook_url varchar(255) unique,
    website_url varchar(255) unique,
    tiktok_url varchar(255) unique,
    instagram_url varchar(255) unique,
    youtube_url varchar(255) unique
);

create table delegates (
    id INT primary key not null,
    name varchar(255) not null,
    party varchar(100),
    image_url varchar(300),
    constituency varchar(200),
    council varchar(200),
    seat_row int,
    seat_col int,
    contacts_id int,
    FOREIGN KEY (contacts_id) REFERENCES contacts(id)
);

CREATE TABLE plenar_infos (
    id varchar(20) primary key,
    add_date timestamptz not null,
    inr int not null,
    title varchar(255) not null,
    description varchar(255) not null
);

CREATE TABLE call_to_order (
    id varchar(20) primary key,
    plenar_id varchar(20) not null,
    inr int not null,
    receiver_id int not null,
    FOREIGN KEY (plenar_id) REFERENCES plenar_infos(id)
);

CREATE TABLE debates (
    id serial primary key,
    plenar_id varchar(20) not null,
    about varchar(512) not null,
    FOREIGN KEY (plenar_id) REFERENCES plenar_infos(id)
);

CREATE TABLE plenar_speeches (
    id serial primary key,
    delegate_id int not null,
    debate_id int not null,
    opinion varchar(40),
    spoken_text text,
    duration_in_seconds int,
    ty varchar(40),
    FOREIGN KEY (debate_id) REFERENCES debates(id),
    FOREIGN KEY (delegate_id) REFERENCES delegates(id)
);
