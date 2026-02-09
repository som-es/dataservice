-- Your SQL goes here
create table contacts(
    id int primary key,
    mail varchar(255),
    phone_number varchar(255),
    twitter_url varchar(255),
    facebook_url varchar(255),
    website_url varchar(255),
    tiktok_url varchar(255),
    instagram_url varchar(255),
    youtube_url varchar(255),
    unique(twitter_url, facebook_url, website_url, youtube_url, instagram_url, tiktok_url)
);

create table delegates (
    id INT primary key not null,
    name varchar(255) not null,
    prev_name varchar(255),
    prev_name_till date,
    party varchar(100),
    image_url varchar(300),
    constituency varchar(200),
    council varchar(200),
    seat_row int default null,
    seat_col int default null,
    gender varchar(1),
    is_active boolean,
    birthdate date,
    created_at timestamptz not null default now(),
    updated_at timestamptz,
    hash bigint,
    FOREIGN KEY (id) REFERENCES contacts(id)
);
