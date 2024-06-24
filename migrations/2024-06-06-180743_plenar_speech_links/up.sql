-- Your SQL goes here
create table plenar_speech_links(
    id serial primary key,
    plenar_speech_id int not null,
    document_url varchar(500),
    FOREIGN KEY (plenar_speech_id) REFERENCES plenar_speeches(id),
    UNIQUE (plenar_speech_id, document_url)
);

create table proposal_documents(
    id serial primary key,
    proposal_id varchar(20),
    title varchar(1024),
    document_type varchar(100) not null,
    document_url varchar(500) not null,
    FOREIGN KEY (proposal_id) REFERENCES proposals(id),
    UNIQUE (proposal_id, document_url)
);
