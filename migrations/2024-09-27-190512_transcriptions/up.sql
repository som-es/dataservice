-- Your SQL goes here
CREATE TABLE introduction_transcriptions(
    id SERIAL PRIMARY KEY,
    delegate_id INT REFERENCES delegates(id)
);

Create Table transcriptionsQA(
    id SERIAL PRIMARY KEY,
    question TEXT not null,
    answer TEXT not null,
    transcription_id INT not null REFERENCES introduction_transcriptions(id)
);