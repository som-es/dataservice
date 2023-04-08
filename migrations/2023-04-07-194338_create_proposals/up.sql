-- Your SQL goes here
create table proposals (
    id uuid primary key,
    title text not null,
    description text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    deleted_at timestamptz
);