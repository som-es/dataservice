-- Your SQL goes here
CREATE TABLE dates (
    id SERIAL PRIMARY KEY,
    date DATE not null,
    date_sort TIMESTAMP,
    time TIME,
    title TEXT not null,
    link TEXT,
    appointment_type TEXT not null,
    appointment_type_display TEXT,
    info_link TEXT,
    location TEXT,
    committee TEXT not null,
    year_month TEXT,
    repeated_date DATE,
    month_year TEXT,
    media_relevant BOOLEAN NOT NULL DEFAULT FALSE,
    start TIMESTAMP,
    "end" TIMESTAMP,
    past BOOLEAN NOT NULL DEFAULT FALSE,
    day_with_date TEXT,
    tour_format TEXT,
    UNIQUE (date, title, appointment_type, committee, media_relevant)
);

CREATE TABLE dates_topics (
    id SERIAL PRIMARY KEY,
    date_id INT NOT NULL REFERENCES dates(id) ON DELETE CASCADE,
    topic TEXT NOT NULL,
    UNIQUE (date_id, topic)
);
