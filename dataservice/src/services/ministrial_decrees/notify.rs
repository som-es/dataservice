use chrono_tz::Europe::Vienna;
use combx::Decree;
use sqlx::{PgPool, query, query_as, query_scalar};
use std::collections::HashMap;

use crate::{
    email::{MAILER, send_mail},
    ministrial_proposals::SimpleGovOfficial,
    push::{ExpoPushMessage, PUSH_CLIENT, PushTarget, send_expo_push_notifications, truncate_for_push},
};

/// Push is one item per message, so a phone push is capped tighter than the
/// email digest (which shows up to 5 entries).
const MAX_PUSH_PER_TOKEN: usize = 3;

pub async fn find_notifiers_decrees<'a>(
    pool: &PgPool,
    decrees: &'a [(i32, Decree)],
) -> HashMap<String, Vec<&'a (i32, Decree)>> {
    let ids = query!("select id, email, send_new_decree_mails, send_new_decree_by_favo_mails from somes_user where is_email_hashed is false and (send_new_decree_mails or send_new_decree_by_favo_mails)")
        .fetch_all(pool)
        .await
        .unwrap_or_default();
    let mut to_notify_emails = HashMap::new();
    for id in ids {
        let user_topics = query!("select topic_name from user_topics inner join unique_eurovoc_topics ut on ut.id_as_hash = topic_id where user_id = $1", id.id).fetch_all(pool).await.unwrap_or_default().into_iter().map(|x| x.topic_name).collect::<Vec<_>>();
        let favo_dels = query!(
            "select delegate_id from favo_dels where user_id = $1",
            id.id
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        let relevant_decrees = decrees
            .iter()
            .filter(|(_id, decree)| {
                if id.send_new_decree_mails {
                    let topics = decree
                        .ai_summary
                        .as_ref()
                        .map(|summary| summary.full_summary.topics.clone())
                        .unwrap_or_default();

                    for topic in &topics {
                        if user_topics.contains(topic) {
                            return true;
                        }
                    }
                }
                if id.send_new_decree_by_favo_mails {
                    // Check if user has the decree's gov official as favorite
                    favo_dels
                        .iter()
                        .find(|favo_del| decree.gov_official_id == Some(favo_del.delegate_id))
                        .is_some()
                } else {
                    false
                }
            })
            .collect::<Vec<_>>();

        if !relevant_decrees.is_empty() {
            to_notify_emails.insert(id.email, relevant_decrees);
        }
    }
    to_notify_emails
}

struct PushToNotify {
    push_token: String,
    user_id: i32,
    send_new_decree: bool,
    send_new_decree_by_favo: bool,
}

