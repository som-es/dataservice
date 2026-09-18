use std::collections::HashMap;

use ::combx::Parliament;
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;

mod activity;
mod mail;
mod push;
mod render;

pub use activity::collect_recent_activity_by_delegate;
use mail::send_delegate_activity_mail;
use push::send_delegate_activity_push;
use render::populate_delegate_profile_html;

use crate::{AppState, Shutdown};

#[derive(Debug, sqlx::FromRow)]
pub struct DelegateNotification {
    user_id: i32,
    delegate_id: i32,
    user_info_days: i32,
    last_sent_at: Option<DateTime<Utc>>,
}

/// A user can be reachable by mail, push, both, or (for those with neither
/// enabled for delegate activity) not at all, in which case they never show
/// up here.
struct NotifyRecipient {
    user_id: i32,
    mail_to: Option<String>,
    push_tokens: Vec<String>,
}
async fn find_notifiers(pg: &PgPool) -> sqlx::Result<Vec<NotifyRecipient>> {
    let mail_users = sqlx::query!(
        r#"
            select u.email as "email!", u.id as "id!"
            from somes_user u
            left join user_notification_settings s on s.user_id = u.id and s.platform = 'web'
            where not u.is_email_hashed and coalesce(s.send_new_delegate_activity, true)
            "#
    )
    .fetch_all(pg)
    .await?;
    let push_rows = sqlx::query!(
        "select distinct ept.user_id, ept.push_token
        from expo_push_tokens ept
        inner join user_notification_settings uns on uns.id = ept.notification_settings_id
        where ept.enabled and uns.send_new_delegate_activity"
    )
    .fetch_all(pg)
    .await?;

    let mut recipients: HashMap<i32, NotifyRecipient> = HashMap::new();

    for user in mail_users {
        recipients
            .entry(user.id)
            .or_insert_with(|| NotifyRecipient {
                user_id: user.id,
                mail_to: None,
                push_tokens: Vec::new(),
            })
            .mail_to = Some(user.email);
    }

    for row in push_rows {
        recipients
            .entry(row.user_id)
            .or_insert_with(|| NotifyRecipient {
                user_id: row.user_id,
                mail_to: None,
                push_tokens: Vec::new(),
            })
            .push_tokens
            .push(row.push_token);
    }

    Ok(recipients.into_values().collect())
}

async fn delegate_notifications_for_user(
    pg: &PgPool,
    user_id: i32,
) -> sqlx::Result<Vec<DelegateNotification>> {
    sqlx::query_as!(
        DelegateNotification,
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
        user_id
    )
    .fetch_all(pg)
    .await
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
        sqlx::query!(
            r#"
            insert into delegate_activity_email_info (user_id, delegate_id, last_sent_at)
            values ($1, $2, $3)
            on conflict (user_id, delegate_id)
            do update set last_sent_at = greatest(
                delegate_activity_email_info.last_sent_at,
                excluded.last_sent_at
            )
            "#,
            user_id,
            delegate_id,
            sent_at
        )
        .execute(pg)
        .await?;
    }

    Ok(())
}

pub async fn notify_delegate_activity_users(
    pg: &PgPool,
    parliament: Parliament,
) -> sqlx::Result<()> {
    let recipients = find_notifiers(pg).await?;

    for recipient in recipients {
        let now = Utc::now();
        let delegate_notifications = delegate_notifications_for_user(pg, recipient.user_id).await?;

        let mut delegate_activities = HashMap::new();
        let mut sent_delegate_ids = Vec::new();

        for delegate_notification in delegate_notifications {
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
                    delegate_activities.insert(delegate_notification.delegate_id, activity);
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

        let mut sent_any = false;

        if let Some(mail_to) = recipient.mail_to.clone() {
            let mut htmls = HashMap::new();
            for (delegate_id, activity) in &delegate_activities {
                htmls.insert(*delegate_id, populate_delegate_profile_html(activity).await);
            }

            let send_result =
                tokio::task::spawn_blocking(move || {
                    match send_delegate_activity_mail(&mail_to, &htmls) {
                        Ok(()) => true,
                        Err(e) => {
                            log::error!("could not send delegate activity mail: {e:?}");
                            false
                        }
                    }
                })
                .await;

            match send_result {
                Ok(true) => sent_any = true,
                Ok(false) => {}
                Err(e) => log::error!("delegate activity mail task panicked: {e:?}"),
            }
        }

        if !recipient.push_tokens.is_empty() {
            send_delegate_activity_push(
                pg,
                parliament,
                &recipient.push_tokens,
                &delegate_activities,
            )
            .await;
            sent_any = true;
        }

        if sent_any {
            mark_delegate_activity_mail_sent(pg, recipient.user_id, &sent_delegate_ids, now)
                .await?;
        }
    }

    Ok(())
}

pub async fn run_shut_delegate_activity_notify(mut shutdown: Shutdown, app_state: AppState) {
    let sleep_secs: u64 = std::env::var("DELEGATE_ACTIVITY_NOTIFY_SLEEP")
        .unwrap_or_else(|_| "3600".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse DELEGATE_ACTIVITY_NOTIFY_SLEEP env var: {e}. Using default value of 3600 seconds"
            );
            3600
        });

    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                break;
            }
            _ = async {
                if let Err(e) = notify_delegate_activity_users(&app_state.pg_pool, app_state.parliament).await {
                    log::error!("Error notifying delegate activity users: {e:?}");
                }
                log::info!("delegate activity notify service sleeps for {sleep_secs} seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(sleep_secs)).await;
            } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        AT_DATABASE_URL, connect_pg2,
        delegates::notify::{populate_delegate_profile_html, send_delegate_activity_mail},
    };

    use super::collect_recent_activity_by_delegate;

    #[tokio::test]
    async fn test_collect_recent_activity_by_delegate() {
        let pg_pool = connect_pg2(AT_DATABASE_URL).await;
        let delegate_id = 32506;
        let last_n_days = 30;
        let activity = collect_recent_activity_by_delegate(delegate_id, &pg_pool, last_n_days)
            .await
            .unwrap();
        dbg!(activity);
    }

    #[tokio::test]
    async fn test_send_delegate_activity_mail() {
        let mail_to = crate::email::TEST_MAIL_RECEIVER;
        let pg_pool = connect_pg2(AT_DATABASE_URL).await;
        let last_n_days = 90;
        let delegate_id = 32506;
        let activity = collect_recent_activity_by_delegate(delegate_id, &pg_pool, last_n_days)
            .await
            .unwrap();
        let html = populate_delegate_profile_html(&activity).await;
        let mut delegate_activities = HashMap::new();
        delegate_activities.insert(delegate_id, html);
        send_delegate_activity_mail(mail_to, &delegate_activities).unwrap();
    }
}
