-- Your SQL goes here
create table article_legis_init_links(
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    legis_init_id int not null references legislative_initiatives(id),
    provider text not null,
    url text not null,
    score float not null,
    lastmod timestamptz not null,
    unique (legis_init_id, url)
);
