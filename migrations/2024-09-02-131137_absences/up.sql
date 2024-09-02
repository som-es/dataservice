-- Your SQL goes here
-- duplication of named_votes (lev, similiarty_score, ...) -> create "delegate_matching" table or something like this
create table absences(
    id serial primary key,
    lev bigint not null,
    similiarity_score bigint not null,
    searched_with varchar(255),
    matched_with varchar(255) not null,
    delegate_id int not null,
    manually_matched boolean default false,

    plenary_session_id int not null,
    unique(delegate_id, plenary_session_id),
    foreign key (planery_session_id) references plenar_infos(id),
    foreign key (delegate_id) references delegates(id)
);