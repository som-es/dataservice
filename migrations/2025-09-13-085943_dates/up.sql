-- Your SQL goes here
CREATE TABLE dates (
    id SERIAL PRIMARY KEY,
    date DATE,
    date_sort TIMESTAMP,
    time TIME,
    title TEXT,
    link TEXT,
    appointment_type TEXT,
    appointment_type_display TEXT,
    info_link TEXT,
    location TEXT,
    committee TEXT,
    year_month TEXT,
    repeated_date DATE,
    month_year TEXT,
    media_relevant BOOLEAN NOT NULL,
    start TIMESTAMP,
    "end" TIMESTAMP,
    past BOOLEAN NOT NULL,
    day_with_date TEXT,
    tour_format TEXT
);

CREATE TABLE dates_topics (
    id SERIAL PRIMARY KEY,
    date_id INT NOT NULL REFERENCES dates(id) ON DELETE CASCADE,
    topic TEXT NOT NULL
);
