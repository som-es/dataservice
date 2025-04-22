-- Your SQL goes here

create table ministrial_decrees(
    id serial primary key,
    gov_official_id int not null,
    ris_id varchar(255) not null,
    ministrial_issuer varchar(1024) not null,
    title varchar(1024) not null,
    short_title varchar(512) not null,
    publication_date date not null,
    part varchar(255) not null,
    unique(gov_official_id),
    foreign key (gov_official_id) references delegate(id)
)

create table ministrial_decrees_documents(
    id serial primary key,
    ministrial_decree_id int not null,
    title varchar(1024),
    document_type varchar(128) not null,
    document_url varchar(512) not null,
    FOREIGN KEY (ministrial_decree_id) REFERENCES ministrial_decrees(id),
    UNIQUE (ministrial_decree_id, document_url)
)
