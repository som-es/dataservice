-- Your SQL goes here
create table political_positions(
    id serial primary key,
    delegate_id int not null,
    is_left float not null,
    is_not_left float not null,
    is_liberal float not null,
    is_not_liberal float not null,
    neutral_count int not null,
    UNIQUE(delegate_id),
    FOREIGN KEY (delegate_id) REFERENCES delegates(id)
)
