drop index if exists expo_push_tokens_notification_settings_idx;
drop index if exists expo_push_tokens_user_id_idx;
drop table if exists expo_push_tokens;

delete from user_notification_settings where platform <> 'web';

alter table somes_user
add column send_new_vote_results_mails boolean not null default true,
add column send_new_vote_result_by_favo_mails boolean not null default false,
add column send_new_delegate_activity_mails boolean not null default true,
add column send_new_ministrial_prop_mails boolean not null default false,
add column send_new_ministrial_prop_by_favo_mails boolean not null default false,
add column send_new_decree_mails boolean not null default false,
add column send_new_decree_by_favo_mails boolean not null default false,
add column send_new_proposal_mails boolean not null default false,
add column send_new_proposal_by_favo_mails boolean not null default false;

update somes_user u
set
    send_new_vote_results_mails = s.send_new_vote_results,
    send_new_vote_result_by_favo_mails = s.send_new_vote_result_by_favo,
    send_new_delegate_activity_mails = s.send_new_delegate_activity,
    send_new_ministrial_prop_mails = s.send_new_ministrial_prop,
    send_new_ministrial_prop_by_favo_mails = s.send_new_ministrial_prop_by_favo,
    send_new_decree_mails = s.send_new_decree,
    send_new_decree_by_favo_mails = s.send_new_decree_by_favo,
    send_new_proposal_mails = s.send_new_proposal,
    send_new_proposal_by_favo_mails = s.send_new_proposal_by_favo
from user_notification_settings s
where s.user_id = u.id and s.platform = 'web';

drop table if exists user_notification_settings;