pub async fn find_push_notifiers_decrees<'a>(
    pool: &PgPool,
    decrees: &'a [(i32, Decree)],
) -> HashMap<String, Vec<(&'a (i32, Decree), &'static str)>> {
    let rows = query_as!(PushToNotify, "
        select ept.push_token, ept.user_id,
            uns.send_new_decree, uns.send_new_decree_by_favo
        from expo_push_tokens ept
        inner join user_notification_settings uns on uns.id = ept.notification_settings_id
        where ept.enabled and (uns.send_new_decree or uns.send_new_decree_by_favo)")
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    let mut to_notify_push = HashMap::new();
    for row in rows {
        let user_topics = query!("select topic_name from user_topics inner join unique_eurovoc_topics ut on ut.id_as_hash = topic_id where user_id = $1", row.user_id).fetch_all(pool).await.unwrap_or_default().into_iter().map(|x| x.topic_name).collect::<Vec<_>>();
        let favo_dels = query_scalar!(
            "select delegate_id from favo_dels where user_id = $1",
            row.user_id
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        let relevant_decrees = decrees
            .iter()
            .filter_map(|item| {
                let decree = &item.1;
                if row.send_new_decree {
                    let topics = decree
                        .ai_summary
                        .as_ref()
                        .map(|summary| summary.full_summary.topics.clone())
                        .unwrap_or_default();

                    for topic in &topics {
                        if user_topics.contains(topic) {
                            return Some((item, "send_new_decree"));
                        }
                    }
                }
                if row.send_new_decree_by_favo
                    && favo_dels
                        .iter()
                        .any(|favo_del| decree.gov_official_id == Some(*favo_del))
                {
                    return Some((item, "send_new_decree_by_favo"));
                }
                None
            })
            .collect::<Vec<_>>();

        if !relevant_decrees.is_empty() {
            to_notify_push.insert(row.push_token, relevant_decrees);
        }
    }
    to_notify_push
}

pub(crate) fn send_new_decree_info_mail(
    mail_to: &str,
    decrees: &[&(i32, Decree)],
    gov_officials: &HashMap<i32, SimpleGovOfficial>,
) -> Result<(), Box<dyn std::error::Error>> {
    let decrees = decrees.iter().map(|x| (*x).clone()).collect::<Vec<_>>();

    let mail_to = mail_to.to_string();
    let gov_officials = gov_officials.clone();
    std::thread::spawn(move || {
        let mut decrees_html = String::new();
        for (idx, (_id, decree)) in decrees.iter().enumerate() {
            if idx >= 5 {
                if decrees.len() >= 6 {
                    decrees_html.push_str("Und weitere ...");
                }
                break;
            }

            let Some(gov_official_id) = decree.gov_official_id else {
                continue;
            };
            let Some(gov_official) = gov_officials.get(&gov_official_id) else {
                continue;
            };

            let description = decree
                .ai_summary
                .as_ref()
                .map(|s| s.short_title.clone())
                .unwrap_or(decree.short_title.clone());

            use chrono::TimeZone;
            let publication_date = Vienna.from_utc_datetime(
                &decree
                    .publication_date
                    .and_hms_opt(0, 0, 0)
                    .unwrap_or_default(),
            );

            let decree_html_entry = crate::email::DECREE_ENTRY_TEMPLATE
                .replace("{*DECREE_DESCRIPTION*}", &description)
                .replace("{*DECREE_MINISTRY*}", &decree.ministrial_issuer)
                .replace(
                    "{*DECREE_DATE*}",
                    &publication_date.date_naive().format("%d.%m.%Y").to_string(),
                )
                .replace("{*DECREE_GP*}", decree.gp.as_deref().unwrap_or(""))
                .replace("{*DECREE_GOV_OFFICIAL_NAME*}", &gov_official.name)
                .replace(
                    "{*DECREE_DELEGATE_IMG_URL*}",
                    gov_official
                        .image_url
                        .as_ref()
                        .unwrap_or(&"https://somes.at/images/somes_logo.png".to_string()),
                )
                .replace(
                    "{*DECREE_SOMES_URL*}",
                    &format!("https://somes.at/decree/{}", decree.ris_id),
                );

            decrees_html.push_str(&decree_html_entry);
        }

        log::info!("Sending composed new decree mail to {mail_to}");
        let content = crate::email::DECREE_EMAIL_TEMPLATE.replace("{*DECREES*}", &decrees_html);
        if let Err(e) = send_mail(&MAILER, &mail_to, "Neue Verordnungen auf somes", content) {
            log::error!("could not send decree info mail: {e:?}");
        }
    });

    Ok(())
}

pub(crate) fn notify_users(
    user_decrees: &HashMap<String, Vec<&(i32, Decree)>>,
    gov_officials: &HashMap<i32, SimpleGovOfficial>,
) {
    for (mail_to, decrees) in user_decrees {
        if let Err(e) = send_new_decree_info_mail(mail_to, decrees, gov_officials) {
            log::error!("could not send decree info mail: {e:?}");
        }
    }
}

pub(crate) async fn send_push_for_decrees(
    pool: &PgPool,
    push_notifiers: &HashMap<String, Vec<(&(i32, Decree), &'static str)>>,
) {
    let mut messages = Vec::new();

    for (push_token, items) in push_notifiers {
        if items.len() > MAX_PUSH_PER_TOKEN {
            log::info!(
                "capping decree pushes to {push_token} at {MAX_PUSH_PER_TOKEN} ({} skipped)",
                items.len() - MAX_PUSH_PER_TOKEN
            );
        }

        for entry in items.iter().take(MAX_PUSH_PER_TOKEN) {
            let decree = &entry.0.1;
            let category = entry.1;
            let description = decree
                .ai_summary
                .as_ref()
                .map(|s| s.short_title.as_str())
                .unwrap_or(&decree.short_title);

            messages.push(ExpoPushMessage {
                to: push_token.clone(),
                title: "Neue Verordnung".to_string(),
                body: truncate_for_push(description),
                channel_id: "default",
                sound: None,
                data: PushTarget::Decree {
                    country: "at",
                    ris_id: decree.ris_id.clone(),
                    category,
                },
            });
        }
    }

    send_expo_push_notifications(&PUSH_CLIENT, pool, messages).await;
}
