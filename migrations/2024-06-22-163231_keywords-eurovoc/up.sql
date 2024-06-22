-- Your SQL goes here
--  table other_keyword_topics;
--  table eurovoc_topics;

create table other_keyword_topics(
    id serial primary key,
    topic varchar(255) not null,
    legislative_initiatives_id int not null,
    FOREIGN KEY (legislative_initiatives_id) REFERENCES legislative_initiatives(id),
    UNIQUE (legislative_initiatives_id, topic)
);

create table eurovoc_topics(
    id serial primary key,
    topic varchar(255) not null,
    legislative_initiatives_id int not null,
    FOREIGN KEY (legislative_initiatives_id) REFERENCES legislative_initiatives(id),
    UNIQUE (legislative_initiatives_id, topic)
);
