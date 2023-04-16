-- Your SQL goes here
create table delegates (
    id INT primary key not null,
    name varchar(255) not null,
    party varchar(100),
    constituency varchar(200),
    council varchar(200),
    seat_row int,
    seat_col int
);