-- Your SQL goes here
create table political_questions(
    id serial primary key,
    question text not null,
    is_left boolean,
    is_liberal boolean,
    unique (question)
);

create table political_answers(
    id serial primary key,
    question_id int not null,
    delegate_id int,
    answer text not null,
    stance_llm varchar(255) not null,
    is_strong_reference boolean,
    model_used varchar(255),
    FOREIGN KEY (delegate_id) REFERENCES delegates(id),
    FOREIGN KEY (question_id) REFERENCES political_questions(id),
    UNIQUE(stance_llm, is_strong_reference, question_id)
);

create table political_opinions(
    id serial primary key,
    delegate_id int not null,
    question_id int not null,
    answer_id int not null,
    stance varchar(255) not null,
    stance_acc float not null,
    strong_reference_score float not null,
    FOREIGN KEY (delegate_id) REFERENCES delegates(id),
    FOREIGN KEY (question_id) REFERENCES political_questions(id),
    FOREIGN KEY (answer_id) REFERENCES political_answers(id)
);