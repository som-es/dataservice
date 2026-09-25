-- This file should undo anything in `up.sql`
drop table ris_amendments;
alter table ministrial_decrees drop column is_norm;
alter table ministrial_decrees alter column gov_official_id set not null;
alter table ministrial_decrees drop column gesetzesnummer;
