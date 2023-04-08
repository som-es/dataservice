-- Your SQL goes here
create table delegates (
    id INT primary key not null,
    name varchar(255) not null,
    party varchar(100) not null,
    constituency varchar(200) not null,
    council varchar(200) not null,
    seat_row int not null,
    seat_col int not null
);