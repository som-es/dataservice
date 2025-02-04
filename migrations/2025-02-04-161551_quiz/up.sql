-- Your SQL goes here
create table quiz(
    id serial primary key,
    title varchar(255),
    description varchar(2048),
    stars int,
    unique(title)
);

create table quiz_questions(
    id serial primary key,
    quiz_id int,
    question varchar(2048),
    answer1 varchar(255),
    answer2 varchar(255),
    answer3 varchar(255),
    answer4 varchar(255),
    unique (question),
    foreign key (quiz_id) references quiz(id)
);