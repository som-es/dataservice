-- Your SQL goes here
create table unique_topics (
    id serial primary key,
    topic_name varchar(255) not null,
    unique(topic_name)
);

create table user_topics(
    id serial primary key,
    user_id int not null,
    topic_id int not null,
    FOREIGN KEY (user_id) REFERENCES somes_user(id),
    FOREIGN KEY (topic_id) REFERENCES unique_topics(id),
    UNIQUE (user_id, topic_id)
);