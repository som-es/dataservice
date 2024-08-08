-- Your SQL goes here

create table named_vote_info(
    id serial primary key,
    legis_init_id int not null,
    pro_count int not null,
    contra_count int not null,
    given_vote_sum int not null,
    invalid_count int not null,
    unique(legis_init_id),
    foreign key (legis_init_id) references legislative_initiatives(id)
);

create table named_votes(
    id serial primary key,
    infavor boolean,
    was_absent boolean default false,
    lev bigint not null,
    similiarity_score bigint not null,
    searched_with varchar(255),
    matched_with varchar(255) not null,
    delegate_id int not null,
    named_vote_info_id int not null,
    manually_matched boolean default false,

    unique(delegate_id, named_vote_info_id),
    foreign key (named_vote_info_id) references named_vote_info(id),
    foreign key (delegate_id) references delegates(id)
);
