-- Your SQL goes here
CREATE TABLE proposal_delegates (
    proposal_id INT,
    delegate_id INT,
    FOREIGN KEY (proposal_id) REFERENCES proposals(id),
    FOREIGN KEY (product_id) REFERENCES products(id),
    PRIMARY KEY (order_id, product_id)
);