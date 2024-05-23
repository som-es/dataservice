-- Your SQL goes here
create table legislative_documents(
    id serial primary key,
    legislative_initiatives_id int not null,
    document_type varchar(100) not null,
    document_url varchar(255) not null,
    FOREIGN KEY (legislative_initiatives_id) REFERENCES legislative_initiatives(id),
    UNIQUE (legislative_initiatives_id, document_url)
);