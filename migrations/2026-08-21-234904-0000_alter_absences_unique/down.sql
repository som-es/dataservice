-- This file should undo anything in `up.sql`
ALTER TABLE absences
    DROP CONSTRAINT absences_delegate_id_plenary_session_id_absence_date_key;

ALTER TABLE absences
    ADD CONSTRAINT absences_delegate_id_plenary_session_id_key
    UNIQUE (delegate_id, plenary_session_id);

ALTER TABLE debates
    ALTER COLUMN about TYPE character varying(512);
