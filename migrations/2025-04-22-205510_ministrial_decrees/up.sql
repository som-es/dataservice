-- Your SQL goes here

create table ministrial_decrees(
    id serial primary key,
    gov_official_id int not null,
    ris_id varchar(255) not null,
    ministrial_issuer varchar(1024) not null,
    title varchar(16384) not null,
    short_title varchar(1024) not null,
    publication_date date not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz,
    part varchar(255) not null,
    emphasis varchar(16384),
    eli varchar(1024),
    document_url varchar(1024),
    gp varchar(255),
    hash bigint,
    unique(ris_id),
    foreign key (gov_official_id) references delegates(id)
);

create table ministrial_decrees_documents(
    id serial primary key,
    ministrial_decree_id int not null,
    title varchar(1024),
    document_type varchar(128) not null,
    document_url varchar(512) not null,
    FOREIGN KEY (ministrial_decree_id) REFERENCES ministrial_decrees(id),
    UNIQUE (ministrial_decree_id, document_url)
);
