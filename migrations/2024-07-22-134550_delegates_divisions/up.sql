-- Your SQL goes here
create table delegates_divisions(
    id serial primary key,
    delegate_id int not null,
    division varchar(255),
    FOREIGN KEY (delegate_id) REFERENCES delegates(id)
);
