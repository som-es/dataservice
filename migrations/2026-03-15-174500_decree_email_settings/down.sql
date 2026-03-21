-- This file should undo anything in `up.sql`
drop table decree_email_info;
ALTER TABLE somes_user
DROP COLUMN send_new_decree_by_favo_mails,
DROP COLUMN send_new_decree_mails,
DROP COLUMN send_new_proposal_by_favo_mails,
DROP COLUMN send_new_proposal_mails;
