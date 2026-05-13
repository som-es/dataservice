use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;

mod activity;
mod mail;
mod render;

pub use activity::collect_recent_activity_by_delegate;
use mail::send_delegate_activity_mail;
use render::populate_delegate_profile_html;

#[derive(Debug, sqlx::FromRow)]
pub struct DelegateNotification {
    user_id: i32,
    delegate_id: i32,
    user_info_days: i32,
    last_sent_at: Option<DateTime<Utc>>,
}

async fn find_notifiers(pg: &PgPool) -> sqlx::Result<HashMap<String, Vec<DelegateNotification>>> {
    let mut notifiers = HashMap::new();
    let users = sqlx::query!("select email, id from somes_user where not is_email_hashed and send_new_delegate_activity_mails").fetch_all(pg).await?;

    for user in users {
        let delegate_ids = sqlx::query_as::<_, DelegateNotification>(
            r#"
            select
                fd.user_id,
                fd.delegate_id,
                fd.user_info_days,
                delegate_activity_email_info.last_sent_at
            from favo_dels fd
            left join delegate_activity_email_info
                on delegate_activity_email_info.user_id = fd.user_id
                and delegate_activity_email_info.delegate_id = fd.delegate_id
            where fd.user_id = $1
            "#,
        )
        .bind(user.id)
        .fetch_all(pg)
        .await?;

        notifiers.insert(user.email, delegate_ids);
    }

    Ok(notifiers)
}

fn should_send_delegate_activity(
    last_sent_at: Option<DateTime<Utc>>,
    user_info_days: i32,
    now: DateTime<Utc>,
) -> bool {
    if user_info_days <= 0 {
        return false;
    }

    last_sent_at
        .map(|last_sent_at| now - last_sent_at >= Duration::days(user_info_days as i64))
        .unwrap_or(true)
}

async fn mark_delegate_activity_mail_sent(
    pg: &PgPool,
    user_id: i32,
    delegate_ids: &[i32],
    sent_at: DateTime<Utc>,
) -> sqlx::Result<()> {
    for delegate_id in delegate_ids {
        sqlx::query(
            r#"
            insert into delegate_activity_email_info (user_id, delegate_id, last_sent_at)
            values ($1, $2, $3)
            on conflict (user_id, delegate_id)
            do update set last_sent_at = greatest(
                delegate_activity_email_info.last_sent_at,
                excluded.last_sent_at
            )
            "#,
        )
        .bind(user_id)
        .bind(delegate_id)
        .bind(sent_at)
        .execute(pg)
        .await?;
    }

    Ok(())
}

pub async fn notify_delegate_activity_users(pg: &PgPool) -> sqlx::Result<()> {
    let notifiers = find_notifiers(pg).await?;

    for (mail_to, delegate_notifications) in notifiers {
        let now = Utc::now();
        let mut delegate_activities = HashMap::new();
        let mut sent_delegate_ids = Vec::new();
        let mut user_id = None;

        for delegate_notification in delegate_notifications {
            user_id = Some(delegate_notification.user_id);

            if !should_send_delegate_activity(
                delegate_notification.last_sent_at,
                delegate_notification.user_info_days,
                now,
            ) {
                continue;
            }

            match collect_recent_activity_by_delegate(
                delegate_notification.delegate_id,
                pg,
                delegate_notification.user_info_days,
            )
            .await
            {
                Ok(activity) if activity.has_activity() => {
                    let html = populate_delegate_profile_html(&activity).await;
                    delegate_activities.insert(delegate_notification.delegate_id, html);
                    sent_delegate_ids.push(delegate_notification.delegate_id);
                }
                Ok(_) => {}
                Err(e) => {
                    log::error!(
                        "Could not collect delegate activity for delegate {}: {e:?}",
                        delegate_notification.delegate_id
                    );
                }
            }
        }

        if delegate_activities.is_empty() {
            continue;
        }

        if let Ok(_x) = tokio::task::spawn_blocking(move || {
            match send_delegate_activity_mail(&mail_to, &delegate_activities) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    log::error!("could not send delegate activity mail: {e:?}");
                    return Err("Could not send delegate activity mail");
                }
            }
        })
        .await
        {
            if let Some(user_id) = user_id {
                mark_delegate_activity_mail_sent(pg, user_id, &sent_delegate_ids, now).await?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

use crate::{connect_pg, delegates::notify::{populate_delegate_profile_html, send_delegate_activity_mail}};

    use super::collect_recent_activity_by_delegate;

    #[tokio::test]
    async fn test_collect_recent_activity_by_delegate() {
        let pg_pool = connect_pg().await;
        let delegate_id = 32506;
        let last_n_days = 30;
        let activity = collect_recent_activity_by_delegate(delegate_id, &pg_pool, last_n_days)
            .await
            .unwrap();
        dbg!(activity);
    }

    #[tokio::test]
    async fn test_send_delegate_activity_mail() {
        let mail_to = "";
        let pg_pool = connect_pg().await;
        let last_n_days = 30;
        let delegate_id = 32506;
        let activity = collect_recent_activity_by_delegate(delegate_id, &pg_pool, last_n_days).await.unwrap();
        let html = populate_delegate_profile_html(&activity).await;
        let mut delegate_activities = HashMap::new();
        delegate_activities.insert(delegate_id, html);
        send_delegate_activity_mail(mail_to, &delegate_activities).unwrap();
    }
}
