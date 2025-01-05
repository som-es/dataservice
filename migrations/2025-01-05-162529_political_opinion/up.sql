-- Your SQL goes here
create table political_questions(
    id serial primary key,
    question text not null,
    is_left boolean not null,
    is_liberal boolean not null
);

create table political_answers(
    id serial primary key,
    question_id int not null,
    delegate_id int not null,
    answer text not null,
    FOREIGN KEY (delegate_id) REFERENCES delegates(id),
    FOREIGN KEY (question_id) REFERENCES political_questions(id)
);

create table political_opinions(
    id serial primary key,
    delegate_id int not null,
    question_id int not null,
    answer_id int not null,
    opinion varchar(10) not null,
    strong_reference_score float not null,
    FOREIGN KEY (delegate_id) REFERENCES delegates(id),
    FOREIGN KEY (question_id) REFERENCES political_questions(id),
    FOREIGN KEY (answer_id) REFERENCES political_answers(id)
);