-- Your SQL goes here
create table proposal_email_info(
    id serial primary key,
    already_sent_id int not null,
    marked_on timestamptz default now(),
    foreign key (already_sent_id) references legislative_initiatives(id) on delete cascade
);
