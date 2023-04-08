-- Your SQL goes here
CREATE TABLE proposal_delegates (
    proposal_id varchar(20) not null,
    delegate_id int not null,
    FOREIGN KEY (proposal_id) REFERENCES proposals(id),
    FOREIGN KEY (delegate_id) REFERENCES delegates(id),
    PRIMARY KEY (proposal_id, delegate_id)
);