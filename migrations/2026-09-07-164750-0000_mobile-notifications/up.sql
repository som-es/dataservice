create table user_notification_settings (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id int not null references somes_user(id) on delete cascade,
    platform varchar(20) not null,
    send_new_vote_results boolean not null default true,
    send_new_vote_result_by_favo boolean not null default false,
    send_new_delegate_activity boolean not null default true,
    send_new_ministrial_prop boolean not null default false,
    send_new_ministrial_prop_by_favo boolean not null default false,
    send_new_decree boolean not null default false,
    send_new_decree_by_favo boolean not null default false,
    send_new_proposal boolean not null default false,
    send_new_proposal_by_favo boolean not null default false,
    created_at timestamptz not null default now(),
    updated_at timestamptz,
    unique (user_id, platform)
);

insert into user_notification_settings (
    user_id,
    platform,
    send_new_vote_results,
    send_new_vote_result_by_favo,
    send_new_delegate_activity,
    send_new_ministrial_prop,
    send_new_ministrial_prop_by_favo,
    send_new_decree,
    send_new_decree_by_favo,
    send_new_proposal,
    send_new_proposal_by_favo
)
select
    id,
    'web',
    send_new_vote_results_mails,
    send_new_vote_result_by_favo_mails,
    send_new_delegate_activity_mails,
    send_new_ministrial_prop_mails,
    send_new_ministrial_prop_by_favo_mails,
    send_new_decree_mails,
    send_new_decree_by_favo_mails,
    send_new_proposal_mails,
    send_new_proposal_by_favo_mails
from somes_user;

alter table somes_user
drop column send_new_vote_results_mails,
drop column send_new_vote_result_by_favo_mails,
drop column send_new_delegate_activity_mails,
drop column send_new_ministrial_prop_mails,
drop column send_new_ministrial_prop_by_favo_mails,
drop column send_new_decree_mails,
drop column send_new_decree_by_favo_mails,
drop column send_new_proposal_mails,
drop column send_new_proposal_by_favo_mails;

create table expo_push_tokens (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id int not null references somes_user(id) on delete cascade,
    notification_settings_id bigint not null references user_notification_settings(id) on delete cascade,
    push_token varchar(255) not null,
    platform varchar(20) not null,
    enabled boolean not null default true,
    created_at timestamptz not null default now(),
    updated_at timestamptz,
    unique (push_token)
);

create index expo_push_tokens_user_id_idx on expo_push_tokens (user_id);
create index expo_push_tokens_notification_settings_idx on expo_push_tokens (notification_settings_id);
