use sqlx::{query, PgPool};

pub async fn find_notifiers(pool: &PgPool, legis_init_topics: &[String]) -> Vec<String> {
    let ids = query!("select id, email from somes_user where is_email_hashed is false").fetch_all(pool).await.unwrap_or_default();
    let mut to_notify_emails = vec![];
    for id in ids {
        let user_topics = query!("select topic_name from user_topics inner join unique_topics ut on ut.id = topic_id where user_id = $1", id.id).fetch_all(pool).await.unwrap_or_default().into_iter().map(|x| x.topic_name).collect::<Vec<_>>();
        for legis_init_topic in legis_init_topics {
            if user_topics.contains(legis_init_topic) {
                to_notify_emails.push(id.email);
                break;
            }
        }
    }
    to_notify_emails
}

pub async fn notify_delegates(pool: &PgPool, legis_init_topics: &[String]) {
    for notifier_mail in find_notifiers(pool, legis_init_topics).await {

    }
}
