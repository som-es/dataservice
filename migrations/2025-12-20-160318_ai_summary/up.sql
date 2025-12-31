-- Your SQL goes here
create table summaries (
    id serial primary key,
    full_summary jsonb not null,
    short_title text not null,
    short_summary text not null,
    detailed_summary text not null,
    complexity_scope_of_proposal text not null,
    model_used text not null,
    version text not null,
    generated_at timestamp not null default NOW()
);

create table generated_eurovoc_topics (
    id serial primary key,
    summary_id integer references summaries(id) on delete cascade,
    topic text not null
);

create table legislative_initiative_summaries (
    id serial primary key,
    legis_init_id integer references legislative_initiatives(id) on delete cascade,
    summary_id integer references summaries(id) on delete cascade
);

create table ministerial_proposal_summaries (
    id serial primary key,
    ministerial_proposal_id integer references ministrial_proposals(id) on delete cascade,
    summary_id integer references summaries(id) on delete cascade
);

create table decree_summaries (
    id serial primary key,
    decree_id integer references ministrial_decrees(id) on delete cascade,
    summary_id integer references summaries(id) on delete cascade
);

create table speech_summaries (
    id serial primary key,
    speech_id integer references plenar_speeches(id) on delete cascade,
    summary_id integer references summaries(id) on delete cascade
);
