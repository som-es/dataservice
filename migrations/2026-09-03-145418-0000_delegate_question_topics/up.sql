-- Your SQL goes here
CREATE TABLE delegate_question_topics (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    question_id BIGINT NOT NULL REFERENCES delegate_questions(id) ON DELETE CASCADE,
    topic_id BIGINT NOT NULL,
    UNIQUE (question_id, topic_id)
);
