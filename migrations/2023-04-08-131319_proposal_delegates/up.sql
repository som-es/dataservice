-- Your SQL goes here
CREATE TABLE proposal_delegates (
    proposal_id varchar(20),
    delegate_id INT,
    FOREIGN KEY (proposal_id) REFERENCES proposals(id),
    FOREIGN KEY (delegate_id) REFERENCES delegates(id),
    PRIMARY KEY (proposal_id, delegate_id)
);