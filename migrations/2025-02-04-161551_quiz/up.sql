-- Your SQL goes here
create table quiz(
    id serial primary key,
    title varchar(255) not null,
    description varchar(2048),
    stars int default 0,
    unique(title)
);

create table quiz_questions(
    id serial primary key,
    quiz_id int,
    question varchar(2048) not null,
    answer1 varchar(255) not null,
    answer2 varchar(255) not null,
    answer3 varchar(255) not null,
    answer4 varchar(255) not null,
    unique (question),
    foreign key (quiz_id) references quiz(id)
);