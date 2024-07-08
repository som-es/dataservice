-- Your SQL goes here
-- drop table other_keyword_topics_proposals;
-- drop table eurovoc_topics_proposals;
-- drop table topics_proposals;
-- drop table ministrial_proposals;

create table ministrial_proposals(
    id serial primary key,
    ityp varchar(7) not null,
    gp varchar(14) not null,
    inr int not null,
    emphasis varchar(6000),
    title text not null,
    description text not null,
    created_at timestamptz not null,
    updated_at timestamptz,
    due_to date not null,
    ressort varchar(255), 
    ressort_shortform varchar(12),
    legis_init_gp varchar(14),
    legis_init_inr int,
    legis_init_ityp varchar(7),
    -- FOREIGN KEY (legis_init_gp, legis_init_inr, legis_init_ityp) REFERENCES legislative_initiatives(gp, inr, ityp),
    UNIQUE (inr, ityp, gp)
);

create table ministrial_proposals_documents(
    id serial primary key,
    ministrial_proposal_id int not null,
    title varchar(1024),
    document_type varchar(100) not null,
    document_url varchar(500) not null,
    FOREIGN KEY (ministrial_proposal_id) REFERENCES ministrial_proposals(id),
    UNIQUE (ministrial_proposal_id, document_url)
);

-- having these as seperate tables without a "type" column
-- these tables could get extremly large with a type column 
create table other_keyword_topics_proposals(
    id serial primary key,
    topic varchar(255) not null,
    proposals_id varchar(20) not null,
    FOREIGN KEY (proposals_id) REFERENCES proposals(id),
    UNIQUE (proposals_id, topic)
);

create table eurovoc_topics_proposals(
    id serial primary key,
    topic varchar(255) not null,
    proposals_id varchar(20) not null,
    FOREIGN KEY (proposals_id) REFERENCES proposals(id),
    UNIQUE (proposals_id, topic)
);

create table topics_proposals(
    id serial primary key,
    topic varchar(255) not null,
    proposals_id varchar(20) not null,
    FOREIGN KEY (proposals_id) REFERENCES proposals(id),
    UNIQUE (proposals_id, topic)
);

-- this time for consistency
create table other_keyword_topics_ministrial_proposals(
    id serial primary key,
    topic varchar(255) not null,
    ministrial_proposal_id int not null,
    FOREIGN KEY (ministrial_proposal_id) REFERENCES ministrial_proposals(id),
    UNIQUE (ministrial_proposal_id, topic)
);

create table eurovoc_topics_ministrial_proposals(
    id serial primary key,
    topic varchar(255) not null,
    ministrial_proposal_id int not null,
    FOREIGN KEY (ministrial_proposal_id) REFERENCES ministrial_proposals(id),
    UNIQUE (ministrial_proposal_id, topic)
);

create table topics_ministrial_proposals(
    id serial primary key,
    topic varchar(255) not null,
    ministrial_proposal_id int not null,
    FOREIGN KEY (ministrial_proposal_id) REFERENCES ministrial_proposals(id),
    UNIQUE (ministrial_proposal_id, topic)
);
