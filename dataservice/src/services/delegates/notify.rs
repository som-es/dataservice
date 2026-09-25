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
    let res = sqlx::query_as!(
        DelegateNotification,
        r#"
        select
            fd.user_id,
            fd.delegate_id,
            fd.user_info_days,
            delegate_activity_email_info.last_sent_at as "last_sent_at?"
        from favo_dels fd
        left join delegate_activity_email_info
            on delegate_activity_email_info.user_id = fd.user_id
            and delegate_activity_email_info.delegate_id = fd.delegate_id
        where fd.user_id = $1
        "#,
        user_id
    )
    .fetch_all(pg)
    .await?;
    Ok(res)
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

    use chrono::{DateTime, Utc};
    use sqlx::PgPool;

    use crate::{
        AT_DATABASE_URL, connect_pg2,
        delegates::notify::{populate_delegate_profile_html, send_delegate_activity_mail},
    };

    use super::{
        collect_recent_activity_by_delegate, delegate_notifications_for_user, find_notifiers,
        mark_delegate_activity_mail_sent, should_send_delegate_activity,
    };

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

    fn at(rfc3339: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(rfc3339).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn test_should_send_delegate_activity() {
        let now = at("2026-03-01T00:00:00Z");
        let long_ago = at("2026-01-01T00:00:00Z");
        let recently = at("2026-02-28T00:00:00Z");
        assert!(should_send_delegate_activity(None, 30, now));
        assert!(should_send_delegate_activity(Some(long_ago), 30, now));
        assert!(!should_send_delegate_activity(Some(recently), 30, now));
        assert!(!should_send_delegate_activity(None, 0, now));
        assert!(!should_send_delegate_activity(None, -5, now));
    }

    #[sqlx::test(fixtures("fixtures/delegate_notify.sql"))]
    async fn test_delegate_notifications_for_user(pg: PgPool) {
        let user_id: i32 =
            sqlx::query_scalar("insert into somes_user (email) values ('mail@example.com') returning id")
                .fetch_one(&pg)
                .await
                .unwrap();
        let delegate_1: i32 =
            sqlx::query_scalar("insert into delegates (name) values ('Del One') returning id")
                .fetch_one(&pg)
                .await
                .unwrap();
        let delegate_2: i32 =
            sqlx::query_scalar("insert into delegates (name) values ('Del Two') returning id")
                .fetch_one(&pg)
                .await
                .unwrap();
        sqlx::query(
            "insert into favo_dels (user_id, delegate_id, user_info_days) values ($1, $2, $3)",
        )
        .bind(user_id)
        .bind(delegate_1)
        .bind(30)
        .execute(&pg)
        .await
        .unwrap();
        sqlx::query(
            "insert into favo_dels (user_id, delegate_id, user_info_days) values ($1, $2, $3)",
        )
        .bind(user_id)
        .bind(delegate_2)
        .bind(7)
        .execute(&pg)
        .await
        .unwrap();

        let last_sent = at("2026-01-15T12:00:00Z");
        mark_delegate_activity_mail_sent(&pg, user_id, &[delegate_1], last_sent)
            .await
            .unwrap();

        let notifications = delegate_notifications_for_user(&pg, user_id)
            .await
            .unwrap();
        assert_eq!(notifications.len(), 2);
        let with_info = notifications
            .iter()
            .find(|n| n.delegate_id == delegate_1)
            .unwrap();
        assert_eq!(with_info.user_id, user_id);
        assert_eq!(with_info.user_info_days, 30);
        assert_eq!(with_info.last_sent_at, Some(last_sent));
        let without_info = notifications
            .iter()
            .find(|n| n.delegate_id == delegate_2)
            .unwrap();
        assert_eq!(without_info.user_info_days, 7);
        assert_eq!(without_info.last_sent_at, None);
    }

    #[sqlx::test(fixtures("fixtures/delegate_notify.sql"))]
    async fn test_find_notifiers(pg: PgPool) {
        let mail_user: i32 =
            sqlx::query_scalar("insert into somes_user (email) values ('mail@example.com') returning id")
                .fetch_one(&pg)
                .await
                .unwrap();
        sqlx::query("insert into user_notification_settings (user_id, platform) values ($1, 'web')")
            .bind(mail_user)
            .execute(&pg)
            .await
            .unwrap();

        let mobile_user: i32 = sqlx::query_scalar(
            "insert into somes_user (email, is_email_hashed) values ('mobile@example.com', true) returning id",
        )
        .fetch_one(&pg)
        .await
        .unwrap();
        let mobile_settings: i64 = sqlx::query_scalar(
            "insert into user_notification_settings (user_id, platform) values ($1, 'ios') returning id",
        )
        .bind(mobile_user)
        .fetch_one(&pg)
        .await
        .unwrap();
        sqlx::query(
            "insert into expo_push_tokens (user_id, notification_settings_id, push_token, platform) values ($1, $2, 'ExponentPushToken(test)', 'ios')",
        )
        .bind(mobile_user)
        .bind(mobile_settings)
        .execute(&pg)
        .await
        .unwrap();

        let muted_user: i32 =
            sqlx::query_scalar("insert into somes_user (email) values ('muted@example.com') returning id")
                .fetch_one(&pg)
                .await
                .unwrap();
        sqlx::query(
            "insert into user_notification_settings (user_id, platform, send_new_delegate_activity) values ($1, 'web', false)",
        )
        .bind(muted_user)
        .execute(&pg)
        .await
        .unwrap();

        let no_settings_user: i32 =
            sqlx::query_scalar("insert into somes_user (email) values ('plain@example.com') returning id")
                .fetch_one(&pg)
                .await
                .unwrap();

        let recipients = find_notifiers(&pg).await.unwrap();
        let mail = recipients.iter().find(|r| r.user_id == mail_user).unwrap();
        assert_eq!(mail.mail_to.as_deref(), Some("mail@example.com"));
        assert!(mail.push_tokens.is_empty());
        let mobile = recipients.iter().find(|r| r.user_id == mobile_user).unwrap();
        assert_eq!(mobile.mail_to, None);
        assert_eq!(mobile.push_tokens, vec!["ExponentPushToken(test)".to_string()]);
        let plain = recipients.iter().find(|r| r.user_id == no_settings_user).unwrap();
        assert_eq!(plain.mail_to.as_deref(), Some("plain@example.com"));
        assert!(!recipients.iter().any(|r| r.user_id == muted_user));
        assert_eq!(recipients.len(), 3);
    }

    #[sqlx::test(fixtures("fixtures/delegate_notify.sql"))]
    async fn test_mark_delegate_activity_mail_sent(pg: PgPool) {
        let user_id: i32 =
            sqlx::query_scalar("insert into somes_user (email) values ('mail@example.com') returning id")
                .fetch_one(&pg)
                .await
                .unwrap();
        let delegate_id: i32 =
            sqlx::query_scalar("insert into delegates (name) values ('Del One') returning id")
                .fetch_one(&pg)
                .await
                .unwrap();
        let early = at("2026-01-01T00:00:00Z");
        let late = at("2026-02-01T00:00:00Z");
        mark_delegate_activity_mail_sent(&pg, user_id, &[delegate_id], early)
            .await
            .unwrap();
        mark_delegate_activity_mail_sent(&pg, user_id, &[delegate_id], late)
            .await
            .unwrap();
        let last_sent: Option<DateTime<Utc>> = sqlx::query_scalar(
            "select last_sent_at from delegate_activity_email_info where user_id = $1 and delegate_id = $2",
        )
        .bind(user_id)
        .bind(delegate_id)
        .fetch_one(&pg)
        .await
        .unwrap();
        assert_eq!(last_sent, Some(late));
        mark_delegate_activity_mail_sent(&pg, user_id, &[delegate_id], early)
            .await
            .unwrap();
        let last_sent: Option<DateTime<Utc>> = sqlx::query_scalar(
            "select last_sent_at from delegate_activity_email_info where user_id = $1 and delegate_id = $2",
        )
        .bind(user_id)
        .bind(delegate_id)
        .fetch_one(&pg)
        .await
        .unwrap();
        assert_eq!(last_sent, Some(late));
    }
}
