-- This file should undo anything in `up.sql`
alter table absences drop column absence_date;
alter table absences drop column document_url;
