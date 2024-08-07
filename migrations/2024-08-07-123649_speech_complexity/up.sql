-- Your SQL goes here
CREATE TABLE speech_complexity (
    id SERIAL PRIMARY KEY,
    speech_id INTEGER NOT NULL,
    flesch_kincaid REAL,
    smog REAL,
    gunning_fog REAL,
    coleman_liau REAL,
    FOREIGN KEY (speech_id) REFERENCES plenar_speeches (id),
    unique (speech_id)
);