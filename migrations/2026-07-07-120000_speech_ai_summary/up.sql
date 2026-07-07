-- Your SQL goes here
create table speech_ai_summaries (
    id serial primary key,
    speech_id integer not null references plenar_speeches(id) on delete cascade,
    short_title text not null,
    one_sentence_short_summary text not null,
    very_short_summary text not null,
    short_summary text not null,
    summary text not null,
    detailed_summary text not null,
    very_detailed_summary text not null,
    full_speech_summary jsonb not null,
    model_used text not null,
    version text not null,
    generated_at timestamptz not null default now(),
    unique (speech_id)
);

create table speech_proposal_relations (
    id serial primary key,
    speech_ai_summary_id integer not null references speech_ai_summaries(id) on delete cascade,
    legis_init_id integer not null references legislative_initiatives(id) on delete cascade,
    full_speech_relations jsonb not null,
    model_used text not null,
    version text not null,
    generated_at timestamptz not null default now(),
    unique (speech_ai_summary_id, legis_init_id)
);
