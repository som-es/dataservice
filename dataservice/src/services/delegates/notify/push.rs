use std::collections::HashMap;

use ::combx::Parliament;
use sqlx::PgPool;

use super::activity::RecentActivity;
use crate::push::{
    ExpoPushMessage, PUSH_CLIENT, PushTarget, parliament_country, send_expo_push_notifications,
    truncate_for_push,
};

pub(super) async fn send_delegate_activity_push(
    pool: &PgPool,
    parliament: Parliament,
    push_tokens: &[String],
    delegate_activities: &HashMap<i32, RecentActivity>,
) {
    let country = parliament_country(parliament);
    let mut messages = Vec::new();

    for (delegate_id, activity) in delegate_activities {
        let body = truncate_for_push(&format!("Neue Aktivität von {}", activity.delegate_name));

        for push_token in push_tokens {
            messages.push(ExpoPushMessage {
                to: push_token.clone(),
                title: "Neue Abgeordnetenaktivität".to_string(),
                body: body.clone(),
                channel_id: "default",
                sound: None,
                data: PushTarget::Delegate {
                    country,
                    id: *delegate_id,
                    category: "send_new_delegate_activity",
                },
            });
        }
    }

    send_expo_push_notifications(&PUSH_CLIENT, pool, messages).await;
}
