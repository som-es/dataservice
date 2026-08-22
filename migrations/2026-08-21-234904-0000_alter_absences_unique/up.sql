-- Your SQL goes here
ALTER TABLE absences
    DROP CONSTRAINT absences_delegate_id_plenary_session_id_key;

ALTER TABLE absences
    ADD CONSTRAINT absences_delegate_id_plenary_session_id_absence_date_key
    UNIQUE (delegate_id, plenary_session_id, absence_date);

ALTER TABLE debates
    ALTER COLUMN about TYPE text;
