-- Your SQL goes here
create table pqa_meta (
    id BIGSERIAL PRIMARY KEY,
    gp text not null,
    inr int not null,
    ityp text not null,
    title text not null,
    description text not null,
    hash bigint,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    raw_data_created_at timestamptz not null,
    raw_data_updated_at timestamptz,
    topics text[] not null,
    eurovoc_topics text[] not null,
    other_keyword_topics text[] not null,
    issuer_ids bigint[] not null,
    reciever_ids bigint[] not null,
    unique (gp, inr, ityp)
);
create table pqa_references(
    id serial primary key,
    pqa_meta_id bigint not null references pqa_meta(id),
    pqa_ref_meta_id bigint,
    pqa_gp text not null,
    pqa_inr int not null,
    pqa_ityp text not null,
    UNIQUE (pqa_gp, pqa_inr, pqa_ityp)
);
create table pqa_documents(
    id serial primary key,
    pqa_meta_id bigint not null,
    title varchar(1024),
    document_type varchar(100) not null,
    document_url varchar(500) not null,
    FOREIGN KEY (pqa_meta_id) REFERENCES pqa_meta(id),
    UNIQUE (pqa_meta_id, document_url)
);

CREATE TABLE pqa_answers (
    id BIGSERIAL PRIMARY KEY,
    pqa_meta_id bigint not null references pqa_meta(id),
    short_title TEXT NOT NULL,
    full_answer_summary TEXT NOT NULL,
    detailed_full_answer_summary TEXT NOT NULL,
    full_answer_entry JSONB NOT NULL,
    model_used TEXT NOT NULL,
    version TEXT NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL default now()
);
CREATE TABLE pqa_questions (
    id bigserial PRIMARY KEY,
    short_title TEXT NOT NULL,
    pqa_meta_id bigint not null references pqa_meta(id),
    once_sentence_short_info_summary TEXT NOT NULL,
    very_short_info_summary TEXT NOT NULL,
    short_info_summary TEXT NOT NULL,
    info_summary TEXT NOT NULL,
    detailed_info_summary TEXT NOT NULL,
    question_summary TEXT NOT NULL,
    detailed_question_summary TEXT NOT NULL,
    full_question_entry JSONB NOT NULL,
    model_used TEXT NOT NULL,
    version TEXT NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL default now()
);
