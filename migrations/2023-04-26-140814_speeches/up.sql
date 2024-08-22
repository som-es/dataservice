-- Your SQL goes here
CREATE TABLE speeches (
    id serial primary key,
    delegate_id int not null,
    infavor boolean,
    opinion varchar(255),
    legislative_initiatives_id int not null,
    FOREIGN KEY (legislative_initiatives_id) REFERENCES legislative_initiatives(id),
    FOREIGN KEY (delegate_id) REFERENCES delegates(id),
    UNIQUE (delegate_id, legislative_initiatives_id)
);