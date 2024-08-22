-- Your SQL goes here
create table speeches_html_urls(
    id serial primary key,
    speech_id int not null,
    document_url varchar(500) not null,
    unique (speech_id, document_url),
    foreign key (speech_id) references speeches(id)
);
