-- Your SQL goes here
CREATE TABLE introduction_transcriptions(
    id SERIAL PRIMARY KEY,
    delegate_id INT REFERENCES delegates(id)
);

Create Table transcriptionsQA(
    id SERIAL PRIMARY KEY,
    question TEXT,
    answer TEXT,
    transcription_id INT REFERENCES introduction_transcriptions(id)
);