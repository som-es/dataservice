-- Your SQL goes here
CREATE TABLE speeches (
    delegate_id int not null,
    infavor boolean,
    legislative_initiatives_id varchar(20) not null,
    FOREIGN KEY (legislative_initiatives_id) REFERENCES legislative_initiatives(id),
    FOREIGN KEY (delegate_id) REFERENCES delegates(id),
    PRIMARY KEY (delegate_id, legislative_initiatives_id)
);