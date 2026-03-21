-- Add decree email notification settings to somes_user table
ALTER TABLE somes_user
ADD COLUMN send_new_decree_mails boolean not null default false,
ADD COLUMN send_new_decree_by_favo_mails boolean not null default false,
ADD COLUMN send_new_vote_result_by_favo_mails boolean not null default false,
ADD COLUMN send_new_proposal_mails boolean not null default false,
ADD COLUMN send_new_proposal_by_favo_mails boolean not null default false;

-- Create table to track which decree emails have been sent
create table decree_email_info(
    id serial primary key,
    already_sent_id int not null,
    foreign key (already_sent_id) references ministrial_decrees(id) on delete cascade,
    unique(already_sent_id)
);
