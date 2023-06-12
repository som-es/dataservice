-- Your SQL goes here
create table contacts(
    id serial primary key,
    mail varchar(255) unique,
    phone_number varchar(255),
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
    gender varchar(1),
    is_active boolean,
    FOREIGN KEY (contacts_id) REFERENCES contacts(id)
);
