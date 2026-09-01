-- Your SQL goes here

create table volksbg_eintragungswoche (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    start_date date not null,
    end_date date not null,
    cut_off_date date,
    online_deadline_utc timestamptz,
    polling_stations_url text,
    unique (start_date, end_date)
);

create table volksbg (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    slug text not null,
    title text not null,
    description text,
    overview_url text not null,
    state text,
    ruling_date date,
    cut_off_date date,
    eintragungswoche bigint references volksbg_eintragungswoche(id),
    unique(slug)
);

create table volksbg_documents(
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    volksbg_id bigint not null references volksbg(id),
    title varchar(1024),
    document_type varchar(100) not null,
    document_url varchar(500) not null,
    UNIQUE (volksbg_id, document_url)
);
