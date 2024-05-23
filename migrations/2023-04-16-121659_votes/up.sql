-- add party table
create table votes (
    party varchar(30) not null,
    fraction int not null,
    infavor boolean not null,
    legislative_initiatives_id int not null,
    FOREIGN KEY (legislative_initiatives_id) REFERENCES legislative_initiatives(id),
    PRIMARY KEY (party, legislative_initiatives_id),
    UNIQUE (party, legislative_initiatives_id)
);