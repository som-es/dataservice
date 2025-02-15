-- Your SQL goes here

create table legis_email_info(
    id serial primary key,
    already_sent_id int not null,
    marked_on timestamp default now(),
    foreign key (already_sent_id) references legislative_initiatives(id)
);