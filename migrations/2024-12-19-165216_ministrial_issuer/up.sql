-- Your SQL goes here
create table ministrial_issuer (
    id serial primary key,
    delegate_id int not null,
    ministrial_proposal int not null,
    unique (delegate_id, ministrial_proposal),
    FOREIGN KEY (delegate_id) REFERENCES delegates(id),
    FOREIGN KEY (ministrial_proposal) REFERENCES ministrial_proposals(id)
);

CREATE TABLE delegate_scores (
    id SERIAL PRIMARY KEY,
    timestamp TIMESTAMP DEFAULT NOW(), 
    score REAL NOT NULL,
    delegate_id INTEGER NOT NULL,
    CONSTRAINT fk_delegate
        FOREIGN KEY (delegate_id)
        REFERENCES delegates (id)
        ON DELETE CASCADE
); 