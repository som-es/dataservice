-- Your SQL goes here
create table seat_history(
    id serial primary key,
    delegate_id int references delegates(id),
    seat_col int not null,
    seat_row int not null,
    council varchar(255),
    gp varchar(255),
    insertion_date date default now(),
    unique (delegate_id, seat_col, seat_row, council)
);