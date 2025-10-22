-- Your SQL goes here
create table somes_user(
    id serial primary key,
    email varchar(356) not null unique,
    is_email_hashed boolean not null default false,
    is_admin boolean not null default false,
    send_new_vote_results_mails boolean not null default true,
    send_new_delegate_activity_mails boolean not null default true,
    send_new_ministrial_prop_mails boolean not null default false
    send_new_ministrial_prop_by_favo_mails boolean not null default false
);
